use crate::models::{RawContent, TextContent};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use symphonia::core::audio::*;
use symphonia::core::codecs::*;
use symphonia::core::formats::*;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Convert RawContent (with optional video/images) to TextContent.
///
/// When `on_progress` is `Some`, progress events are sent via the callback
/// instead of printed to stdout. The callback must be `Send + Sync` so it
/// can be shared across `spawn_blocking` boundaries during ASR/OCR.
pub async fn process(
    raw: &RawContent,
    asr_model: &str,
    ocr_images: bool,
    on_progress: Option<Arc<dyn Fn(&str) + Send + Sync>>,
) -> Result<TextContent, TextifierError> {
    let progress_cb = on_progress.clone();
    let progress = |stage: &str| {
        if let Some(ref cb) = progress_cb {
            cb(stage);
        } else {
            let icon = match stage {
                "downloading" | "ocr" | "asr" => "  ↓",
                _ => "  ↓",
            };
            println!("{} {}", icon, match stage {
                "downloading" => "下载并分析视频...",
                "ocr" => "分析画面文字...",
                "asr" => "转写音频中...",
                _ => stage,
            });
        }
    };

    let mut text_parts = vec![format!("标题：{}", raw.title)];
    if !raw.text_content.is_empty() {
        text_parts.push(format!("描述：{}", raw.text_content));
    }

    let mut image_texts = Vec::new();

    if raw.has_video {
        match &raw.video_url {
            Some(url) => {
                progress("downloading");
                let video_text = transcribe_video(url, asr_model, on_progress).await?;
                if !video_text.is_empty() {
                    text_parts.push(video_text);
                }
            }
            None => {
                if on_progress.is_none() {
                    println!("  ⚠ 未提取到视频直链，跳过视频处理");
                }
            }
        }
    } else if ocr_images && !raw.image_urls.is_empty() {
        progress("downloading");
        match ocr_post_images_tesseract(&raw.image_urls).await {
            Ok(texts) => {
                progress("ocr");
                let non_empty: Vec<&str> = texts.iter().filter(|t| !t.is_empty()).map(|s| s.as_str()).collect();
                if non_empty.is_empty() {
                    if on_progress.is_none() {
                        println!("  ⚠ 图片未识别出文字");
                    }
                } else {
                    text_parts.push(format!("图片文字内容：\n{}", non_empty.join("\n\n")));
                }
                image_texts = texts;
            }
            Err(e) => {
                if on_progress.is_none() {
                    println!("  ⚠ 图片 OCR 失败: {}", e);
                }
            }
        }
    }

    Ok(TextContent {
        full_text: text_parts.join("\n\n"),
        image_texts,
        title: raw.title.clone(),
        source: raw.source.clone(),
        source_url: raw.source_url.clone(),
    })
}

/// Convenience wrapper for CLI usage (no progress callback).
pub async fn process_cli(raw: &RawContent, asr_model: &str, ocr_images: bool) -> Result<TextContent, TextifierError> {
    process(raw, asr_model, ocr_images, None).await
}

#[derive(Debug, thiserror::Error)]
pub enum TextifierError {
    #[error("ffmpeg not found (run: brew install ffmpeg)")]
    FfmpegNotFound,
    #[error("tesseract not found (run: brew install tesseract)")]
    TesseractNotFound,
    #[error("qwen-asr not found (run: cargo install qwen-asr-cli)")]
    QwenAsrNotFound,
    #[error("download failed: {0}")]
    DownloadFailed(String),
    #[error("transcription failed: {0}")]
    TranscriptionFailed(String),
    #[error("OCR failed: {0}")]
    OcrFailed(String),
}

// ── Video download (reqwest) ──────────────────────────────────────────

fn video_download_client() -> reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
            .cookie_store(true)
            .build()
            .expect("reqwest client build")
    }).clone()
}

async fn download_video(url: &str, output_dir: &Path) -> Result<Option<PathBuf>, TextifierError> {
    let client = video_download_client();
    let output_path = output_dir.join("video.mp4");

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| TextifierError::DownloadFailed(format!("视频下载请求失败: {}", e)))?;

    if !response.status().is_success() {
        return Err(TextifierError::DownloadFailed(
            format!("视频下载 HTTP {}", response.status()),
        ));
    }

    let total = response.content_length().unwrap_or(0);
    let bytes = response
        .bytes()
        .await
        .map_err(|e| TextifierError::DownloadFailed(format!("读取视频响应: {}", e)))?;

    tokio::fs::write(&output_path, &bytes)
        .await
        .map_err(|e| TextifierError::DownloadFailed(format!("保存视频文件: {}", e)))?;

    let size = bytes.len() as f64 / 1_048_576.0;
    let total_mb = total as f64 / 1_048_576.0;
    if total > 0 {
        crate::vprintln!("  ✓ 视频已下载 ({:.1} MB / {:.1} MB)", size, total_mb);
    } else {
        crate::vprintln!("  ✓ 视频已下载 ({:.1} MB)", size);
    }

    Ok(Some(output_path))
}

