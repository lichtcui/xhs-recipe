"""小红书内容抓取器。

使用 Playwright 加载小红书笔记页面，提取文字、图片和视频信息。
支持图文笔记和视频笔记两种类型。
"""

import asyncio
import json
import os
import re
from typing import Optional
from urllib.parse import urlparse

from playwright.async_api import async_playwright, Page, BrowserContext

from .models import XHSContent
from .login import get_saved_cookies, cookie_to_header

# 小红书笔记 URL 模式
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
                resolved = page.url
                return resolved
            finally:
                await browser.close()
    return url


def _extract_note_id(url: str) -> Optional[str]:
    """从 URL 中提取笔记 ID。"""
    parsed = urlparse(url)
    path = parsed.path
    # /explore/xxxxxxxxx 或 /discovery/item/xxxxxxxxx
    for pattern in [r"/explore/([a-f0-9]+)", r"/discovery/item/([a-f0-9]+)"]:
        m = re.search(pattern, path)
        if m:
            return m.group(1)
    return None


async def extract_from_dom(page: Page) -> dict:
    """从 DOM 中提取笔记内容。"""
    result = {
        "title": "",
        "description": "",
        "images": [],
        "has_video": False,
    }

    # 尝试提取标题和描述
    # XHS 页面结构和类名可能变化，尝试多种选择器
    selectors = {
        "title": [
            "#detail-title",
            ".title",
            "h1.title",
            "[class*='title']",
            "meta[property='og:title']",
        ],
        "description": [
            "#detail-desc",
            ".desc",
            ".description",
            "[class*='desc']",
            "meta[property='og:description']",
        ],
    }

    # 从 meta 标签提取
    for key, tag in [("title", "og:title"), ("description", "og:description")]:
        meta = await page.query_selector(f'meta[property="{tag}"]')
        if meta:
            content = await meta.get_attribute("content")
            if content:
                result[key] = content.strip()

    # 从 DOM 元素提取
    for key, sel_list in selectors.items():
        if not result[key]:
            for sel in sel_list:
                try:
                    el = await page.query_selector(sel)
                    if el:
                        text = await el.inner_text()
                        if text:
                            result[key] = text.strip()
                            break
                except Exception:
                    continue

    # 提取图片
    image_selectors = [
        ".swiper-slide img",
        ".carousel img",
        "[class*='carousel'] img",
        ".note-image img",
        "img[data-src]",
    ]
    seen_urls = set()
    for sel in image_selectors:
        try:
            imgs = await page.query_selector_all(sel)
            for img in imgs:
                src = await img.get_attribute("src") or await img.get_attribute("data-src") or ""
                if src and "http" in src and src not in seen_urls:
                    result["images"].append(src)
                    seen_urls.add(src)
        except Exception:
            continue

    # 检查是否有视频
    video_el = await page.query_selector("video")
    if video_el:
        result["has_video"] = True

    return result


async def extract_from_initial_state(page: Page) -> Optional[dict]:
    """从 window.__INITIAL_STATE__ 提取结构化数据（最可靠）。"""
    try:
        state = await page.evaluate("""() => {
            try {
                return JSON.parse(document.getElementById('__NEXT_DATA__').textContent);
            } catch(e) {
                return null;
            }
        }""")
        if state:
            return state

        state = await page.evaluate("""() => {
            try {
                return window.__INITIAL_STATE__;
            } catch(e) {
                return null;
            }
        }""")
        return state
    except Exception:
        return None


def _parse_note_from_state(state: dict) -> Optional[dict]:
    """从 __INITIAL_STATE__ 中解析笔记内容。"""
    try:
        # XHS Next.js state 结构
        if "note" in state:
            note = state["note"]
            return {
                "title": note.get("title", ""),
                "description": note.get("desc", ""),
                "images": [img.get("url", "") for img in note.get("imageList", [])],
                "has_video": note.get("type") == "video",
            }

        # 尝试其他可能的路径
        for key in ["noteDetail", "noteData", "currentNote"]:
            if key in state:
                note = state[key]
                return {
                    "title": note.get("title", ""),
                    "description": note.get("desc", ""),
                    "images": [
                        img.get("url", "") or img.get("infoList", [{}])[-1].get("url", "")
                        for img in note.get("imageList", [])
                    ],
                    "has_video": note.get("type") == "video" or "video" in note,
                }
    except Exception:
        return None
    return None


