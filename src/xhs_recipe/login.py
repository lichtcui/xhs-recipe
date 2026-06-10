"""小红书扫码登录模块。

通过 Playwright 打开小红书登录页面，用户使用小红书 App 扫码完成登录，
自动保存 Cookie 供后续抓取使用。
"""

import asyncio
import json
import sys
import time
from pathlib import Path
from typing import Optional

from playwright.async_api import async_playwright, TimeoutError

# Cookie 存储路径
COOKIE_DIR = Path.home() / ".cache" / "xhs-recipe"
COOKIE_FILE = COOKIE_DIR / "cookies.json"


def get_saved_cookies() -> Optional[list[dict]]:
    """读取已保存的 Cookie。"""
    if COOKIE_FILE.exists():
        try:
            return json.loads(COOKIE_FILE.read_text())
        except (json.JSONDecodeError, OSError):
            return None
    return None


def cookie_to_header(cookies: list[dict]) -> str:
    """将 cookie 列表转为请求头字符串。"""
    return "; ".join(f"{c['name']}={c['value']}" for c in cookies)


async def login(
    headless: bool = False,
    timeout: int = 120,
) -> bool:
    """扫码登录小红书。

    流程：
    1. 打开浏览器进入小红书登录页
    2. 等待用户使用小红书 App 扫描二维码
    3. 登录成功后自动保存 Cookie

    Args:
        headless: 是否使用无头模式（默认 False，显示浏览器窗口）
        timeout: 等待登录超时时间（秒）

    Returns:
        是否登录成功
    """
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

        # 等待页面渲染二维码
        await page.wait_for_timeout(2000)

        # 尝试多种选择器定位二维码图片
        qr_selectors = [
            "img[class*='qrcode']",
            "img[alt*='QR']",
            ".login-qrcode img",
            "[class*='qrcode'] img",
            "canvas",  # 有些二维码是用 canvas 画的
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
            # 保存二维码截图
            COOKIE_DIR.mkdir(parents=True, exist_ok=True)
            qr_path = COOKIE_DIR / "login_qr.png"
            await qr_element.screenshot(path=str(qr_path))
            qr_saved_path = qr_path
        elif headless:
            # 如果找不到二维码元素，直接截全屏
            COOKIE_DIR.mkdir(parents=True, exist_ok=True)
            qr_path = COOKIE_DIR / "login_qr.png"
            await page.screenshot(path=str(qr_path), full_page=True)
            qr_saved_path = qr_path

        # 显示引导信息
        _print_login_instructions(qr_saved_path, headless)

        # 等待登录成功（页面跳转或出现用户元素）
        login_success = False
        start_time = time.time()
        check_interval = 2

        while time.time() - start_time < timeout:
            current_url = page.url

            # 检查是否已登录（URL 不再是 /login）
            if "/login" not in current_url and "/explore" in current_url:
                login_success = True
                break

            # 检查是否存在用户头像（已登录标志）
            try:
                avatar = await page.query_selector(
                    "[class*='avatar'], [class*='user'], [class*='User']"
                )
                if avatar:
                    login_success = True
                    break
            except Exception:
                pass

            await asyncio.sleep(check_interval)

            # 倒计时输出
            elapsed = int(time.time() - start_time)
            remaining = timeout - elapsed
            if remaining % 10 == 0:  # 每 10 秒提醒一次
                print(f"  ⏳ 等待扫码... 还剩 {remaining} 秒")

        if login_success:
            # 保存 Cookie
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


def _print_login_instructions(qr_path: Optional[Path], headless: bool):
    """打印登录引导信息。"""
    print("\n" + "=" * 50)
    print("📱 小红书扫码登录")
    print("=" * 50)

    if qr_path and qr_path.exists():
        print(f"\n二维码已保存到: {qr_path}")
        print("请用「小红书 App」扫描该二维码登录")
        print("   打开小红书 App → 点击首页左上角扫码图标 → 扫描二维码")
    else:
        print("\n请在浏览器窗口中完成登录")

    if headless:
        print("\n提示: 如果看不到二维码，可以尝试不加 --headless 参数")
        print("   xhs-recipe login")

    print("\n等待扫码中...")


async def logout():
    """清除已保存的 Cookie。"""
    if COOKIE_FILE.exists():
        COOKIE_FILE.unlink()
        print(f"✅ Cookie 已清除")
    else:
        print("ℹ️  没有已保存的 Cookie")