// ── Audio extraction (symphonia) ─────────────────────────────────

fn extract_audio(video_path: &Path, output_dir: &Path) -> Result<Option<PathBuf>, TextifierError> {
    let stem = video_path.file_stem().unwrap_or_default();
    let audio_path = output_dir.join(format!("{}.wav", stem.to_string_lossy()));

    crate::vprintln!("  ↓ 提取音频...");

    let src = std::fs::File::open(video_path)
        .map_err(|e| TextifierError::DownloadFailed(format!("打开视频文件: {}", e)))?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    hint.with_extension("mp4");

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| TextifierError::DownloadFailed(format!("格式识别失败: {}", e)))?;
    let mut format = probed.format;

    let (track_id, mut decoder) = format.tracks().iter()
        .find_map(|t| {
            if t.codec_params.codec == CODEC_TYPE_NULL {
                return None;
            }
            symphonia::default::get_codecs()
                .make(&t.codec_params, &DecoderOptions::default())
                .ok()
                .map(|dec| (t.id, dec))
        })
        .ok_or_else(|| TextifierError::DownloadFailed("未找到可解码的音频轨道".into()))?;

    let mut samples: Vec<f32> = Vec::new();
    let mut out_spec: Option<SignalSpec> = None;

    loop {
        let packet = match format.next_packet() {
            Ok(pkt) => pkt,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(_) => break,
        };
        if packet.track_id() != track_id {
            continue;
        }
        if let Ok(audio_buf) = decoder.decode(&packet) {
            if out_spec.is_none() {
                out_spec = Some(*audio_buf.spec());
            }
            let n_ch = audio_buf.spec().channels.count();
            append_audio_frames(audio_buf, n_ch, &mut samples);
        }
    }

    let spec = out_spec.ok_or_else(|| TextifierError::DownloadFailed("未解码到音频数据".into()))?;
    let (src_channels, src_rate) = (spec.channels.count(), spec.rate as usize);

    let mono: Vec<f32> = if src_channels == 1 {
        samples
    } else {
        let frames = samples.len() / src_channels;
        (0..frames).map(|f| {
            let sum: f32 = (0..src_channels).map(|c| samples[f * src_channels + c]).sum();
            sum / src_channels as f32
        }).collect()
    };

    let target_rate = 16000usize;
    let wav_spec = hound::WavSpec {
        channels: 1,
        sample_rate: target_rate as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&audio_path, wav_spec)
        .map_err(|e| TextifierError::DownloadFailed(format!("创建 WAV: {}", e)))?;

    if src_rate == target_rate {
        for &s in &mono {
            writer.write_sample((s.clamp(-1.0, 1.0) * 32767.0) as i16)
                .map_err(|e| TextifierError::DownloadFailed(format!("写入 WAV: {}", e)))?;
        }
    } else {
        let ratio = target_rate as f64 / src_rate as f64;
        let out_len = (mono.len() as f64 * ratio).ceil() as usize;
        for i in 0..out_len {
            let src_idx = i as f64 / ratio;
            let lo = src_idx.floor() as usize;
            let hi = (lo + 1).min(mono.len().saturating_sub(1));
            let frac = src_idx - lo as f64;
            let val = if lo < mono.len() {
                mono[lo] as f64 * (1.0 - frac) + mono[hi] as f64 * frac
            } else {
                0.0
            };
            writer.write_sample((val.clamp(-1.0, 1.0) * 32767.0) as i16)
                .map_err(|e| TextifierError::DownloadFailed(format!("写入 WAV: {}", e)))?;
        }
    }

    writer.finalize()
        .map_err(|e| TextifierError::DownloadFailed(format!("关闭 WAV: {}", e)))?;

    if audio_path.exists() {
        let size = audio_path.metadata().map(|m| m.len() as f64 / 1024.0).unwrap_or(0.0);
        crate::vprintln!("  ✓ 音频提取完成 ({:.0} KB)", size);
        Ok(Some(audio_path))
    } else {
        Ok(None)
    }
}

