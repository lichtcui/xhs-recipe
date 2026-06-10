"""视频处理与语音转写模块。

处理流程：
1. 使用 yt-dlp 下载小红书视频（支持无水印）
2. 使用 ffmpeg 提取音频
3. 使用 faster-whisper 转写为文字
"""

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Optional

from .models import XHSContent


def _find_yt_dlp() -> str:
    """查找 yt-dlp 可执行文件路径（支持 venv 内安装的情况）。"""
    # 1. PATH 查找
    path = shutil.which("yt-dlp")
    if path:
        return path
    # 2. 当前 Python 环境的 bin 目录
    venv_bin = Path(sys.executable).parent / "yt-dlp"
    if venv_bin.exists():
        return str(venv_bin)
    # 3. pip 安装的包
    try:
        import yt_dlp
        pkg_path = Path(yt_dlp.__file__).parent.parent.parent
        bin_path = pkg_path / "bin" / "yt-dlp"
        if bin_path.exists():
            return str(bin_path)
    except ImportError:
        pass
    raise FileNotFoundError(
        "yt-dlp 未找到，请执行 'pip install yt-dlp' 安装"
    )


async def download_video(
    url: str,
    output_dir: Optional[Path] = None,
) -> Optional[Path]:
    """使用 yt-dlp 下载小红书视频。

    Args:
        url: 小红书笔记 URL
        output_dir: 输出目录，默认使用系统临时目录

    Returns:
        下载的视频文件路径，失败返回 None
    """
    if output_dir is None:
        output_dir = Path(tempfile.mkdtemp(prefix="xhs_recipe_"))
    output_dir.mkdir(parents=True, exist_ok=True)

    output_template = str(output_dir / "%(id)s.%(ext)s")

    try:
        yt_dlp_path = _find_yt_dlp()
    except FileNotFoundError as e:
        print(f"错误: {e}")
        return None

    try:
        subprocess.run(
            [
                yt_dlp_path,
                "--quiet",
                "--no-warnings",
                "--no-playlist",
                "-f", "best[ext=mp4]/best",
                "-o", output_template,
                url,
            ],
            check=True,
            capture_output=True,
            text=True,
            timeout=120,
        )
    except subprocess.CalledProcessError as e:
        print(f"yt-dlp 下载失败: {e.stderr}")
        return None
    except FileNotFoundError:
        print("错误: 未找到 yt-dlp")
        return None

    # 找到下载的文件
    for f in output_dir.iterdir():
        if f.suffix in (".mp4", ".webm", ".mkv"):
            return f
    return None


def extract_audio(video_path: Path, output_dir: Optional[Path] = None) -> Optional[Path]:
    """从视频中提取音频（16kHz mono WAV）。

    Args:
        video_path: 视频文件路径
        output_dir: 输出目录，默认与视频同目录

    Returns:
        音频文件路径
    """
    if output_dir is None:
        output_dir = video_path.parent

    audio_path = output_dir / f"{video_path.stem}.wav"

    try:
        subprocess.run(
            [
                "ffmpeg",
                "-y",
                "-i", str(video_path),
                "-vn",
                "-acodec", "pcm_s16le",
                "-ar", "16000",
                "-ac", "1",
                str(audio_path),
            ],
            check=True,
            capture_output=True,
            text=True,
            timeout=120,
        )
    except subprocess.CalledProcessError as e:
        print(f"ffmpeg 音频提取失败: {e.stderr}")
        return None
    except FileNotFoundError:
        print("错误: 未找到 ffmpeg，请安装 ffmpeg")
        return None

    return audio_path if audio_path.exists() else None


def transcribe(
    audio_path: Path,
    model_size: str = "medium",
    language: str = "zh",
    device: str = "auto",
) -> Optional[str]:
    """使用 faster-whisper 将音频转写为文字。

    Args:
        audio_path: 音频文件路径
        model_size: Whisper 模型大小 (tiny/base/small/medium/large-v3)
        language: 语言代码
        device: 运行设备 (auto/cpu/cuda)

    Returns:
        转写文本
    """
    try:
        from faster_whisper import WhisperModel
    except ImportError:
        print("错误: 未安装 faster-whisper，请执行 'pip install faster-whisper'")
        return None

    # 自动检测设备
    if device == "auto":
        try:
            import torch
            device = "cuda" if torch.cuda.is_available() else "cpu"
        except ImportError:
            device = "cpu"
        compute_type = "float16" if device == "cuda" else "int8"
    else:
        compute_type = "float16" if device == "cuda" else "int8"

    print(f"  加载 Whisper 模型 ({model_size}, {device})...")
    try:
        model = WhisperModel(model_size, device=device, compute_type=compute_type)
    except Exception as e:
        print(f"  Whisper 模型加载失败，尝试 CPU int8: {e}")
        model = WhisperModel(model_size, device="cpu", compute_type="int8")

    print(f"  转写音频中...")
    segments, info = model.transcribe(
        str(audio_path),
        language=language,
        beam_size=5,
        vad_filter=True,
    )

    text_parts = []
    for segment in segments:
        text_parts.append(segment.text)

    return " ".join(text_parts)


async def process_video(
    content: XHSContent,
    url: str,
    output_dir: Optional[Path] = None,
    whisper_model: str = "medium",
) -> str:
    """完整视频处理流程：下载 → 提取音频 → 转写。

    Args:
        content: XHS 笔记内容（用于判断是否需要处理视频）
        url: 小红书笔记 URL
        output_dir: 工作目录
        whisper_model: Whisper 模型大小

    Returns:
        转写的文字内容（非视频笔记返回空字符串）
    """
    if content.note_type != "video":
        return ""

    if output_dir:
        output_dir = Path(output_dir) / "video"
    else:
        output_dir = Path(tempfile.mkdtemp(prefix="xhs_video_"))

    print("  ↓ 下载视频...")
    video_path = await download_video(url, output_dir)
    if not video_path:
        print("  ⚠ 视频下载失败，跳过转写")
        return ""

    print("  ↓ 提取音频...")
    audio_path = extract_audio(video_path, output_dir)
    if not audio_path:
        print("  ⚠ 音频提取失败，跳过转写")
        return ""

    print("  ↓ 语音转写...")
    transcript = transcribe(audio_path, model_size=whisper_model)
    if not transcript:
        print("  ⚠ 转写失败")
        return ""

    return transcript.strip()
