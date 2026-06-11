# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

CLI tool to extract structured Chinese recipes from 小红书 (Xiaohongshu / RedNote) post URLs, written in Rust.

## Build & Run

```bash
# Build
cargo build

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
  → src/main.rs                # Clap CLI
  → src/pipeline.rs            # fetch → textify → analyze
    → src/sources/base.rs      # zendriver-rs browser automation (Rust-native)
    → src/textifier.rs         # yt-dlp + ffmpeg + whisper-rs transcription
    → src/analyzer.rs          # reqwest → DeepSeek API (function calling)
  → src/presentation/          # Terminal render + .md/.json save
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
# Run all tests
cargo test                   # 48 lib + 7 bin = 55 tests
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
├── textifier.rs          # yt-dlp + ffmpeg + whisper-rs
├── analyzer.rs           # LLM function calling (reqwest → DeepSeek)
├── sources/
│   ├── mod.rs            # Source routing
│   ├── base.rs           # zendriver-rs browser automation
│   └── xiaohongshu/
│       ├── mod.rs
│       ├── auth.rs       # Cookie / login
│       ├── scraper.rs    # Scrape fallback chain
│       └── url.rs        # URL parsing
└── presentation/
    ├── mod.rs
    ├── render.rs         # Terminal output (colored)
    └── save.rs           # .md / .json file output
```
