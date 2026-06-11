use core::{RawContent, TextContent};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Convert RawContent (with optional video) to TextContent.
pub async fn process(raw: &RawContent, whisper_model: &str) -> Result<TextContent, TextifierError> {
    let mut text_parts = vec![format!("标题：{}", raw.title)];
    if !raw.text_content.is_empty() {
        text_parts.push(format!("描述：{}", raw.text_content));
    }

    if raw.has_video {
        println!("  ↓ 下载视频...");
        let transcript = transcribe_video(&raw.source_url, whisper_model).await?;
        if !transcript.is_empty() {
            text_parts.push(format!("视频口述内容：\n{}", transcript));
        }
    } else {
        println!("  ✓ 图文笔记，无需转写");
    }

    Ok(TextContent {
        full_text: text_parts.join("\n\n"),
        title: raw.title.clone(),
        source: raw.source.clone(),
        source_url: raw.source_url.clone(),
    })
}

#[derive(Debug)]
pub enum TextifierError {
    YtDlpNotFound,
    FfmpegNotFound,
    DownloadFailed(String),
    TranscriptionFailed(String),
}

impl std::fmt::Display for TextifierError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::YtDlpNotFound => write!(f, "yt-dlp not found"),
            Self::FfmpegNotFound => write!(f, "ffmpeg not found"),
            Self::DownloadFailed(msg) => write!(f, "download failed: {}", msg),
            Self::TranscriptionFailed(msg) => write!(f, "transcription failed: {}", msg),
        }
    }
}

impl std::error::Error for TextifierError {}

// ── Video download (yt-dlp) ───────────────────────────────────────

fn find_yt_dlp() -> Result<String, TextifierError> {
    if let Ok(path) = which("yt-dlp") {
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

fn which(name: &str) -> Result<String, ()> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.exists() {
            return Ok(candidate.to_string_lossy().to_string());
        }
    }
    Err(())
}

fn download_video(url: &str, output_dir: &Path) -> Result<Option<PathBuf>, TextifierError> {
    let yt_dlp = find_yt_dlp()?;
    let template = output_dir.join("%(id)s.%(ext)s");
    let template_str = template.to_string_lossy().to_string();

    println!("  ↓ 下载视频（yt-dlp）...");
    let result = Command::new(&yt_dlp)
        .args([
            "--quiet", "--no-warnings", "--no-playlist",
            "-f", "best[ext=mp4]/best",
            "-o", &template_str, url,
        ])
        .output()
        .map_err(|e| TextifierError::DownloadFailed(format!("yt-dlp exec error: {}", e)))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(TextifierError::DownloadFailed(stderr.trim().to_string()));
    }

    println!("  ✓ 视频下载完成");

    for entry in std::fs::read_dir(output_dir).map_err(|e| {
        TextifierError::DownloadFailed(format!("cannot read output dir: {}", e))
    })? {
        let entry = entry.map_err(|e| TextifierError::DownloadFailed(format!("read entry: {}", e)))?;
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if matches!(ext, "mp4" | "webm" | "mkv") {
            let size = path.metadata().map(|m| m.len() as f64 / 1_048_576.0).unwrap_or(0.0);
            let fname = path.file_name().unwrap_or_default().to_string_lossy();
            println!("  ✓ 视频文件: {} ({:.1} MB)", fname, size);
            return Ok(Some(path));
        }
    }

    println!("  ✗ 未找到下载的视频文件");
    Ok(None)
}

// ── Audio extraction (ffmpeg) ────────────────────────────────────

fn extract_audio(video_path: &Path, output_dir: &Path) -> Result<Option<PathBuf>, TextifierError> {
    let stem = video_path.file_stem().unwrap_or_default();
    let audio_path = output_dir.join(format!("{}.wav", stem.to_string_lossy()));

    println!("  ↓ 提取音频...");
    let result = Command::new("ffmpeg")
        .args([
            "-y",
            "-i", &video_path.to_string_lossy(),
            "-vn", "-acodec", "pcm_s16le",
            "-ar", "16000", "-ac", "1",
            &audio_path.to_string_lossy(),
        ])
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                TextifierError::FfmpegNotFound
            } else {
                TextifierError::DownloadFailed(format!("ffmpeg error: {}", e))
            }
        })?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let msg = stderr.trim().to_string();
        return Err(TextifierError::DownloadFailed(format!("ffmpeg: {}", &msg[..msg.len().min(200)])));
    }

    if audio_path.exists() {
        let size = audio_path.metadata().map(|m| m.len() as f64 / 1024.0).unwrap_or(0.0);
        println!("  ✓ 音频提取完成 ({:.0} KB)", size);
        Ok(Some(audio_path))
    } else {
        Ok(None)
    }
}

