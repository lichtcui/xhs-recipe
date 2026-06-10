#!/usr/bin/env python3
"""
Bridge: Rust calls this via subprocess to perform QR login.

Usage: python3 login.py [--headless] [--timeout 120]

Exit code 0 on success, 1 on failure.
"""
import asyncio, os, sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))
from xhs_recipe.sources.xiaohongshu.auth import login


async def main():
    headless = "--headless" in sys.argv
    timeout = 120
    for i, arg in enumerate(sys.argv):
        if arg == "--timeout" and i + 1 < len(sys.argv):
            timeout = int(sys.argv[i + 1])
    success = await login(headless=headless, timeout=timeout)
    sys.exit(0 if success else 1)


asyncio.run(main())
