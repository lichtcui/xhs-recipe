# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

CLI tool to extract structured Chinese recipes from 小红书 (Xiaohongshu / RedNote) post URLs, written in Rust.

## Build & Run

```bash
# Build all crates
cargo build --workspace

# Run
cargo run -- extract <xhs-url>
cargo run -- extract <xhs-url> --output recipe.md
cargo run -- extract <xhs-url> --no-images

# Manual login (if auto-cookie fails)
cargo run -- login [--headless]

# Clear saved cookies
cargo run -- logout

# Check deps
cargo run -- setup
```

Prerequisites: `ffmpeg` (brew install ffmpeg), `yt-dlp` (brew install yt-dlp).

## Architecture

数据流单向分层：**Source Adapter → Textifier → Analyzer**

```
cargo run -- extract <url>
  → cli/src/main.rs          # Clap CLI
  → pipeline/src/lib.rs      # fetch → textify → analyze
    → sources/src/base.rs    # zendriver-rs browser automation (Rust-native)
    → textifier/src/lib.rs   # yt-dlp + ffmpeg + whisper-rs transcription
    → analyzer/src/lib.rs    # reqwest → DeepSeek API (function calling)
  → presentation/src/        # Terminal render + .md/.json save
```

## Data Flow

1. **`pipeline.extract()`** 接收 URL，调用 `sources.fetch(url)` 路由到对应适配器
2. **Source Adapter**（如 `sources/xiaohongshu/`）：zendriver-rs 抓取页面，返回平台无关的 `RawContent`
3. **`textifier.process()`**：视频 → yt-dlp + ffmpeg + whisper-rs 转写，返回 `TextContent`
4. **`analyzers.recipe.extract_recipe()`**：纯文本 + 可选图片 → LLM function calling → `Recipe` 模型
5. **`presentation`**：终端 rich 渲染 / 保存 `.md` 或 `.json`

## Key Design Decisions

- **数据流单向**：sources/ 不依赖 analyzers/，analyzers/ 不关心内容来源
- **RawContent（平台无关）**：各 Source Adapter 统一输出 `RawContent`，新增来源只需新写 adapter
- **Cookie auth**：zendriver-rs 管理浏览器 cookie
- **Video download**：yt-dlp 子进程，在 textifier 中完成下载 → 提音频 → whisper-rs 转写
- **Image selection**：仅发送最多 3 张图片给 LLM（`--images`/`--no-images` 控制）
- **Fallback chain for scraping**：`__NEXT_DATA__` → DOM selectors → API response interception
- **CLI uses clap** for argument parsing

## Test

```bash
# Run all 55 tests
cargo test --workspace

# Run specific crate tests
cargo test -p core         # 3 tests (models)
cargo test -p analyzer     # 22 tests (LLM parsing, images, fallback)
cargo test -p presentation # 6 tests (terminal render, golden file save)
cargo test -p sources      # 13 tests (URL routing, scraper parsing)
cargo test -p textifier    # 2 tests (text assembly)
cargo test -p pipeline     # 2 tests (orchestration)
cargo test -p cli          # 7 tests (argument parsing)

# Run with real URL (requires network + API key + cookies)
cargo run -- extract <xhs-url>
```

## Rust Crate Layout

```
crates/
├── core/            # Data models (serde)
├── presentation/    # Terminal output + file save
├── analyzer/        # LLM function calling (reqwest)
├── textifier/       # Video download + audio extraction + whisper-rs transcription
├── sources/         # Source adapters (multi-platform) with zendriver-rs browser automation
├── pipeline/        # Orchestration
└── cli/             # Binary (clap)
```
