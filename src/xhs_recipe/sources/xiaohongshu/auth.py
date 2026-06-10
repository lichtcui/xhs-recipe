"""小红书扫码登录与 Cookie 管理。"""

import asyncio
import json
import time
from pathlib import Path
from typing import Optional

from playwright.async_api import async_playwright

COOKIE_DIR = Path.home() / ".cache" / "xhs-recipe"
COOKIE_FILE = COOKIE_DIR / "cookies.json"


def get_saved_cookies() -> Optional[list[dict]]:
    if COOKIE_FILE.exists():
        try:
            return json.loads(COOKIE_FILE.read_text())
        except (json.JSONDecodeError, OSError):
            return None
    return None


def cookie_to_header(cookies: list[dict]) -> str:
    return "; ".join(f"{c['name']}={c['value']}" for c in cookies)


async def fetch_cookies_from_login() -> Optional[str]:
    """无头访问小红书登录页，获取会话 Cookie。"""
    async with async_playwright() as p:
        browser = await p.chromium.launch(
            headless=True,
            args=["--no-sandbox"],
        )
        context = await browser.new_context(
            user_agent=(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
                "AppleWebKit/537.36 (KHTML, like Gecko) "
                "Chrome/125.0.0.0 Safari/537.36"
            ),
            viewport={"width": 1280, "height": 800},
            locale="zh-CN",
        )
        page = await context.new_page()

        try:
            await page.goto(
                "https://www.xiaohongshu.com/login",
                wait_until="networkidle",
                timeout=30000,
            )
        except Exception:
            await browser.close()
            return None

        await page.wait_for_timeout(3000)
        cookies = await context.cookies()
        await browser.close()

        if not cookies:
            return None

        COOKIE_DIR.mkdir(parents=True, exist_ok=True)
        COOKIE_FILE.write_text(
            json.dumps(
                [
                    {"name": c["name"], "value": c["value"],
                     "domain": c["domain"], "path": c["path"]}
                    for c in cookies
                ],
                ensure_ascii=False,
                indent=2,
            )
        )

        return cookie_to_header(cookies)


async def login(headless: bool = False, timeout: int = 120) -> bool:
    """扫码登录小红书，保存 Cookie。"""
    from playwright.async_api import TimeoutError

    qr_saved_path: Optional[Path] = None

    async with async_playwright() as p:
        browser = await p.chromium.launch(
            headless=headless,
            args=["--no-sandbox"],
        )
        context = await browser.new_context(
            user_agent=(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
                "AppleWebKit/537.36 (KHTML, like Gecko) "
                "Chrome/125.0.0.0 Safari/537.36"
            ),
            viewport={"width": 1280, "height": 800},
            locale="zh-CN",
        )
        page = await context.new_page()

        try:
            await page.goto(
                "https://www.xiaohongshu.com/login",
                wait_until="networkidle",
                timeout=30000,
            )
        except Exception as e:
            print(f"❌ 页面加载失败: {e}")
            await browser.close()
            return False

        await page.wait_for_timeout(2000)

        qr_selectors = [
            "img[class*='qrcode']",
            "img[alt*='QR']",
            ".login-qrcode img",
            "[class*='qrcode'] img",
            "canvas",
        ]

        qr_element = None
        for sel in qr_selectors:
            try:
                qr_element = await page.wait_for_selector(sel, timeout=3000)
                if qr_element:
                    break
            except Exception:
                continue

        if qr_element:
            COOKIE_DIR.mkdir(parents=True, exist_ok=True)
            qr_path = COOKIE_DIR / "login_qr.png"
            await qr_element.screenshot(path=str(qr_path))
            qr_saved_path = qr_path
        elif headless:
            COOKIE_DIR.mkdir(parents=True, exist_ok=True)
            qr_path = COOKIE_DIR / "login_qr.png"
            await page.screenshot(path=str(qr_path), full_page=True)
            qr_saved_path = qr_path

        _print_login_instructions(qr_saved_path, headless)

        login_success = False
        start_time = time.time()

        while time.time() - start_time < timeout:
            current_url = page.url
            if "/login" not in current_url and "/explore" in current_url:
                login_success = True
                break

            try:
                avatar = await page.query_selector(
                    "[class*='avatar'], [class*='user'], [class*='User']"
                )
                if avatar:
                    login_success = True
                    break
            except Exception:
                pass

            await asyncio.sleep(2)

            elapsed = int(time.time() - start_time)
            remaining = timeout - elapsed
            if remaining % 10 == 0:
                print(f"  ⏳ 等待扫码... 还剩 {remaining} 秒")

        if login_success:
            cookies = await context.cookies()
            COOKIE_DIR.mkdir(parents=True, exist_ok=True)
            COOKIE_FILE.write_text(
                json.dumps(
                    [
                        {"name": c["name"], "value": c["value"],
                         "domain": c["domain"], "path": c["path"]}
                        for c in cookies
                    ],
                    ensure_ascii=False,
                    indent=2,
                )
            )
            print(f"\n✅ 登录成功！Cookie 已保存到 {COOKIE_FILE}")
            print(f"   Cookie 数: {len(cookies)}")
            await browser.close()
            return True
        else:
            print(f"\n❌ 登录超时（{timeout} 秒），请重试")
            await browser.close()
            return False


async def logout():
    """清除已保存的 Cookie。"""
    if COOKIE_FILE.exists():
        COOKIE_FILE.unlink()
        print(f"✅ Cookie 已清除")
    else:
        print("ℹ️  没有已保存的 Cookie")


def _print_login_instructions(qr_path: Optional[Path], headless: bool):
    print("\n" + "=" * 50)
    print("📱 小红书扫码登录")
    print("=" * 50)

    if qr_path and qr_path.exists():
        print(f"\n二维码已保存到: {qr_path}")
        print("请用「小红书 App」扫描该二维码登录")
    else:
        print("\n请在浏览器窗口中完成登录")

    if headless:
        print("\n提示: 如果看不到二维码，可以尝试不加 --headless 参数")
        print("   xhs-recipe login")

    print("\n等待扫码中...")
