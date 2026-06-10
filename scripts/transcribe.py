#!/usr/bin/env python3
"""
Bridge: Rust calls this via subprocess to transcribe audio with faster-whisper.

Usage: python3 transcribe.py <audio_path> [model_size]

Outputs JSON on stdout: {"text": "..."}
Errors on stderr.
"""
import json, sys
from pathlib import Path

audio_path = Path(sys.argv[1])
model_size = sys.argv[2] if len(sys.argv) > 2 else "medium"

try:
    from faster_whisper import WhisperModel
except ImportError:
    print(json.dumps({"error": "faster-whisper not installed", "text": ""}))
    sys.exit(0)

model = WhisperModel(model_size, device="cpu", compute_type="int8")
segments, _ = model.transcribe(str(audio_path), language="zh", beam_size=5, vad_filter=True)
text = " ".join(s.text for s in segments)
print(json.dumps({"text": text}))
