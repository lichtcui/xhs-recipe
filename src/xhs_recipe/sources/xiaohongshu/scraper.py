"""小红书页面抓取器。

使用 Playwright 加载页面，从 __NEXT_DATA__ / DOM / API 响应中提取内容。
"""

import asyncio
import json
import os
import traceback
from typing import Optional

from playwright.async_api import async_playwright, Page, BrowserContext

from ...models import RawContent
from .auth import get_saved_cookies, cookie_to_header, fetch_cookies_from_login, COOKIE_DIR, COOKIE_FILE


async def scrape(url: str, note_id: str) -> RawContent:
    """抓取小红书笔记内容。"""
    # Cookie 获取
    cookie = os.getenv("XHS_COOKIE")
    if not cookie:
        saved = get_saved_cookies()
        if saved:
            cookie = cookie_to_header(saved)
            print("  ✓ 使用已保存的 Cookie")
    if not cookie:
        print("  ↓ 尝试自动扫码登录获取 Cookie...")
        cookie = await fetch_cookies_from_login()
        if cookie:
            print("  ✓ 已获取 Cookie")

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

        note_api_data: dict = {}

        async def intercept_response(response):
            if "/api/sns/web/v1/feed" in response.url:
                try:
                    data = await response.json()
                    note_api_data["feed"] = data
                    if "items" in data.get("data", {}):
                        print("  ✓ 拦截到 API 响应数据")
                except Exception as e:
                    print(f"  [dim]API 响应解析失败: {e}[/dim]")

        page.on("response", intercept_response)

        print("  ↓ 加载页面...")
        try:
            await page.goto(url, wait_until="domcontentloaded", timeout=30000)
            print("  ✓ 页面 DOM 加载完成")
        except Exception as e:
            await browser.close()
            raise RuntimeError(f"页面加载失败: {e}")

        try:
            await page.wait_for_load_state("networkidle", timeout=10000)
            print("  ✓ 页面网络空闲")
        except Exception:
            print("  ⚠ 页面网络未完全空闲（超时 10s），继续处理...")
        await asyncio.sleep(2)

        title, description, images, has_video = await _extract_page_data(page, note_api_data)

        # 登录页检测 → 自动获取 Cookie 重试
        if not title or title in ("手机号登录", "登录", "小红书"):
            print("  ⚠ 检测到登录页，尝试获取 Cookie 后重试...")
            if not cookie:
                new_cookies = await context.cookies()
                if new_cookies:
                    COOKIE_DIR.mkdir(parents=True, exist_ok=True)
                    COOKIE_FILE.write_text(
                        json.dumps(
                            [
                                {"name": c["name"], "value": c["value"],
                                 "domain": c["domain"], "path": c["path"]}
                                for c in new_cookies
                            ],
                            ensure_ascii=False,
                            indent=2,
                        )
                    )
                    print("  ✓ 已保存新 Cookie")

                note_api_data.clear()
                try:
                    await page.goto(url, wait_until="domcontentloaded", timeout=30000)
                    await page.wait_for_load_state("networkidle", timeout=10000)
                except Exception as e:
                    print(f"  ⚠ 重试加载失败: {e}")
                await asyncio.sleep(2)

                title, description, images, has_video = await _extract_page_data(page, note_api_data)

        await browser.close()

        if not title or title in ("手机号登录", "登录", "小红书"):
            raise PermissionError("需要登录才能查看内容。")

        return RawContent(
            title=title,
            text_content=description,
            image_urls=images,
            has_video=has_video,
            source="xiaohongshu",
            source_url=url,
        )


async def _extract_page_data(page: Page, note_api_data: dict):
    """从已加载的页面中提取标题、描述、图片列表和视频标志。"""
    state = await _extract_from_initial_state(page)
    parsed = _parse_note_from_state(state) if state else None

    if parsed:
        title = parsed.get("title", "")
        description = parsed.get("description", "")
        images = parsed.get("images", [])
        has_video = parsed.get("has_video", False)
        print("  ✓ 从 __NEXT_DATA__ 中提取数据")
    else:
        print("  ↓ __NEXT_DATA__ 未找到，尝试 DOM 提取...")
        dom_data = await _extract_from_dom(page)
        title = dom_data.get("title", "")
        description = dom_data.get("description", "")
        images = dom_data.get("images", [])
        has_video = dom_data.get("has_video", False)
        if title:
            print("  ✓ 从 DOM 中提取数据")
        else:
            print("  ⚠ DOM 提取未找到标题")

    # 从 API 响应补充
    if (not title or not description) and "feed" in note_api_data:
        print("  ↓ 尝试从 API 响应补充...")
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
                print("  ✓ API 响应补充完成")
            else:
                print("  ⚠ API 响应无 items")
        except Exception as e:
            print(f"  ⚠ API 响应解析异常: {e}")

    return title, description, images, has_video


async def _extract_from_initial_state(page: Page) -> Optional[dict]:
    """从 window.__INITIAL_STATE__ 提取结构化数据。"""
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
    except Exception as e:
        print(f"  ⚠ 提取 __INITIAL_STATE__ 异常: {e}")
        return None


def _parse_note_from_state(state: dict) -> Optional[dict]:
    """从 __INITIAL_STATE__ 中解析笔记内容。"""
    try:
        if "note" in state:
            note = state["note"]
            return {
                "title": note.get("title", ""),
                "description": note.get("desc", ""),
                "images": [img.get("url", "") for img in note.get("imageList", [])],
                "has_video": note.get("type") == "video",
            }

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


async def _extract_from_dom(page: Page) -> dict:
    """从 DOM 中提取笔记内容。"""
    result = {"title": "", "description": "", "images": [], "has_video": False}

    selectors = {
        "title": [
            "#detail-title", ".title", "h1.title", "[class*='title']",
            "meta[property='og:title']",
        ],
        "description": [
            "#detail-desc", ".desc", ".description", "[class*='desc']",
            "meta[property='og:description']",
        ],
    }

    for key, tag in [("title", "og:title"), ("description", "og:description")]:
        meta = await page.query_selector(f'meta[property="{tag}"]')
        if meta:
            content = await meta.get_attribute("content")
            if content:
                result[key] = content.strip()

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

    image_selectors = [
        ".swiper-slide img", ".carousel img", "[class*='carousel'] img",
        ".note-image img", "img[data-src]",
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

    video_el = await page.query_selector("video")
    if video_el:
        result["has_video"] = True

    return result
