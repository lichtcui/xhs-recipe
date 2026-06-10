"""小红书 URL 检测、短链接解析、笔记 ID 提取。"""

import re
from urllib.parse import urlparse

from playwright.async_api import async_playwright

XHS_URL_PATTERNS = [
    re.compile(r"xiaohongshu\.com/explore/[a-f0-9]+"),
    re.compile(r"xiaohongshu\.com/discovery/item/[a-f0-9]+"),
    re.compile(r"xhslink\.com/\w+"),
]


def is_xhs_url(url: str) -> bool:
    return any(p.search(url) for p in XHS_URL_PATTERNS)


async def resolve_short_url(url: str) -> str:
    """解析 xhslink 短链接为完整 URL。"""
    if "xhslink.com" in url:
        async with async_playwright() as p:
            browser = await p.chromium.launch(headless=True)
            page = await browser.new_page()
            try:
                await page.goto(url, wait_until="commit", timeout=15000)
                return page.url
            finally:
                await browser.close()
    return url


def extract_note_id(url: str) -> str | None:
    """从 URL 中提取笔记 ID。"""
    parsed = urlparse(url)
    path = parsed.path
    for pattern in [r"/explore/([a-f0-9]+)", r"/discovery/item/([a-f0-9]+)"]:
        m = re.search(pattern, path)
        if m:
            return m.group(1)
    return None