fn append_audio_frames(buf: AudioBufferRef<'_>, n_ch: usize, out: &mut Vec<f32>) {
    macro_rules! planar_copy {
        ($planes:expr, $to_f32:expr) => {{
            let ap = $planes.planes();
            let all = ap.planes();
            let frames = all.first().map(|p| p.len()).unwrap_or(0);
            for f in 0..frames {
                for c in 0..n_ch {
                    if let Some(plane) = all.get(c) {
                        out.push($to_f32(plane[f]));
                    }
                }
            }
        }};
    }

    match buf {
        AudioBufferRef::F32(b) => planar_copy!(b, |v: f32| v),
        AudioBufferRef::S16(b) => planar_copy!(b, |v: i16| v as f32 * (1.0 / 32768.0)),
        AudioBufferRef::S32(b) => planar_copy!(b, |v: i32| v as f32 * (1.0 / 2147483648.0)),
        AudioBufferRef::U8(b)  => planar_copy!(b, |v: u8|  (v as f32 - 128.0) / 128.0),
        AudioBufferRef::F64(b) => planar_copy!(b, |v: f64| v as f32),
        _ => {}
    }
}

// ── Transcription (Qwen3-ASR subprocess) ──────────────────────────

fn transcribe_audio(audio_path: &Path, model_name: &str) -> Result<Option<String>, TextifierError> {
    let qwen_asr = find_qwen_asr()?;
    let model_dir = resolve_model_dir(model_name)?;

    crate::vprintln!("  ↓ 转写音频中 (Qwen3-ASR, {})...", model_name);

    let output = Command::new(&qwen_asr)
        .args([
            "-d", &model_dir,
            "-i", &audio_path.to_string_lossy(),
            "--language", "Chinese",
            "--silent",
        ])
        .output()
        .map_err(|e| TextifierError::TranscriptionFailed(format!("qwen-asr exec: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TextifierError::TranscriptionFailed(stderr.trim().to_string()));
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        crate::vprintln!("  ⚠ 转写结果为空");
        Ok(None)
    } else {
        crate::vprintln!("  ✓ 转写完成 (约{}字)", text.chars().count());
        Ok(Some(text))
    }
}

fn find_qwen_asr() -> Result<String, TextifierError> {
    if let Some(path) = crate::which("qwen-asr") {
        return Ok(path);
    }
    Err(TextifierError::QwenAsrNotFound)
}

fn resolve_model_dir(model_name: &str) -> Result<String, TextifierError> {
    if model_name.contains('/') || model_name.contains('\\') {
        if std::path::Path::new(model_name).exists() {
            return Ok(model_name.to_string());
        }
        return Err(TextifierError::TranscriptionFailed(
            format!("模型目录不存在: {}", model_name),
        ));
    }

    let cache_dir = crate::home_dir()
        .join(".cache")
        .join("qwen-asr")
        .join(model_name);

    if cache_dir.exists() {
        Ok(cache_dir.to_string_lossy().to_string())
    } else {
        Err(TextifierError::TranscriptionFailed(
            format!("Qwen3-ASR 模型 '{}' 未下载，请先运行: qwen-asr download {}", model_name, model_name),
        ))
    }
}

// ── Video Frame OCR (ffmpeg + tesseract) ─────────────────────────

fn ocr_video_frames(video_path: &Path) -> Result<Option<String>, TextifierError> {
    let output_dir = tempfile::tempdir()
        .map_err(|e| TextifierError::OcrFailed(format!("tempdir: {}", e)))?;
    let dir = output_dir.path().to_path_buf();

    crate::vprintln!("  ↓ ffmpeg 抽取视频帧...");
    let frame_pattern = dir.join("frame_%04d.png");
    let ffmpeg_output = Command::new("ffmpeg")
        .args([
            "-i", &video_path.to_string_lossy(),
            "-vf", "fps=1/3",
            "-q:v", "2",
            "-y",
            &frame_pattern.to_string_lossy(),
        ])
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                TextifierError::FfmpegNotFound
            } else {
                TextifierError::OcrFailed(format!("ffmpeg exec: {}", e))
            }
        })?;

    if !ffmpeg_output.status.success() {
        let stderr = String::from_utf8_lossy(&ffmpeg_output.stderr);
        return Err(TextifierError::OcrFailed(format!("ffmpeg error: {}", stderr.trim())));
    }

    let mut frame_paths: Vec<PathBuf> = Vec::new();
    let mut i = 1;
    loop {
        let path = dir.join(format!("frame_{i:04}.png"));
        if path.exists() {
            frame_paths.push(path);
            i += 1;
        } else {
            break;
        }
    }

    if frame_paths.is_empty() {
        crate::vprintln!("  ⚠ ffmpeg 未生成任何帧");
        return Ok(None);
    }

    crate::vprintln!("  ↓ 对 {} 帧进行 OCR (tesseract)...", frame_paths.len());

    let mut frame_texts: Vec<String> = Vec::new();
    for path in &frame_paths {
        match ocr_image_tesseract(path) {
            Ok(Some(text)) => frame_texts.push(text),
            Ok(None) => {}
            Err(e) => crate::vprintln!("  ⚠ 帧 OCR 失败: {}", e),
        }
    }

    let mut results: Vec<String> = Vec::new();
    let mut last_text = String::new();

    for text in &frame_texts {
        if text.len() < 5 {
            continue;
        }
        if !last_text.is_empty() && text_similar(text, &last_text) {
            continue;
        }
        results.push(text.clone());
        last_text = text.clone();
    }

    if results.is_empty() {
        return Ok(None);
    }

    let combined = results.join("\n");
    crate::vprintln!("  ✓ 帧 OCR 完成 (约{}字)", combined.chars().count());
    Ok(Some(combined))
}