async def _extract_page_data(
    page: Page,
    note_api_data: dict,
) -> tuple[str, str, list[str], bool]:
    """从已加载的页面中提取标题、描述、图片列表和视频标志。"""
    state = await extract_from_initial_state(page)
    parsed = _parse_note_from_state(state) if state else None

    if parsed:
        title = parsed.get("title", "")
        description = parsed.get("description", "")
        images = parsed.get("images", [])
        has_video = parsed.get("has_video", False)
    else:
        dom_data = await extract_from_dom(page)
        title = dom_data.get("title", "")
        description = dom_data.get("description", "")
        images = dom_data.get("images", [])
        has_video = dom_data.get("has_video", False)

    # 从 API 响应中补充
    if (not title or not description) and "feed" in note_api_data:
        try:
            items = note_api_data["feed"].get("data", {}).get("items", [])
            if items:
                note_card = items[0].get("note_card", {})
                if not title:
                    title = note_card.get("title", "")
                if not description:
                    description = note_card.get("desc", "")
                if not images:
                    img_list = note_card.get("image_list", [])
                    images = [
                        img.get("url", "") or img.get("info_list", [{}])[-1].get("url", "")
                        for img in img_list
                    ]
                if note_card.get("type") == "video":
                    has_video = True
        except Exception:
            pass

    return title, description, images, has_video


async def fetch(
    url: str,
    cookie: Optional[str] = None,
    timeout: int = 30000,
) -> XHSContent:
    """抓取小红书笔记内容。

    Args:
        url: 小红书笔记链接（支持短链接和完整链接）
        cookie: 可选的小红书 Cookie 字符串（登录态）。
                未提供时自动尝试加载已保存的 Cookie（来自 xhs-recipe login）
        timeout: 页面加载超时时间（毫秒）

    Returns:
        XHSContent: 包含标题、描述、图片列表和视频信息的结构化数据
    """
    url = await resolve_short_url(url)
    note_id = _extract_note_id(url)
    if not note_id:
        raise ValueError(f"无法从 URL 中提取笔记 ID: {url}")

    # 自动加载已保存的 Cookie
    if not cookie:
        cookie = os.getenv("XHS_COOKIE")
    if not cookie:
        saved = get_saved_cookies()
        if saved:
            cookie = cookie_to_header(saved)

    async with async_playwright() as p:
        browser = await p.chromium.launch(
            headless=True,
            args=[
                "--disable-blink-features=AutomationControlled",
                "--no-sandbox",
            ],
        )
        context: BrowserContext = await browser.new_context(
            user_agent=(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
                "AppleWebKit/537.36 (KHTML, like Gecko) "
                "Chrome/125.0.0.0 Safari/537.36"
            ),
            viewport={"width": 1920, "height": 1080},
            locale="zh-CN",
        )

        if cookie:
            for c in cookie.split(";"):
                c = c.strip()
                if "=" in c:
                    name, value = c.split("=", 1)
                    await context.add_cookies([{
                        "name": name,
                        "value": value,
                        "domain": ".xiaohongshu.com",
                        "path": "/",
                    }])

        page = await context.new_page()

        # 拦截 XHR 请求获取笔记数据
        note_api_data = {}

        async def intercept_response(response):
            url_path = response.url
            if "/api/sns/web/v1/feed" in url_path:
                try:
                    body = await response.json()
                    note_api_data["feed"] = body
                except Exception:
                    pass

        page.on("response", intercept_response)

        try:
            await page.goto(url, wait_until="domcontentloaded", timeout=timeout)
        except Exception as e:
            await browser.close()
            raise RuntimeError(f"页面加载失败: {e}")

        # 等待内容渲染
        try:
            await page.wait_for_load_state("networkidle", timeout=10000)
        except Exception:
            pass
        await asyncio.sleep(2)

        # 提取页面数据
        title, description, images, has_video = await _extract_page_data(page, note_api_data)

        # 检测是否被拦截到登录页 — 自动获取 Cookie 重试
        if not title or title in ("手机号登录", "登录", "小红书"):
            if not cookie:  # 首次没有 Cookie，尝试自动获取
                # 保存从登录页获得的 Cookie（服务器会自动 Set-Cookie）
                new_cookies = await context.cookies()
                if new_cookies:
                    from .login import COOKIE_DIR, COOKIE_FILE

                    COOKIE_DIR.mkdir(parents=True, exist_ok=True)
                    COOKIE_FILE.write_text(
                        json.dumps(
                            [
                                {
                                    "name": c["name"],
                                    "value": c["value"],
                                    "domain": c["domain"],
                                    "path": c["path"],
                                }
                                for c in new_cookies
                            ],
                            ensure_ascii=False,
                            indent=2,
                        )
                    )

                # 重新加载页面（浏览器上下文已有 Cookie）
                note_api_data.clear()
                try:
                    await page.goto(url, wait_until="domcontentloaded", timeout=timeout)
                    await page.wait_for_load_state("networkidle", timeout=10000)
                except Exception:
                    pass
                await asyncio.sleep(2)

                # 重新提取
                title, description, images, has_video = await _extract_page_data(
                    page, note_api_data
                )

        await browser.close()

        # 最终检查
        if not title or title in ("手机号登录", "登录", "小红书"):
            raise PermissionError(
                "需要登录才能查看内容。"
            )

        return XHSContent(
            title=title,
            description=description,
            images=images,
            video_url=None,  # 视频下载 URL 由 yt-dlp 处理
            note_type="video" if has_video else "image",
        )
