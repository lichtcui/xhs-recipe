"""CLI 入口模块。

仅定义 CLI 命令，所有逻辑委托给 pipeline / presentation / sources。
"""

import asyncio
import traceback
from pathlib import Path
from typing import Optional

import typer
from dotenv import load_dotenv
from rich.console import Console

from . import pipeline
from . import presentation
from . import sources

# 加载 .env 文件
load_dotenv()

app = typer.Typer(
    name="xhs-recipe",
    help="从社交媒体链接提取菜谱的 CLI 工具",
    no_args_is_help=True,
)
console = Console()


@app.command()
def extract(
    url: str = typer.Argument(
        ...,
        help="笔记链接（支持 xhslink.com 短链接和完整链接）",
    ),
    output: Optional[Path] = typer.Option(
        None,
        "--output", "-o",
        help="输出文件路径（支持 .md 和 .json 后缀）",
        exists=False,
    ),
    model: str = typer.Option(
        "deepseek-chat",
        "--model", "-m",
        help="模型名称（默认 deepseek-chat）",
    ),
    whisper_model: str = typer.Option(
        "medium",
        "--whisper-model",
        help="Whisper 模型大小 (tiny/base/small/medium/large-v3)",
    ),
    images: bool = typer.Option(
        True,
        "--images/--no-images",
        help="是否发送图片给 AI 分析",
    ),
):
    """从社交媒体链接提取菜谱。"""
    if not sources.supports_url(url):
        console.print("[red]错误: 请提供支持的链接（当前支持小红书）[/red]")
        raise typer.Exit(1)

    console.print(f"\n[bold]🔍 正在处理:[/bold] {url}")

    try:
        recipe = asyncio.run(pipeline.extract(
            url=url,
            whisper_model=whisper_model,
            model=model,
            send_images=images,
        ))
    except PermissionError as e:
        console.print(f"[yellow]⛔ {e}[/yellow]")
        console.print("\n[yellow]提示: 小红书自动获取 Cookie 失败[/yellow]")
        from .sources.xiaohongshu.auth import get_saved_cookies
        if get_saved_cookies():
            console.print("  已保存的 Cookie 可能已过期，请清除后重试：")
            console.print("  [bold]xhs-recipe logout[/bold]")
        console.print("  或者尝试手动扫码登录获取新的 Cookie：")
        console.print("  [bold]xhs-recipe login[/bold]")
        raise typer.Exit(1)
    except ValueError as e:
        console.print(f"[red]{e}[/red]")
        console.print("请设置 DEEPSEEK_API_KEY 环境变量或在 .env 文件中配置")
        raise typer.Exit(1)
    except Exception as e:
        console.print(f"[red]处理失败: {e}[/red]")
        if "DEEPSEEK_API_KEY" in str(e):
            console.print("请设置 DEEPSEEK_API_KEY 环境变量或在 .env 文件中配置")
        else:
            console.print("\n[dim]详细错误信息:[/dim]")
            traceback.print_exc()
        raise typer.Exit(1)

    presentation.render_and_save(recipe, output)


@app.command()
def setup():
    """初始化项目环境。"""
    import shutil

    console.print("[bold]📦 检查系统依赖...[/bold]")

    ffmpeg_path = shutil.which("ffmpeg")
    if ffmpeg_path:
        console.print("  [green]✓[/green] ffmpeg 已安装")
    else:
        console.print("  [red]✗[/red] ffmpeg 未安装")
        console.print("    macOS: brew install ffmpeg")

    yt_dlp_path = shutil.which("yt-dlp")
    if yt_dlp_path:
        console.print("  [green]✓[/green] yt-dlp 已安装")
    else:
        console.print("  [red]✗[/red] yt-dlp 未安装（pip install yt-dlp）")

    console.print()
    console.print("[bold]📦 安装 Playwright 浏览器...[/bold]")
    console.print("  运行: playwright install chromium")
    console.print()
    console.print("[bold]🔑 配置 API Key[/bold]")
    console.print("  将 DEEPSEEK_API_KEY 添加到 .env 文件")
    console.print("  或存入 macOS 钥匙串: security add-generic-password -a \"$USER\" -s deepseek-api -w \"sk-...\"")
    console.print()
    console.print("完成！运行 [bold]xhs-recipe extract <链接>[/bold] 开始使用")


@app.command()
def login(
    headless: bool = typer.Option(
        False,
        "--headless",
        help="无头模式（不显示浏览器窗口，二维码保存为图片文件）",
    ),
    timeout: int = typer.Option(
        120,
        "--timeout", "-t",
        help="等待扫码超时时间（秒）",
    ),
):
    """📱 扫码登录小红书，保存 Cookie 供后续使用。"""
    from .sources.xiaohongshu.auth import login as xhs_login

    console.print("[bold]📱 小红书登录[/bold]")
    console.print("即将打开浏览器窗口...")
    console.print("请用小红书 App 扫描二维码完成登录\n")

    success = asyncio.run(xhs_login(headless=headless, timeout=timeout))
    if success:
        console.print("\n现在可以运行 [bold]xhs-recipe extract[/bold] 来提取菜谱了！")
    else:
        raise typer.Exit(1)


@app.command()
def logout():
    """清除已保存的小红书登录 Cookie。"""
    from .sources.xiaohongshu.auth import logout as xhs_logout
    asyncio.run(xhs_logout())


if __name__ == "__main__":
    app()
