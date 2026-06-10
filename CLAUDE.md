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

```
src/
├── xhs_recipe/
│   ├── __init__.py
│   ├── __main__.py           # python -m xhs_recipe entry
│   ├── cli.py                # typer CLI: extract, login, logout, setup commands
│   ├── models.py             # pydantic data models: XHSContent, Recipe, Ingredient, Step
│   ├── login.py              # Playwright QR code login, cookie save/load
│   ├── xhs_fetcher.py        # Playwright-based XHS page scraper (DOM + __NEXT_DATA__)
│   ├── transcriber.py        # yt-dlp → ffmpeg → faster-whisper pipeline for video notes
│   └── recipe_extractor.py   # LLM API function-calling extraction via httpx
├── pyproject.toml
└── .env                      # API keys
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

1. **`xhs-recipe extract <url>`** → `xhs_fetcher.fetch()` opens headless Chromium, resolves short links, extracts title/description/images/video-type from `__NEXT_DATA__` or DOM selectors
2. Auto-cookie retry: if blocked by login page, saves session cookies and retries automatically
3. If video note → `transcriber.process_video()`: yt-dlp downloads video, ffmpeg extracts 16kHz mono WAV, faster-whisper transcribes to Chinese text
4. `recipe_extractor.extract_recipe()` sends text + images to LLM with function calling → parses response into `Recipe` model
5. Output rendered via rich (terminal) or saved as `.md` / `.json`

## Key Design Decisions

- **Cookie auth**: `login.py` persists cookies to `~/.cache/xhs-recipe/cookies.json`. Auto-cookie retry in `fetch()` captures session cookies on first blocked request so login is usually automatic.
- **Video download**: Uses yt-dlp subprocess for reliable XHS video extraction.
- **Image selection**: Only sends up to 3 images to LLM API (controlled by `--images`/`--no-images`) to manage cost.
- **Fallback chain for scraping**: `__NEXT_DATA__` (SSR data) → DOM selectors → API response interception.
- **CLI uses typer** with rich for terminal output.
