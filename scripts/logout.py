#!/usr/bin/env python3
"""
Bridge: Rust calls this via subprocess to clear saved cookies.

Usage: python3 logout.py
"""
import asyncio, os, sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))
from xhs_recipe.sources.xiaohongshu.auth import logout


async def main():
    await logout()


asyncio.run(main())
