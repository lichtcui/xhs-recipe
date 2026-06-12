use crate::models::{RawContent, TextContent};
use std::path::{Path, PathBuf};
use std::process::Command;
use symphonia::core::audio::*;
use symphonia::core::codecs::*;
use symphonia::core::formats::*;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Convert RawContent (with optional video) to TextContent.
pub async fn process(raw: &RawContent, asr_model: &str) -> Result<TextContent, TextifierError> {
    let mut text_parts = vec![format!("标题：{}", raw.title)];
    if !raw.text_content.is_empty() {
        text_parts.push(format!("描述：{}", raw.text_content));
    }

    if raw.has_video {
        println!("  ↓ 下载视频...");
        let transcript = transcribe_video(&raw.source_url, asr_model).await?;
        if !transcript.is_empty() {
            text_parts.push(format!("视频口述内容：\n{}", transcript));
        }
    } else {
        println!("  ✓ 无需转写");
    }

    Ok(TextContent {
        full_text: text_parts.join("\n\n"),
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
                out_spec = Some(audio_buf.spec().clone());
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


// ── Orchestration ─────────────────────────────────────────────────

async fn transcribe_video(url: &str, asr_model: &str) -> Result<String, TextifierError> {
    let url = url.to_string();
    let output_dir = tempfile::tempdir()
        .map_err(|e| TextifierError::DownloadFailed(format!("tempdir: {}", e)))?;
    let dir = output_dir.path().to_path_buf();

    let video = {
        let d = dir.clone();
        let u = url;
        tokio::task::spawn_blocking(move || download_video(&u, &d)).await
            .map_err(|e| TextifierError::DownloadFailed(format!("task: {}", e)))?
            .map_err(|e| { println!("  ✗ 视频下载失败: {}", e); e })?
    };
    let video = match video { Some(v) => v, None => return Ok(String::new()) };

    let audio = {
        let d = dir.clone();
        tokio::task::spawn_blocking(move || extract_audio(&video, &d)).await
            .map_err(|e| TextifierError::TranscriptionFailed(format!("task: {}", e)))?
            .map_err(|e| { println!("  ✗ 音频提取失败: {}", e); e })?
    };
    let audio = match audio { Some(a) => a, None => return Ok(String::new()) };

    let model = asr_model.to_string();
    let transcript = tokio::task::spawn_blocking(move || transcribe_audio(&audio, &model)).await
        .map_err(|e| TextifierError::TranscriptionFailed(format!("task: {}", e)))?
        .map_err(|e| { println!("  ✗ 转写失败: {}", e); e })?;

    Ok(transcript.unwrap_or_default())
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
        let result = rt.block_on(process(&raw, "qwen3-asr-0.6b")).unwrap();
        assert!(result.full_text.contains("测试标题"));
        assert!(!result.full_text.contains("视频口述内容"));
    }
}
