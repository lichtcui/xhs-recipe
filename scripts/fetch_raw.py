#!/usr/bin/env python3
"""
Bridge: Rust calls this via subprocess to fetch page content as JSON.

Usage: python3 fetch_raw.py <url>

Outputs RawContent JSON on stdout (ONLY JSON, no logs).
All scraper log output goes to stderr.
Exit code 0 on success, 1 on failure.
"""
import asyncio, contextlib, json, os, sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))

from xhs_recipe.sources.xiaohongshu import fetch


async def main():
    url = sys.argv[1]
    # Redirect scraper print() output to stderr so stdout is clean JSON
    with contextlib.redirect_stdout(sys.stderr):
        raw = await fetch(url)
    print(json.dumps(raw.model_dump(), ensure_ascii=False))


try:
    asyncio.run(main())
except Exception as e:
    print(json.dumps({"error": str(e)}), file=sys.stderr)
    sys.exit(1)
