use crate::models::{RawContent, TextContent};
use std::path::{Path, PathBuf};
use std::process::Command;
use symphonia::core::audio::*;
use symphonia::core::codecs::*;
use symphonia::core::formats::*;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Convert RawContent (with optional video/images) to TextContent.
pub async fn process(raw: &RawContent, asr_model: &str, ocr_images: bool) -> Result<TextContent, TextifierError> {
    let mut text_parts = vec![format!("标题：{}", raw.title)];
    if !raw.text_content.is_empty() {
        text_parts.push(format!("描述：{}", raw.text_content));
    }

    let mut image_texts = Vec::new();

    if raw.has_video {
        println!("  ↓ 下载并分析视频...");
        let video_text = transcribe_video(&raw.source_url, asr_model).await?;
        if !video_text.is_empty() {
            text_parts.push(video_text);
        }
    } else if ocr_images && !raw.image_urls.is_empty() {
        println!("  ↓ 分析图片文字...");
        match ocr_post_images_individual(&raw.image_urls).await {
            Ok(texts) => {
                let non_empty: Vec<&str> = texts.iter().filter(|t| !t.is_empty()).map(|s| s.as_str()).collect();
                if non_empty.is_empty() {
                    println!("  ⚠ 图片未识别出文字");
                } else {
                    text_parts.push(format!("图片文字内容：\n{}", non_empty.join("\n\n")));
                }
                image_texts = texts;
            }
            Err(e) => {
                println!("  ⚠ 图片 OCR 失败: {}", e);
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

#[derive(Debug, thiserror::Error)]
pub enum TextifierError {
    #[error("yt-dlp not found")]
    YtDlpNotFound,
    #[error("qwen-asr not found (run: cargo install qwen-asr-cli)")]
    QwenAsrNotFound,
    #[error("download failed: {0}")]
    DownloadFailed(String),
    #[error("transcription failed: {0}")]
    TranscriptionFailed(String),
    #[error("OCR failed: {0}")]
    OcrFailed(String),
}

// ── Video download (yt-dlp) ───────────────────────────────────────

fn find_yt_dlp() -> Result<String, TextifierError> {
    if let Some(path) = crate::which("yt-dlp") {
        return Ok(path);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let sibling = parent.join("yt-dlp");
            if sibling.exists() {
                return Ok(sibling.to_string_lossy().to_string());
            }
        }
    }
    Err(TextifierError::YtDlpNotFound)
}

fn download_video(url: &str, output_dir: &Path) -> Result<Option<PathBuf>, TextifierError> {
    let yt_dlp = find_yt_dlp()?;
    let template = output_dir.join("video.%(ext)s");
    let template_str = template.to_string_lossy().to_string();

    let result = Command::new(&yt_dlp)
        .args([
            "--quiet", "--no-warnings", "--no-playlist",
            "-f", "best[ext=mp4]/best",
            "-o", &template_str, "--", url,
        ])
        .output()
        .map_err(|e| TextifierError::DownloadFailed(format!("yt-dlp exec error: {}", e)))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(TextifierError::DownloadFailed(stderr.trim().to_string()));
    }

    for entry in std::fs::read_dir(output_dir).map_err(|e| {
        TextifierError::DownloadFailed(format!("cannot read output dir: {}", e))
    })? {
        let entry = entry.map_err(|e| TextifierError::DownloadFailed(format!("read entry: {}", e)))?;
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if matches!(ext, "mp4" | "webm" | "mkv") {
            let size = path.metadata().map(|m| m.len() as f64 / 1_048_576.0).unwrap_or(0.0);
            let fname = path.file_name().unwrap_or_default().to_string_lossy();
            crate::vprintln!("  ✓ 视频文件: {} ({:.1} MB)", fname, size);
            return Ok(Some(path));
        }
    }

    crate::vprintln!("  ✗ 未找到下载的视频文件");
    Ok(None)
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

    // Find the first decodable audio track (skip video tracks)
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

    // Decode all audio frames into f32 interleaved samples
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

    // Downmix to mono
    let mono: Vec<f32> = if src_channels == 1 {
        samples
    } else {
        let frames = samples.len() / src_channels;
        (0..frames).map(|f| {
            let sum: f32 = (0..src_channels).map(|c| samples[f * src_channels + c]).sum();
            sum / src_channels as f32
        }).collect()
    };

    // Resample to 16kHz and write WAV
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

/// Convert decoded audio frames (any sample format) to interleaved f32.
fn append_audio_frames(buf: AudioBufferRef<'_>, n_ch: usize, out: &mut Vec<f32>) {
    // AAC decodes to F32 in symphonia, so the F32 arm is the hot path.
    // Other formats are handled for robustness with downloads that may
    // contain PCM audio instead of AAC.
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
        _ => {} // S24/U24/U16 — extremely rare, skip
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

// ── Qwen3-ASR helpers ─────────────────────────────────────────────

fn find_qwen_asr() -> Result<String, TextifierError> {
    if let Some(path) = crate::which("qwen-asr") {
        return Ok(path);
    }
    Err(TextifierError::QwenAsrNotFound)
}

fn resolve_model_dir(model_name: &str) -> Result<String, TextifierError> {
    // If it's a path, use it directly
    if model_name.contains('/') || model_name.contains('\\') {
        if std::path::Path::new(model_name).exists() {
            return Ok(model_name.to_string());
        }
        return Err(TextifierError::TranscriptionFailed(
            format!("模型目录不存在: {}", model_name),
        ));
    }

    // Look in default cache directory (~/.cache/qwen-asr/<model_name>)
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


// ── Video Frame OCR (ffmpeg + Vision framework) ─────────────────

/// Source code for the macOS Vision-based OCR helper binary.
const OCR_HELPER_SOURCE: &str = r#"import Vision
import AppKit
import Foundation

let paths = CommandLine.arguments.dropFirst()

for path in paths {
    let url = URL(fileURLWithPath: path)
    guard let image = NSImage(contentsOf: url) else {
        print("---FRAME_END---")
        continue
    }
    guard let cgImage = image.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
        print("---FRAME_END---")
        continue
    }

    let request = VNRecognizeTextRequest()
    request.recognitionLanguages = ["zh-Hans", "en-US"]
    request.recognitionLevel = .accurate

    let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])
    do {
        try handler.perform([request])
    } catch {
        print("---FRAME_END---")
        continue
    }

    guard let observations = request.results, !observations.isEmpty else {
        print("---FRAME_END---")
        continue
    }

    // Sort top-to-bottom by bounding box top edge
    let sorted = observations.sorted { a, b in
        let aTop = a.boundingBox.origin.y + a.boundingBox.size.height
        let bTop = b.boundingBox.origin.y + b.boundingBox.size.height
        return aTop > bTop
    }

    let texts = sorted.compactMap { $0.topCandidates(1).first?.string }
    print(texts.joined(separator: "\n"))
    print("---FRAME_END---")
}
"#;

/// Ensure the OCR helper binary is compiled and cached.
fn ensure_ocr_helper() -> Result<String, TextifierError> {
    let cache_dir = crate::home_dir().join(".cache").join("xhs-recipe");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| TextifierError::OcrFailed(format!("create cache dir: {}", e)))?;

    let binary_path = cache_dir.join("ocr_helper");
    let source_path = cache_dir.join("ocr_helper.swift");

    // Recompile if source has changed
    let needs_compile = match (std::fs::metadata(&binary_path), std::fs::metadata(&source_path)) {
        (Ok(bin_md), Ok(src_md)) => {
            match (bin_md.modified(), src_md.modified()) {
                (Ok(bin_time), Ok(src_time)) => src_time > bin_time,
                _ => true,
            }
        }
        _ => true,
    };

    if needs_compile {
        std::fs::write(&source_path, OCR_HELPER_SOURCE)
            .map_err(|e| TextifierError::OcrFailed(format!("write Swift source: {}", e)))?;

        crate::vprintln!("  ↓ 编译 OCR 辅助工具...");
        let output = Command::new("swiftc")
            .args(["-O", "-o", &binary_path.to_string_lossy(), &source_path.to_string_lossy()])
            .output()
            .map_err(|e| TextifierError::OcrFailed(format!("swiftc exec: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TextifierError::OcrFailed(format!("swiftc error: {}", stderr.trim())));
        }
        crate::vprintln!("  ✓ OCR 辅助工具编译完成");
    }

    Ok(binary_path.to_string_lossy().to_string())
}

/// Extract frames from video at regular intervals using ffmpeg.
fn extract_frames(video_path: &Path, output_dir: &Path) -> Result<Vec<PathBuf>, TextifierError> {
    let ffmpeg = crate::which("ffmpeg")
        .ok_or_else(|| TextifierError::OcrFailed("ffmpeg not found (brew install ffmpeg)".into()))?;

    let output_pattern = output_dir.join("frame_%04d.png");

    let result = Command::new(&ffmpeg)
        .args([
            "-i",
            &video_path.to_string_lossy(),
            "-vf", "fps=1/3",
            "-q:v", "2",
            "-y",
            &output_pattern.to_string_lossy(),
        ])
        .output()
        .map_err(|e| TextifierError::OcrFailed(format!("ffmpeg exec error: {}", e)))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(TextifierError::OcrFailed(format!("ffmpeg error: {}", stderr.trim())));
    }

    let mut frames: Vec<PathBuf> = std::fs::read_dir(output_dir)
        .map_err(|e| TextifierError::OcrFailed(format!("read dir: {}", e)))?
        .filter_map(|entry| entry.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("png"))
        .collect();

    frames.sort();

    if frames.is_empty() {
        crate::vprintln!("  ⚠ 未提取到视频帧");
        return Err(TextifierError::OcrFailed("no frames extracted".into()));
    }

    Ok(frames)
}

/// Run OCR on all frames using the macOS Vision framework through the compiled helper.
/// Returns deduplicated text from all frames.
fn ocr_all_frames(frame_paths: &[PathBuf]) -> Result<Option<String>, TextifierError> {
    let helper = ensure_ocr_helper()?;

    let output = Command::new(&helper)
        .args(frame_paths.iter().map(|p| p.to_string_lossy().to_string()))
        .output()
        .map_err(|e| TextifierError::OcrFailed(format!("OCR helper exec: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TextifierError::OcrFailed(format!("OCR helper error: {}", stderr.trim())));
    }

    // Parse output: each frame's text is separated by ---FRAME_END---
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut frame_texts: Vec<String> = Vec::new();
    let mut current = String::new();

    for line in stdout.lines() {
        if line == "---FRAME_END---" {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                frame_texts.push(trimmed);
            }
            current = String::new();
        } else if !line.is_empty() {
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
        }
    }

    // Deduplicate consecutive similar frames
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
    crate::vprintln!("  ✓ OCR 完成 (约{}字)", combined.chars().count());
    Ok(Some(combined))
}

// ── Post image OCR ─────────────────────────────────────────────────

/// Download post images to a local directory for OCR.
async fn download_images(urls: &[String], output_dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut paths = Vec::new();
    for (i, url) in urls.iter().enumerate() {
        let resp = match client.get(url).send().await {
            Ok(r) if r.status().is_success() => r,
            _ => {
                crate::vprintln!("  ⚠ 图片下载失败: {}", &url[..url.len().min(80)]);
                continue;
            }
        };
        let bytes = match resp.bytes().await {
            Ok(b) => b,
            Err(_) => continue,
        };
        let ext = if bytes.len() > 4 && &bytes[..4] == b"\x89PNG" {
            "png"
        } else if bytes.len() > 2 && &bytes[..2] == b"\xff\xd8" {
            "jpg"
        } else {
            "png"
        };
        let path = output_dir.join(format!("image_{:04}.{}", i, ext));
        if std::fs::write(&path, &bytes).is_ok() {
            let size = bytes.len() as f64 / 1024.0;
            crate::vprintln!("  ✓ 图片[{}] 已下载 ({:.0} KB)", i, size);
            paths.push(path);
        }
    }
    paths
}

/// Download post images and OCR each one individually.
/// Returns per-image OCR texts (one string per image, empty if no text).
async fn ocr_post_images_individual(urls: &[String]) -> Result<Vec<String>, TextifierError> {
    let output_dir = tempfile::tempdir()
        .map_err(|e| TextifierError::OcrFailed(format!("tempdir: {}", e)))?;
    let dir = output_dir.path().to_path_buf();

    let image_paths = download_images(urls, &dir).await;

    if image_paths.is_empty() {
        return Ok(vec![]);
    }

    ocr_images_individual(&image_paths)
}

/// Run OCR on multiple images and return per-image results (no dedup).
fn ocr_images_individual(paths: &[PathBuf]) -> Result<Vec<String>, TextifierError> {
    let helper = ensure_ocr_helper()?;

    let output = Command::new(&helper)
        .args(paths.iter().map(|p| p.to_string_lossy().to_string()))
        .output()
        .map_err(|e| TextifierError::OcrFailed(format!("OCR helper exec: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TextifierError::OcrFailed(format!("OCR helper error: {}", stderr.trim())));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut results = Vec::new();
    let mut current = String::new();

    for line in stdout.lines() {
        if line == "---FRAME_END---" {
            results.push(current.trim().to_string());
            current = String::new();
        } else if !line.is_empty() {
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
        }
    }

    // Handle case where last frame doesn't have FRAME_END
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        results.push(trimmed);
    }

    // Pad with empty strings to match expected count if some failed
    while results.len() < paths.len() {
        results.push(String::new());
    }

    Ok(results)
}

/// Check if two OCR results are similar enough to be duplicates.
fn text_similar(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    if a.contains(b) || b.contains(a) {
        return true;
    }
    let (short, long) = if a.len() < b.len() { (a, b) } else { (b, a) };
    if short.is_empty() {
        return false;
    }
    let overlap = short.chars().filter(|c| long.contains(*c)).count();
    overlap as f64 / short.chars().count() as f64 > 0.7
}

// ── Orchestration ─────────────────────────────────────────────────

async fn transcribe_video(url: &str, asr_model: &str) -> Result<String, TextifierError> {
    let url = url.to_string();
    let output_dir = tempfile::tempdir()
        .map_err(|e| TextifierError::DownloadFailed(format!("tempdir: {}", e)))?;
    let dir = output_dir.path().to_path_buf();

    // Step 1: Download video once, shared between ASR and OCR
    let video = {
        let d = dir.clone();
        let u = url;
        tokio::task::spawn_blocking(move || download_video(&u, &d)).await
            .map_err(|e| TextifierError::DownloadFailed(format!("task: {}", e)))?
            .map_err(|e| { println!("  ⚠ 无视频内容: {} (跳过视频处理)", e); e })
            .ok()
            .flatten()
    };
    let video = match video { Some(v) => v, None => {
        println!("  ⚠ 该笔记没有实际视频，使用图文内容继续分析");
        return Ok(String::new());
    }};

    // Step 2: Run ASR (audio) and OCR (frames) in parallel
    let video_asr = video.clone();
    let dir_asr = dir.clone();
    let model = asr_model.to_string();
    let asr_handle = tokio::task::spawn_blocking(move || -> Result<Option<String>, TextifierError> {
        match extract_audio(&video_asr, &dir_asr) {
            Ok(Some(audio)) => transcribe_audio(&audio, &model),
            Ok(None) => Ok(None),
            Err(e) => {
                println!("  ✗ 音频提取失败: {}", e);
                Ok(None) // Non-fatal: OCR might still work
            }
        }
    });

    let video_ocr = video;
    let dir_ocr = dir;
    let ocr_handle = tokio::task::spawn_blocking(move || -> Result<Option<String>, TextifierError> {
        let frames = match extract_frames(&video_ocr, &dir_ocr) {
            Ok(f) => f,
            Err(e) => {
                crate::vprintln!("  ⚠ 帧提取失败: {}", e);
                return Ok(None);
            }
        };
        ocr_all_frames(&frames)
    });

    let transcript = asr_handle.await
        .map_err(|e| TextifierError::TranscriptionFailed(format!("ASR task: {}", e)))?
        .unwrap_or(None);
    let ocr_text = ocr_handle.await
        .map_err(|e| TextifierError::OcrFailed(format!("OCR task: {}", e)))?
        .unwrap_or(None);

    // Step 3: Combine results
    let mut parts: Vec<String> = Vec::new();
    if let Some(ref t) = transcript {
        if !t.is_empty() {
            parts.push(format!("视频口述内容：\n{}", t));
        }
    }
    if let Some(ref o) = ocr_text {
        if !o.is_empty() {
            parts.push(format!("视频画面文字：\n{}", o));
        }
    }

    Ok(parts.join("\n\n"))
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
        };
        let result = rt.block_on(process(&raw, "qwen3-asr-0.6b", false)).unwrap();
        assert!(result.full_text.contains("测试标题"));
        assert!(!result.full_text.contains("视频口述内容"));
    }
}
