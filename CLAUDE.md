# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

CLI tool to extract structured Chinese recipes from 小红书 (Xiaohongshu / RedNote) post URLs, written in Rust.

## Build & Run

```bash
# Build
cargo build

# Run with Qwen3-ASR transcription
cargo run -- extract <xhs-url>
cargo run -- extract <xhs-url> -o recipe.md           # save to file
cargo run -- extract <xhs-url> --no-images             # skip image OCR
cargo run -- extract <xhs-url> --asr-model qwen3-asr-1.7b  # higher accuracy model

# Local storage (auto-saved after each extract)
cargo run -- list                  # list saved recipes
cargo run -- show <id>             # view a saved recipe

# Install Qwen3-ASR (required for transcription)
cargo install qwen-asr-cli
qwen-asr download qwen3-asr-0.6b

# 国内源（ModelScope）下载方式:
#   brew install git-lfs && git lfs install
#   git clone https://www.modelscope.cn/Qwen/Qwen3-ASR-0.6B.git \
#     ~/.cache/qwen-asr/qwen3-asr-0.6b
#   rm -rf ~/.cache/qwen-asr/qwen3-asr-0.6b/.git

# Manual login (if auto-cookie fails)
cargo run -- login [--headless]

# Clear saved cookies
cargo run -- logout

# Security audit
cargo audit

# Check deps
cargo run -- setup
```

Prerequisites: `yt-dlp` (brew install yt-dlp), `ffmpeg` (brew install ffmpeg), Xcode Command Line Tools (`swiftc` for macOS Vision OCR, `ffmpeg` for video frame extraction).

## Architecture

数据流单向分层：**Source Adapter → Textifier → Analyzer**

```
cargo run -- extract <url>
  → src/main.rs                # Clap CLI
  → src/pipeline.rs            # fetch → textify → analyze
    → src/sources/base.rs      # URL routing → xiaohongshu adapter
    → src/textifier.rs         # yt-dlp + symphonia + Qwen3-ASR + macOS Vision OCR
    → src/analyzer.rs          # reqwest → DeepSeek API (function calling)
  → src/presentation/          # Terminal render + .md/.json save
  → src/storage/               # Auto-save to ~/.xhs-recipe/recipes/
```

## Data Flow

1. **`main.rs`** 先查本地缓存（`Storage::get_by_source_url`），命中则直接显示并跳过后续步骤
2. **`pipeline.extract()`** 接收 URL，调用 `sources.fetch(url)` 路由到对应适配器
3. **Source Adapter**（如 `sources/xiaohongshu/`）：zendriver-rs 浏览器自动化抓取页面，返回平台无关的 `RawContent`
4. **`textifier.process()`**：视频 → yt-dlp + symphonia + Qwen3-ASR 转写 + ffmpeg + macOS Vision 帧 OCR，返回 `TextContent`
5. **`analyzer::extract_recipe()`**：OCR 文本 + 描述文字 → LLM function calling → `Recipe` 模型
6. **`storage`**：提取后自动保存到 `~/.xhs-recipe/recipes/`。同一 URL 重复提取时自动去重，跳过保存
7. **`presentation`**：终端 rich 渲染 / 保存 `.md` 或 `.json`

## Key Design Decisions

- **数据流单向**：sources/ 不依赖 analyzer，analyzer 不关心内容来源
- **RawContent（平台无关）**：各 Source Adapter 统一输出 `RawContent`，新增来源只需新写 adapter
- **Cookie auth**：zendriver-rs 管理浏览器 cookie
- **Video download**：yt-dlp 子进程下载，symphonia（纯 Rust）提取音频 → Qwen3-ASR 转写
- **Video OCR**：ffmpeg 提取关键帧 → macOS Vision 框架（VNRecognizeTextRequest）识别画面中文字
- **ASR + OCR 并行**：下载视频后，音频转写与帧 OCR 同时执行，结果合并后送入 LLM
- **Image OCR**：`send_images` 控制是否为图文笔记执行 OCR；图片在 textifier 阶段 OCR 为文字后送入 LLM，而非直接发送图片
- **Fallback chain for scraping**：zendriver（`__NEXT_DATA__` → DOM selectors）→ reqwest HTTP fallback（`__NEXT_DATA__` → og:meta）
- **CLI uses clap** for argument parsing
- **Dedup & cache**：同一 URL 重复提取时先查本地缓存，命中则跳过 pipeline 直接显示；`save()` 自动去重不产生重复条目

## Test

```bash
# Run all tests
cargo test                   # 78 lib + 11 bin + 4 integration = 93 tests
cargo audit                  # Security audit (install: cargo install cargo-audit)
cargo test --lib             # Library tests only
cargo test --bin xhs-recipe  # Binary (CLI) tests only

# Run with real URL (requires network + API key + cookies)
cargo run -- extract <xhs-url>
```

## Source Layout

Single crate with `src/lib.rs` + `src/main.rs`:

```
src/
├── lib.rs                # Library root
├── main.rs               # Binary (CLI, clap)
├── models.rs             # Data models (serde)
├── pipeline.rs           # Orchestration: fetch → textify → analyze
├── textifier.rs          # yt-dlp + symphonia + Qwen3-ASR + macOS Vision OCR
├── analyzer.rs           # LLM function calling (reqwest → DeepSeek)
├── sources/
│   ├── mod.rs            # Source routing
│   ├── base.rs           # URL routing & domain checking
│   └── xiaohongshu/
│       ├── mod.rs
│       ├── auth.rs       # Cookie / login
│       ├── scraper.rs    # Scrape fallback chain
│       └── url.rs        # URL parsing
├── storage/
│   ├── mod.rs            # Storage trait (save/list/get/get_by_source_url/delete)
│   └── local.rs          # LocalStorage: ~/.xhs-recipe/recipes/*.json
└── presentation/
    ├── mod.rs
    ├── render.rs         # Terminal output (colored)
    └── save.rs           # .md / .json file output
```