// ── Transcription (whisper-rs native) ──────────────────────────────

fn transcribe_audio(audio_path: &Path, model_size: &str) -> Result<Option<String>, TextifierError> {
    transcribe_whisper_rs(audio_path, model_size)
}

// ── whisper-rs (native) ───────────────────────────────────────────

fn transcribe_whisper_rs(audio_path: &Path, model_size: &str) -> Result<Option<String>, TextifierError> {
    let model_path = find_or_download_model(model_size)?;

    // Read WAV file into f32 samples (whisper.cpp expects f32)
    let samples_i16 = read_wav_i16(audio_path)?;
    let samples: Vec<f32> = samples_i16.iter().map(|&s| s as f32 / 32768.0).collect();

    println!("  ↓ 加载 Whisper 模型 ({}, cpu)...", model_size);
    let ctx = whisper_rs::WhisperContext::new_with_params(
        &model_path,
        whisper_rs::WhisperContextParameters::default(),
    )
    .map_err(|e| TextifierError::TranscriptionFailed(format!("whisper load: {}", e)))?;

    let mut state = ctx.create_state()
        .map_err(|e| TextifierError::TranscriptionFailed(format!("whisper state: {}", e)))?;

    let mut params = whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("zh"));
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    println!("  ↓ 转写音频中...");
    state.full(params, &samples[..])
        .map_err(|e| TextifierError::TranscriptionFailed(format!("whisper full: {}", e)))?;

    let num_segments = state.full_n_segments();
    let mut text_parts = Vec::new();
    for i in 0..num_segments {
        if let Some(segment) = state.get_segment(i) {
            text_parts.push(segment.to_string());
        }
    }

    let full_text = text_parts.join(" ");
    if full_text.is_empty() {
        println!("  ⚠ 转写结果为空");
        Ok(None)
    } else {
        println!("  ✓ 转写完成 (约{}字)", full_text.chars().count());
        Ok(Some(full_text))
    }
}

fn find_or_download_model(size: &str) -> Result<String, TextifierError> {
    let cache_dir = dirs_next().join(".cache").join("whisper-rs");
    std::fs::create_dir_all(&cache_dir).ok();

    let model_file = format!("ggml-{}.bin", size);
    let model_path = cache_dir.join(&model_file);

    if model_path.exists() {
        return Ok(model_path.to_string_lossy().to_string());
    }

    // Clean up stale partial download from previous interrupted run
    let tmp_path = cache_dir.join(format!("{}.tmp", model_file));
    if tmp_path.exists() {
        println!("  ↓ 清除上次未完成的下载缓存...");
        let _ = std::fs::remove_file(&tmp_path);
    }

    // Try multiple mirrors in case HuggingFace is slow/unreachable
    let urls: [&str; 3] = [
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main",
        "https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main",
        "https://cdn-lfs-us-1.hf.co/repos/ggerganov/whisper.cpp/resolve/main",
    ];

    println!("  ↓ 下载 Whisper 模型 ({}, 约 1-3 GB，请耐心等待)...", size);

    let client = reqwest::blocking::Client::builder()
        .timeout(None) // no timeout for large files
        .build()
        .map_err(|e| TextifierError::DownloadFailed(format!("client: {}", e)))?;

    let mut last_err = String::new();
    for (i, base) in urls.iter().enumerate() {
        if i > 0 {
            println!("  ↓ 尝试镜像 {} ...", base);
            // Clean up partial from previous attempt
            let _ = std::fs::remove_file(&tmp_path);
        }

        let url = format!("{}/{}", base, model_file);
        match try_download(&client, &url, &tmp_path, &model_path) {
            Ok(path) => return Ok(path),
            Err(e) => {
                last_err = e;
                continue;
            }
        }
    }

    Err(TextifierError::DownloadFailed(format!(
        "all mirrors failed, last error: {}", last_err
    )))
}

