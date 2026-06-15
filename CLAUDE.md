# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

CLI tool + HTTP server to extract structured Chinese recipes from 小红书 (Xiaohongshu / RedNote) post URLs, written in Rust. Cargo workspace with `xhs-recipe` (CLI/lib) and `xhs-recipe-server` (axum HTTP server).

## Build & Run

```bash
# Build all
cargo build

# CLI: extract recipe
cargo run -- extract <xhs-url>
cargo run -- extract <xhs-url> -o recipe.md           # save to file
cargo run -- extract <xhs-url> --no-images             # skip image OCR
cargo run -- extract <xhs-url> --asr-model qwen3-asr-1.7b  # higher accuracy model

# Server
cargo run -p xhs-recipe-server      # starts on http://127.0.0.1:3000
PORT=8080 cargo run -p xhs-recipe-server  # custom port

# Local storage (auto-saved after each extract)
cargo run -- list                  # list saved recipes
cargo run -- show <id>             # view a saved recipe

# Check deps
cargo run -- setup

# Security audit
cargo audit
```

### Prerequisites

- ffmpeg (`brew install ffmpeg` / `apt install ffmpeg`)
- tesseract + chi_sim language pack (`brew install tesseract`)
- Qwen3-ASR (`cargo install qwen-asr-cli && qwen-asr download qwen3-asr-0.6b`)

国内源（ModelScope）下载 ASR 模型:
```
brew install git-lfs && git lfs install
git clone https://www.modelscope.cn/Qwen/Qwen3-ASR-0.6B.git \
  ~/.cache/qwen-asr/qwen3-asr-0.6b
rm -rf ~/.cache/qwen-asr/qwen3-asr-0.6b/.git
```

## Architecture

数据流单向分层：**Source Adapter → Textifier → Analyzer**（CLI）/ **Source Adapter → Textifier → Splitter**（Server）

```
# CLI
cargo run -- extract <url>
  → src/main.rs                # Clap CLI
  → src/pipeline.rs            # fetch → textify → analyze
    → src/sources/base.rs      # URL routing
    → src/textifier.rs         # reqwest + symphonia + Qwen3-ASR + ffmpeg/tesseract OCR
    → src/analyzer.rs          # reqwest → DeepSeek API (function calling)

# Server
cargo run -p xhs-recipe-server
  → server/src/main.rs         # axum (port 3000)
  → server/src/routes.rs       # POST /process (SSE) + GET /health
    → src/textifier.rs         # (same lib, with progress callback)
    → server/src/splitter.rs   # item splitting for SSE
```

## Data Flow

1. **CLI `main.rs` / Server `POST /process`** — 接收 URL
2. **`sources.fetch(url)`** — reqwest HTTP 抓取页面 → 返回平台无关 `RawContent`（含 `content_type` 字段）
3. **`textifier::process()`** — 视频下载 + symphonia 音频提取 + Qwen3-ASR 转写；ffmpeg 抽帧 + tesseract OCR（全平台统一）
   - CLI 使用 `process_cli()`（println! 进度），Server 使用 `process()` 传入 `Arc<dyn Fn(&str)>` 回调驱动 SSE
4. **CLI: `analyzer::extract_recipe()`** — LLM function calling → `Recipe` 模型
   **Server: `splitter::split()`** — 按 content_type 拆分为 items → SSE 流式返回
5. **`storage`** — 自动保存到 `~/.xhs-recipe/recipes/`，同一 URL 重复提取自动去重
6. **`presentation`** — 终端 rich 渲染 / 保存 `.md` / `.json`

## Server Design

- **POST /process (SSE)** — validates URL → fetches page → textifies (download + OCR + ASR) → splits items → streams result
- **SSE stages** — `fetching` → `downloading` → `ocr` + `asr` (parallel for video) → `result`
- **Concurrency** — `Semaphore` limits concurrent /process to 3; each request gets a permit before processing
- **Timeout** — 5-minute overall timeout per request, sends `TIMEOUT` error event on expiry
- **Client disconnect** — SSE channel close detected via `mpsc::Sender::is_closed()`; aborts further work silently
- **Temp files** — each request uses `tempfile::tempdir()`; cleaned up on drop regardless of success/failure
- **`--port` flag** — `xhs-recipe-server --port 8080` or `PORT=8080` env var

## Key Design Decisions

- **数据流单向**：sources/ 不依赖 analyzer，analyzer 不关心内容来源
- **RawContent.content_type**：scraper 自动检测（Video / Image / Collection），splitter 据此决定拆项策略
- **Cargo workspace**：`server/` crate 依赖 lib crate，复用 sources + textifier + models
- **Server 不依赖 analyzer**：LLM 调用由前端处理，后端只做提取 + 拆项
- **OCR 统一 ffmpeg + tesseract**：全平台（macOS/Linux）同一套方案，不再依赖 Swift/Vision/AVFoundation
- **Progress callback**：textifier::process() 接受 `Option<Arc<dyn Fn(&str) + Send + Sync>>`，Server 通过它驱动 SSE 事件
- **Video processing**：reqwest 直链下载 → symphonia 提取音频 → Qwen3-ASR 转写；ffmpeg 抽取关键帧 → tesseract OCR
- **ASR + OCR 并行**：下载视频后音频转写与帧 OCR 同时执行
- **Cookie auth**：JSON 文件管理（~/.cache/xhs-recipe/cookies.json）
- **Scraping**：reqwest HTTP → `__NEXT_DATA__` → `__INITIAL_STATE__` → OG meta tag fallback
- **Dedup & cache**：同一 URL 重复提取时先查本地缓存，命中则跳过 pipeline

## Test

```bash
# Run all tests
cargo test                    # lib + bin + integration + server
cargo test -p xhs-recipe      # CLI/lib tests only
cargo test -p xhs-recipe-server  # Server tests only

# Security audit
cargo audit                   # install: cargo install cargo-audit
```

## Source Layout

Cargo workspace with two members:

```
xhs-recipe/
├── Cargo.toml              # workspace root
├── src/                    # lib crate (xhs-recipe)
│   ├── lib.rs              # Library root (which, vprintln!, STEP_NUMS)
│   ├── main.rs             # CLI binary (clap)
│   ├── models.rs           # Data models (RawContent, TextContent, Recipe, ContentType)
│   ├── pipeline.rs         # Orchestration: fetch → textify → analyze
│   ├── textifier.rs        # reqwest + symphonia + Qwen3-ASR + ffmpeg/tesseract
│   ├── analyzer.rs         # LLM function calling (reqwest → DeepSeek)
│   ├── sources/
│   │   ├── mod.rs          # Source routing
│   │   ├── base.rs         # URL routing & domain checking
│   │   └── xiaohongshu/
│   │       ├── mod.rs
│   │       ├── auth.rs     # Cookie management
│   │       ├── scraper.rs  # Scraper (reqwest HTTP)
│   │       └── url.rs      # URL parsing
│   ├── storage/
│   │   ├── mod.rs          # Storage trait
│   │   └── local.rs        # LocalStorage: ~/.xhs-recipe/recipes/*.json
│   └── presentation/
│       ├── mod.rs
│       ├── render.rs       # Terminal output (colored)
│       └── save.rs         # .md / .json file output
├── server/                 # server crate (xhs-recipe-server)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         # axum startup (port 3000, graceful shutdown, Semaphore)
│       ├── routes.rs       # POST /process (SSE) + GET /health
│       ├── splitter.rs     # Items splitting by content_type
│       └── error.rs        # Error codes
└── tests/                  # Integration tests
    └── integration.rs
```