/// Run tesseract OCR on an image file.
fn ocr_image_tesseract(path: &Path) -> Result<Option<String>, TextifierError> {
    let output = Command::new("tesseract")
        .args([
            &path.to_string_lossy(),
            "stdout",
            "-l", "chi_sim+eng",
            "--psm", "6",
        ])
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                TextifierError::TesseractNotFound
            } else {
                TextifierError::OcrFailed(format!("tesseract exec: {}", e))
            }
        })?;

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() && !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TextifierError::OcrFailed(format!("tesseract error: {}", stderr.trim())));
    }

    if text.is_empty() { Ok(None) } else { Ok(Some(text)) }
}

// ── Post image OCR (tesseract) ────────────────────────────────────

fn image_download_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(30))
            .cookie_store(true)
            .build()
            .expect("reqwest client build")
    })
}

/// Download post images and OCR each one with tesseract.
async fn ocr_post_images_tesseract(urls: &[String]) -> Result<Vec<String>, TextifierError> {
    let output_dir = tempfile::tempdir()
        .map_err(|e| TextifierError::OcrFailed(format!("tempdir: {}", e)))?;
    let dir = output_dir.path().to_path_buf();

    let image_paths = download_images(urls, &dir).await;

    if image_paths.is_empty() {
        return Ok(vec![]);
    }

    ocr_images_tesseract(&image_paths)
}

async fn download_images(urls: &[String], output_dir: &Path) -> Vec<PathBuf> {
    let client = image_download_client();

    let mut paths = Vec::new();
    for (i, url) in urls.iter().enumerate() {
        let resp = match client
            .get(url)
            .header("Referer", "https://www.xiaohongshu.com/")
            .header("Accept", "image/webp,image/apng,image/*,*/*")
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => r,
            _ => {
                println!("  ⚠ 图片下载失败: {}", &url[..url.len().min(80)]);
                continue;
            }
        };
        let bytes = match resp.bytes().await {
            Ok(b) => b,
            Err(_) => {
                println!("  ⚠ 图片读取失败: {}", &url[..url.len().min(80)]);
                continue;
            }
        };
        let ext = if bytes.len() > 4 && &bytes[..4] == b"\x89PNG" {
            "png"
        } else if bytes.len() > 2 && &bytes[..2] == b"\xff\xd8" {
            "jpg"
        } else {
            "png"
        };
        let path = output_dir.join(format!("image_{i:04}.{ext}"));
        if std::fs::write(&path, &bytes).is_ok() {
            let size = bytes.len() as f64 / 1024.0;
            crate::vprintln!("  ✓ 图片[{i}] 已下载 ({size:.0} KB)");
            paths.push(path);
        }
    }
    paths
}

fn ocr_images_tesseract(paths: &[PathBuf]) -> Result<Vec<String>, TextifierError> {
    let mut results = Vec::new();
    for path in paths {
        match ocr_image_tesseract(path) {
            Ok(Some(text)) => results.push(text),
            Ok(None) => results.push(String::new()),
            Err(e) => {
                crate::vprintln!("  ⚠ 图片 OCR 失败: {e}");
                results.push(String::new());
            }
        }
    }
    Ok(results)
}

fn text_similar(a: &str, b: &str) -> bool {
    if a == b { return true; }
    if a.contains(b) || b.contains(a) { return true; }
    let (short, long) = if a.len() < b.len() { (a, b) } else { (b, a) };
    if short.is_empty() { return false; }
    let overlap = short.chars().filter(|c| long.contains(*c)).count();
    overlap as f64 / short.chars().count() as f64 > 0.7
}

