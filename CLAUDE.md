# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

CLI tool (Python) to extract structured Chinese recipes from 小红书 (Xiaohongshu / RedNote) post URLs. It fetches note content via Playwright (headless Chromium), transcribes video audio with yt-dlp + faster-whisper, and extracts structured recipe data via LLM API.

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
