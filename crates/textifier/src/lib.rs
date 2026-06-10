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

// ── Errors ─────────────────────────────────────────────────────────

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
            "-o", &template_str,
            url,
        ])
        .output()
        .map_err(|e| TextifierError::DownloadFailed(format!("yt-dlp exec error: {}", e)))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let msg = stderr.trim().to_string();
        return Err(TextifierError::DownloadFailed(msg));
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

// ── Transcription (Python bridge) ─────────────────────────────────

fn transcribe_audio(audio_path: &Path, model_size: &str) -> Result<Option<String>, TextifierError> {
    let python = if Command::new("python3").arg("--version").output().is_ok() {
        "python3"
    } else {
        "python"
    };

    let script_path = locate_bridge_script();
    let audio_str = audio_path.to_string_lossy().to_string();

    println!("  ↓ 加载 Whisper 模型 ({}, cpu)...", model_size);
    println!("  ↓ 转写音频中...");

    let result = Command::new(python)
        .args([&script_path, &audio_str, model_size])
        .output()
        .map_err(|e| TextifierError::TranscriptionFailed(format!("subprocess: {}", e)))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(TextifierError::TranscriptionFailed(stderr.trim().to_string()));
    }

    let stdout = String::from_utf8_lossy(&result.stdout);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        if let Some(text) = json["text"].as_str() {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                println!("  ✓ 转写完成 (约{}字)", trimmed.chars().count());
                return Ok(Some(trimmed.to_string()));
            }
        }
        // Check for error field
        if let Some(err) = json["error"].as_str() {
            println!("  ⚠ {}", err);
        }
    }

    println!("  ⚠ 转写结果为空");
    Ok(None)
}

fn locate_bridge_script() -> String {
    // Try multiple locations relative to cwd and binary
    for p in &[
        "scripts/transcribe.py",
        "../scripts/transcribe.py",
    ] {
        if Path::new(p).exists() {
            return p.to_string();
        }
    }
    // Try relative to binary
    if let Ok(exe) = std::env::current_exe() {
        let mut probe = exe.clone();
        probe.pop(); // remove binary name
        // walk up to find scripts/
        for _ in 0..4 {
            let candidate = probe.join("scripts").join("transcribe.py");
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
            probe.pop();
        }
    }
    "scripts/transcribe.py".to_string()
}

// ── Orchestration ─────────────────────────────────────────────────

async fn transcribe_video(url: &str, whisper_model: &str) -> Result<String, TextifierError> {
    let url = url.to_string();
    let output_dir = tempfile::tempdir()
        .map_err(|e| TextifierError::DownloadFailed(format!("tempdir: {}", e)))?;
    let dir = output_dir.path().to_path_buf();

    // Download
    let video = {
        let d = dir.clone();
        let u = url.clone();
        tokio::task::spawn_blocking(move || download_video(&u, &d))
            .await
            .map_err(|e| TextifierError::DownloadFailed(format!("task: {}", e)))?
            .map_err(|e| {
                println!("  ✗ 视频下载失败: {}", e);
                e
            })?
    };
    let video = match video {
        Some(v) => v,
        None => return Ok(String::new()),
    };

    // Extract audio
    let audio = {
        let v = video.clone();
        let d = dir.clone();
        tokio::task::spawn_blocking(move || extract_audio(&v, &d))
            .await
            .map_err(|e| TextifierError::DownloadFailed(format!("task: {}", e)))?
            .map_err(|e| {
                println!("  ✗ 音频提取失败: {}", e);
                e
            })?
    };
    let audio = match audio {
        Some(a) => a,
        None => return Ok(String::new()),
    };

    // Transcribe
    let model = whisper_model.to_string();
    let transcript = tokio::task::spawn_blocking(move || transcribe_audio(&audio, &model))
        .await
        .map_err(|e| TextifierError::TranscriptionFailed(format!("task: {}", e)))?
        .map_err(|e| {
            println!("  ✗ 转写失败: {}", e);
            e
        })?;

    // output_dir (TempDir) dropped here, cleans up temp files
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
            title: "测试标题".into(),
            text_content: "测试描述".into(),
            image_urls: vec![],
            has_video: false,
            video_url: None,
            source: "test".into(),
            source_url: "https://example.com".into(),
        };
        let result = rt.block_on(process(&raw, "medium")).unwrap();
        assert!(result.full_text.contains("测试标题"));
        assert!(result.full_text.contains("测试描述"));
        assert!(!result.full_text.contains("视频口述内容"));
    }
}