// ── Orchestration ─────────────────────────────────────────────────

async fn transcribe_video(
    url: &str,
    asr_model: &str,
    on_progress: Option<Arc<dyn Fn(&str) + Send + Sync>>,
) -> Result<String, TextifierError> {
    let output_dir = tempfile::tempdir()
        .map_err(|e| TextifierError::DownloadFailed(format!("tempdir: {}", e)))?;
    let dir = output_dir.path().to_path_buf();

    let video = match download_video(url, &dir).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            print_video_skip(&on_progress);
            return Ok(String::new());
        }
        Err(e) => {
            println!("  ⚠ 视频下载失败: {e} (跳过视频处理)");
            return Ok(String::new());
        }
    };

    // ASR (inner function handles its own progress via vprintln)
    let video_asr = video.clone();
    let dir_asr = dir.clone();
    let model = asr_model.to_string();
    let asr_handle = tokio::task::spawn_blocking(move || -> Result<Option<String>, TextifierError> {
        match extract_audio(&video_asr, &dir_asr) {
            Ok(Some(audio)) => transcribe_audio(&audio, &model),
            Ok(None) => Ok(None),
            Err(e) => {
                println!("  ✗ 音频提取失败: {e}");
                Ok(None)
            }
        }
    });

    // OCR in parallel, with progress callback
    let video_ocr = video;
    let ocr_progress = on_progress.clone();
    let ocr_handle = tokio::task::spawn_blocking(move || -> Result<Option<String>, TextifierError> {
        if let Some(ref cb) = ocr_progress {
            cb("ocr");
        }
        ocr_video_frames(&video_ocr)
    });

    let transcript = asr_handle.await
        .map_err(|e| TextifierError::TranscriptionFailed(format!("ASR task: {e}")))?
        .unwrap_or(None);
    let ocr_text = ocr_handle.await
        .map_err(|e| TextifierError::OcrFailed(format!("OCR task: {e}")))?
        .unwrap_or(None);

    let mut parts: Vec<String> = Vec::new();
    if let Some(ref t) = transcript {
        if !t.is_empty() {
            parts.push(format!("视频口述内容：\n{t}"));
            if let Some(ref cb) = on_progress {
                cb("asr");
            }
        }
    }
    if let Some(ref o) = ocr_text {
        if !o.is_empty() {
            parts.push(format!("视频画面文字：\n{o}"));
        }
    }

    Ok(parts.join("\n\n"))
}

fn print_video_skip(on_progress: &Option<Arc<dyn Fn(&str) + Send + Sync>>) {
    if on_progress.is_none() {
        println!("  ⚠ 该笔记没有实际视频，使用图文内容继续分析");
    }
}

// ── Dependency checks ─────────────────────────────────────────────

pub fn check_ffmpeg() -> bool {
    crate::which("ffmpeg").is_some()
}

pub fn check_tesseract() -> bool {
    crate::which("tesseract").is_some()
}

pub fn check_tesseract_chi_sim() -> bool {
    Command::new("tesseract")
        .args(["--list-langs"])
        .output()
        .ok()
        .map(|o| {
            let all = format!("{}\n{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr));
            all.contains("chi_sim")
        })
        .unwrap_or(false)
}

pub fn check_qwen_asr() -> bool {
    crate::which("qwen-asr").is_some()
}

pub fn check_qwen_asr_model() -> bool {
    crate::home_dir()
        .join(".cache")
        .join("qwen-asr")
        .join("qwen3-asr-0.6b")
        .exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::RawContent;

    #[test]
    fn test_process_no_video() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let raw = RawContent {
            title: "测试标题".into(), text_content: "测试描述".into(),
            image_urls: vec![], has_video: false, video_url: None,
            source: "test".into(), source_url: "https://example.com".into(),
            content_type: Default::default(),
        };
        let result = rt.block_on(process_cli(&raw, "qwen3-asr-0.6b", false)).unwrap();
        assert!(result.full_text.contains("测试标题"));
        assert!(!result.full_text.contains("视频口述内容"));
    }

    #[test]
    fn test_text_similar_exact() {
        assert!(text_similar("hello", "hello"));
    }

    #[test]
    fn test_text_similar_contains() {
        assert!(text_similar("hello world", "hello"));
    }

    #[test]
    fn test_text_similar_mostly_overlapping() {
        // "123456789" contains "123456" → returns true via contains check
        assert!(text_similar("123456789", "123456"));
    }

    #[test]
    fn test_text_similar_different() {
        assert!(!text_similar("abc", "xyz"));
    }
}