/// Download from a single URL, verify size, rename atomically.
/// Returns the model path on success.
fn try_download(
    client: &reqwest::blocking::Client,
    url: &str,
    tmp_path: &std::path::Path,
    model_path: &std::path::Path,
) -> Result<String, String> {
    let mut resp = client
        .get(url)
        .send()
        .map_err(|e| format!("download: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let total = resp.content_length();
    if let Some(t) = total {
        let total_mb = t as f64 / 1_048_576.0;
        println!("  ↓ 模型大小: {:.0} MB, 下载中...", total_mb);
    }

    let mut file =
        std::fs::File::create(tmp_path).map_err(|e| format!("create tmp: {}", e))?;

    resp.copy_to(&mut file)
        .map_err(|e| format!("download: {}", e))?;

    // Verify download size before renaming
    let actual_size = tmp_path.metadata().map(|m| m.len()).unwrap_or(0);
    if let Some(expected) = total {
        if actual_size != expected {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(format!(
                "incomplete: {:.0}/{:.0} MB",
                actual_size as f64 / 1_048_576.0,
                expected as f64 / 1_048_576.0
            ));
        }
    }

    std::fs::rename(&tmp_path, model_path)
        .map_err(|e| format!("rename: {}", e))?;

    let size_mb = actual_size as f64 / 1_048_576.0;
    println!("  ✓ 模型下载完成 ({:.0} MB)", size_mb);
    Ok(model_path.to_string_lossy().to_string())
}

fn dirs_next() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home)
}

fn read_wav_i16(path: &Path) -> Result<Vec<i16>, TextifierError> {
    let file = std::fs::File::open(path)
        .map_err(|e| TextifierError::TranscriptionFailed(format!("open wav: {}", e)))?;

    let mut reader = std::io::BufReader::new(file);

    // Read WAV header
    let mut header = [0u8; 44];
    std::io::Read::read_exact(&mut reader, &mut header)
        .map_err(|e| TextifierError::TranscriptionFailed(format!("read wav header: {}", e)))?;

    // Verify RIFF/WAVE
    if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
        return Err(TextifierError::TranscriptionFailed("invalid WAV file".into()));
    }

    // Verify PCM 16-bit mono
    let channels = u16::from_le_bytes([header[22], header[23]]);
    let bits_per_sample = u16::from_le_bytes([header[34], header[35]]);
    if channels != 1 || bits_per_sample != 16 {
        return Err(TextifierError::TranscriptionFailed(
            format!("expected 16-bit mono, got {}ch {}bit", channels, bits_per_sample)
        ));
    }

    // Read PCM data
    let mut pcm = Vec::new();
    std::io::Read::read_to_end(&mut reader, &mut pcm)
        .map_err(|e| TextifierError::TranscriptionFailed(format!("read pcm: {}", e)))?;

    // Convert u8 bytes to i16 samples (little-endian)
    let samples: Vec<i16> = pcm
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    Ok(samples)
}

// ── Orchestration ─────────────────────────────────────────────────

async fn transcribe_video(url: &str, whisper_model: &str) -> Result<String, TextifierError> {
    let url = url.to_string();
    let output_dir = tempfile::tempdir()
        .map_err(|e| TextifierError::DownloadFailed(format!("tempdir: {}", e)))?;
    let dir = output_dir.path().to_path_buf();

    let video = {
        let d = dir.clone();
        let u = url.clone();
        tokio::task::spawn_blocking(move || download_video(&u, &d)).await
            .map_err(|e| TextifierError::DownloadFailed(format!("task: {}", e)))?
            .map_err(|e| { println!("  ✗ 视频下载失败: {}", e); e })?
    };
    let video = match video { Some(v) => v, None => return Ok(String::new()) };

    let audio = {
        let v = video.clone();
        let d = dir.clone();
        tokio::task::spawn_blocking(move || extract_audio(&v, &d)).await
            .map_err(|e| TextifierError::TranscriptionFailed(format!("task: {}", e)))?
            .map_err(|e| { println!("  ✗ 音频提取失败: {}", e); e })?
    };
    let audio = match audio { Some(a) => a, None => return Ok(String::new()) };

    let model = whisper_model.to_string();
    let transcript = tokio::task::spawn_blocking(move || transcribe_audio(&audio, &model)).await
        .map_err(|e| TextifierError::TranscriptionFailed(format!("task: {}", e)))?
        .map_err(|e| { println!("  ✗ 转写失败: {}", e); e })?;

    Ok(transcript.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::RawContent;

    #[test]
    fn test_process_no_video() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let raw = RawContent {
            title: "测试标题".into(), text_content: "测试描述".into(),
            image_urls: vec![], has_video: false, video_url: None,
            source: "test".into(), source_url: "https://example.com".into(),
        };
        let result = rt.block_on(process(&raw, "medium")).unwrap();
        assert!(result.full_text.contains("测试标题"));
        assert!(!result.full_text.contains("视频口述内容"));
    }

    #[test]
    fn test_read_wav_invalid() {
        let tmp = std::env::temp_dir().join("test_not_wav.bin");
        std::fs::write(&tmp, b"not a wav file").ok();
        let result = read_wav_i16(&tmp);
        assert!(result.is_err());
        let _ = std::fs::remove_file(&tmp);
    }
}
