# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

CLI tool to extract structured Chinese recipes from 小红书 (Xiaohongshu / RedNote) post URLs. 

**Python version** (original): fetches note content via Playwright, transcribes video audio with yt-dlp + faster-whisper, and extracts structured recipe data via LLM API.

**Rust version** (in progress): same layered architecture, with Python bridge scripts for Playwright scraping and faster-whisper transcription. The Rust binary `xhs-recipe` is a drop-in replacement.

## Build & Run

```bash
# Install
pip install -e .

# Install Playwright browser
playwright install chromium

# Run
xhs-recipe extract <xhs-url>
xhs-recipe extract <xhs-url> --output recipe.md
xhs-recipe extract <xhs-url> --no-images --model claude-3-5-sonnet-20241022

# Manual login (if auto-cookie fails)
xhs-recipe login [--headless]

# Clear saved cookies
xhs-recipe logout

# Check deps
xhs-recipe setup
```

Prerequisites: `ffmpeg` (brew install ffmpeg), `yt-dlp` (brew install yt-dlp).

## Architecture

数据流单向分层：**Source Adapter → Textifier → Analyzer**

```
xhs_recipe/
├── __init__.py
├── __main__.py           # python -m xhs_recipe entry
├── cli.py                # 薄 CLI 层：仅命令定义，委托给 pipeline
├── pipeline.py           # 流程编排：fetch → textify → analyze
├── models.py             # 数据模型：RawContent, TextContent, Recipe, ...
├── presentation.py       # 终端渲染 + 文件保存
├── textifier.py          # 媒体转文字（视频转写 + 图片 caption）
│
├── sources/              # 来源适配器（按平台拆分）
│   ├── __init__.py
│   ├── base.py           # 路由 + URL 检测
│   └── xiaohongshu/      # 小红书
│       ├── __init__.py   # fetch() 入口
│       ├── url.py        # 短链解析、笔记 ID 提取
│       ├── auth.py       # 扫码登录 + Cookie 管理
│       └── scraper.py    # Playwright 页面抓取
│
└── analyzers/            # AI 分析器（按能力拆分）
    ├── __init__.py
    └── recipe.py         # LLM function calling -> 菜谱
```

## Python Dependencies

| Package | Purpose |
|---------|---------|
| typer | CLI argument parsing |
| rich | Terminal rendering (tables, panels, markdown) |
| pydantic | Data model validation |
| playwright | Browser automation |
| yt-dlp | Video download |
| httpx | Async HTTP client (LLM API) |
| openai | OpenAI-compatible API client |
| python-dotenv | .env file loading |
| faster-whisper | (optional) Audio transcription |

## Data Flow

1. **`pipeline.extract()`** 接收 URL，调用 `sources.fetch(url)` 路由到对应适配器
2. **Source Adapter**（如 `sources/xiaohongshu/`）：Playwright 抓取页面，返回平台无关的 `RawContent`
3. **`textifier.process()`**：视频 → yt-dlp + ffmpeg + faster-whisper 转写，返回 `TextContent`
4. **`analyzers.recipe.extract_recipe()`**：纯文本 + 可选图片 → LLM function calling → `Recipe` 模型
5. **`presentation`**：终端 rich 渲染 / 保存 `.md` 或 `.json`

## Key Design Decisions

- **数据流单向**：sources/ 不依赖 analyzers/，analyzers/ 不关心内容来源
- **RawContent（平台无关）**：各 Source Adapter 统一输出 `RawContent`，新增来源只需新写 adapter
- **Cookie auth**：属于各源自身认证，`sources/xiaohongshu/auth.py` 管理小红书 Cookie
- **Video download**：yt-dlp 子进程，在 textifier.py 中完成下载 → 提音频 → 转写
- **Image selection**：仅发送最多 3 张图片给 LLM（`--images`/`--no-images` 控制）
- **Fallback chain for scraping**：`__NEXT_DATA__` → DOM selectors → API response interception
- **CLI uses typer** with rich for terminal output

## Rust

Rust rewrite (functionally complete for `extract` command). The workspace mirrors the Python architecture one-to-one. Python bridge scripts handle Playwright scraping and faster-whisper transcription.

```bash
# Build all crates
cargo build --workspace

# Run (drop-in replacement for Python xhs-recipe)
cargo run -- extract <xhs-url>
cargo run -- extract <xhs-url> --output recipe.md
cargo run -- extract <xhs-url> --no-images
cargo run -- setup
cargo run -- login [--headless]
cargo run -- logout

# Test
cargo test --workspace

# Run all 44 tests
cargo test --workspace

# Run specific crate tests
cargo test -p core         # 3 tests (models)
cargo test -p analyzer     # 22 tests (LLM parsing, images, fallback)
cargo test -p presentation # 6 tests (terminal render, golden file save)
cargo test -p sources      # 3 tests (URL routing)
cargo test -p textifier    # 1 test (text assembly)
cargo test -p pipeline     # 2 tests (orchestration)
cargo test -p cli          # 7 tests (argument parsing)

# Run with real URL (requires network + API key + cookies)
cargo run -- extract <xhs-url>
cargo run -- extract <xhs-url> --output recipe.md
```

### Architecture

```
cargo run -- extract <url>
  → cli/src/main.rs          # Clap CLI
  → pipeline/src/lib.rs      # fetch → textify → analyze
    → sources/src/base.rs    # Python bridge: python3 scripts/fetch_raw.py
    → textifier/src/lib.rs   # yt-dlp + ffmpeg + Python bridge: scripts/transcribe.py
    → analyzer/src/lib.rs    # reqwest → DeepSeek API (function calling)
  → presentation/src/        # Terminal render + .md/.json save
```

### Rust Crate Layout

```
crates/
├── core/            # Data models (serde)
├── presentation/    # Terminal output + file save
├── analyzer/        # LLM function calling (reqwest)
├── textifier/       # Video download + transcription
├── sources/         # Source adapters (multi-platform)
├── pipeline/        # Orchestration
└── cli/             # Binary (clap)
```
