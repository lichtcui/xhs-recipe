"""媒体转文字模块。

将 RawContent 中的视频转写、图片 caption 等统一转为纯文本。
"""

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Optional

from .models import RawContent, TextContent


async def process(raw: RawContent, whisper_model: str = "medium") -> TextContent:
    """将 RawContent 统一转为文字。

    - 视频笔记 → 下载 + 转写
    - 图片笔记 → 保持文字
    """
    text_parts = [f"标题：{raw.title}"]
    if raw.text_content:
        text_parts.append(f"描述：{raw.text_content}")

    if raw.has_video:
        print("  ↓ 下载视频...")
        transcript = await _transcribe_video(raw.source_url, whisper_model=whisper_model)
        if transcript:
            text_parts.append(f"视频口述内容：\n{transcript}")
    else:
        print("  ✓ 图文笔记，无需转写")

    return TextContent(
        full_text="\n\n".join(text_parts),
        title=raw.title,
        source=raw.source,
        source_url=raw.source_url,
    )


# ── 视频转写（yt-dlp → ffmpeg → faster-whisper）──


def _find_yt_dlp() -> str:
    path = shutil.which("yt-dlp")
    if path:
        return path
    venv_bin = Path(sys.executable).parent / "yt-dlp"
    if venv_bin.exists():
        return str(venv_bin)
    try:
        import yt_dlp
        pkg_path = Path(yt_dlp.__file__).parent.parent.parent
        bin_path = pkg_path / "bin" / "yt-dlp"
        if bin_path.exists():
            return str(bin_path)
    except ImportError:
        pass
    raise FileNotFoundError("yt-dlp 未找到，请执行 'pip install yt-dlp' 安装")


async def _download_video(url: str, output_dir: Optional[Path] = None) -> Optional[Path]:
    if output_dir is None:
        output_dir = Path(tempfile.mkdtemp(prefix="xhs_video_"))
    output_dir.mkdir(parents=True, exist_ok=True)
    output_template = str(output_dir / "%(id)s.%(ext)s")

    try:
        yt_dlp_path = _find_yt_dlp()
    except FileNotFoundError:
        return None

    print(f"  ↓ 下载视频（yt-dlp）...")
    try:
        result = subprocess.run(
            [
                yt_dlp_path, "--quiet", "--no-warnings", "--no-playlist",
                "-f", "best[ext=mp4]/best",
                "-o", output_template, url,
            ],
            check=True, capture_output=True, text=True, timeout=120,
        )
        print(f"  ✓ 视频下载完成")
    except subprocess.CalledProcessError as e:
        stderr = e.stderr.strip() or e.stdout.strip() or str(e)
        print(f"  ✗ 视频下载失败: {stderr[:200]}")
        return None
    except subprocess.TimeoutExpired:
        print("  ✗ 视频下载超时（120s）")
        return None
    except FileNotFoundError:
        print("  ✗ ffmpeg 未找到，请执行 brew install ffmpeg")
        return None

    for f in output_dir.iterdir():
        if f.suffix in (".mp4", ".webm", ".mkv"):
            print(f"  ✓ 视频文件: {f.name} ({f.stat().st_size / 1024 / 1024:.1f} MB)")
            return f
    print("  ✗ 未找到下载的视频文件")
    return None


def _extract_audio(video_path: Path, output_dir: Optional[Path] = None) -> Optional[Path]:
    if output_dir is None:
        output_dir = video_path.parent
    audio_path = output_dir / f"{video_path.stem}.wav"

    print("  ↓ 提取音频...")
    try:
        result = subprocess.run(
            [
                "ffmpeg", "-y",
                "-i", str(video_path),
                "-vn", "-acodec", "pcm_s16le",
                "-ar", "16000", "-ac", "1",
                str(audio_path),
            ],
            check=True, capture_output=True, text=True, timeout=120,
        )
        print(f"  ✓ 音频提取完成 ({audio_path.stat().st_size / 1024:.0f} KB)")
    except subprocess.CalledProcessError as e:
        stderr = e.stderr.strip() or str(e)
        print(f"  ✗ 音频提取失败: {stderr[:200]}")
        return None
    except subprocess.TimeoutExpired:
        print("  ✗ 音频提取超时（120s）")
        return None
    except FileNotFoundError:
        print("  ✗ ffmpeg 未找到，请执行 brew install ffmpeg")
        return None

    return audio_path if audio_path.exists() else None


def _transcribe_audio(
    audio_path: Path,
    model_size: str = "medium",
    language: str = "zh",
    device: str = "auto",
) -> Optional[str]:
    try:
        from faster_whisper import WhisperModel
    except ImportError as e:
        print(f"  ✗ faster-whisper 未安装: {e}")
        return None

    if device == "auto":
        try:
            import torch
            device = "cuda" if torch.cuda.is_available() else "cpu"
        except ImportError:
            device = "cpu"
        compute_type = "float16" if device == "cuda" else "int8"
    else:
        compute_type = "float16" if device == "cuda" else "int8"

    print(f"  ↓ 加载 Whisper 模型 ({model_size}, {device})...")
    try:
        model = WhisperModel(model_size, device=device, compute_type=compute_type)
    except Exception as e:
        print(f"  ⚠ Whisper 加载失败 ({e})，尝试 CPU int8 回退...")
        try:
            model = WhisperModel(model_size, device="cpu", compute_type="int8")
        except Exception as e2:
            print(f"  ✗ Whisper 模型加载失败: {e2}")
            return None

    print("  ↓ 转写音频中...")
    try:
        segments, info = model.transcribe(
            str(audio_path),
            language=language,
            beam_size=5,
            vad_filter=True,
        )

        text_parts = []
        for segment in segments:
            text_parts.append(segment.text)

        if text_parts:
            full = " ".join(text_parts)
            print(f"  ✓ 转写完成 (约{len(full)}字)")
            return full
        else:
            print("  ⚠ 转写结果为空")
            return None
    except Exception as e:
        print(f"  ✗ 转写失败: {e}")
        return None


async def _transcribe_video(url: str, whisper_model: str = "medium") -> str:
    """下载视频 → 提取音频 → 转写为文字。"""
    output_dir = Path(tempfile.mkdtemp(prefix="xhs_video_"))

    video_path = await _download_video(url, output_dir)
    if not video_path:
        return ""

    audio_path = _extract_audio(video_path, output_dir)
    if not audio_path:
        return ""

    transcript = _transcribe_audio(audio_path, model_size=whisper_model)
    return transcript.strip() if transcript else ""
