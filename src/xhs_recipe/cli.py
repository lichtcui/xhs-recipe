"""CLI 入口模块。

用法:
    xhs-recipe extract <小红书链接>
    xhs-recipe extract <链接> --output recipe.md --model claude-sonnet-4-20250514
"""

import asyncio
import json
from pathlib import Path
from typing import Optional

import typer
from dotenv import load_dotenv
from rich.console import Console
from rich.markdown import Markdown
from rich.panel import Panel
from rich.table import Table

from . import xhs_fetcher
from . import transcriber
from . import login as xhs_login
from .recipe_extractor import extract_recipe
from .models import Recipe

app = typer.Typer(
    name="xhs-recipe",
    help="从小红书链接提取菜谱的 CLI 工具",
    no_args_is_help=True,
)
console = Console()

# 加载 .env 文件
load_dotenv()


def _has_login_cookie() -> bool:
    """检查是否有已保存的登录 Cookie。"""
    return xhs_login.get_saved_cookies() is not None


def _render_recipe(recipe: Recipe):
    """极简分享风：紧凑、适合终端阅读和截图分享。"""
    if not recipe.is_food:
        console.print(Panel(
            f"[yellow]⚠ 此内容与美食无关[/yellow]\n{recipe.reason or ''}",
            title="未找到菜谱",
        ))
        return

    console.print()
    console.print(f"  🍖 [bold green]{recipe.name}[/bold green]")
    time_parts = []
    if recipe.total_time:
        time_parts.append(f"⏱ [yellow]{recipe.total_time}[/yellow]")
    time_parts.append("👨‍👩‍👧‍👦 约2-3人份")
    console.print(f"  {' ｜ '.join(time_parts)}")

    if recipe.ingredients:
        console.print(f"\n  [bold]🥩 食材[/bold]")
        for ing in recipe.ingredients:
            parts = [f"· [cyan]{ing.name}[/cyan]{' ' + ing.amount if ing.amount else ''}"]
            if ing.prep:
                parts.append(f"（{ing.prep}）")
            console.print(f"    {' '.join(parts)}")

    if recipe.seasonings:
        console.print(f"  [bold]🧂 调料[/bold]")
        items = [f"{s.name}{' ' + s.amount if s.amount else ''}{'（' + s.prep + '）' if s.prep else ''}" for s in recipe.seasonings]
        console.print(f"    · {'、'.join(items)}")

    if recipe.equipment:
        console.print(f"  [bold]🔧 器具[/bold]")
        console.print(f"    · {'、'.join(recipe.equipment)}")

    if recipe.steps:
        console.print(f"\n  [bold]📝 步骤[/bold]")
        nums = ["①", "②", "③", "④", "⑤", "⑥", "⑦", "⑧", "⑨", "⑩"]
        for i, step in enumerate(recipe.steps):
            num = nums[i] if i < len(nums) else f"{i+1}."
            time_str = f"（{step.time}）" if step.time else ""
            console.print(f"\n  [bold]{num} {step.title}[/bold] {time_str}")
            console.print(f"     {step.content}")

    if recipe.tips:
        tips_short = [tip.rstrip('。') for tip in recipe.tips]
        console.print(f"\n  [bold]💡 小贴士[/bold]")
        console.print(f"    {' · '.join(tips_short)}")


def _save_recipe(recipe: Recipe, output_path: Path, fmt: str = "md"):
    """保存菜谱到文件。"""
    if fmt == "json":
        data = recipe.model_dump(exclude_none=True)
        output_path.write_text(
            json.dumps(data, ensure_ascii=False, indent=2),
            encoding="utf-8",
        )
    else:
        lines = [
            f"# {recipe.name}",
            "",
        ]
        if recipe.total_time:
            lines.append(f"总时间：{recipe.total_time}")
            lines.append("")

        if recipe.ingredients:
            lines.append("## 食材")
            for ing in recipe.ingredients:
                parts = [ing.name]
                if ing.amount:
                    parts.append(ing.amount)
                if ing.prep:
                    parts.append(f"（{ing.prep}）")
                lines.append(f"- {' '.join(parts)}")
            lines.append("")

        if recipe.seasonings:
            lines.append("## 调料")
            for s in recipe.seasonings:
                line = s.name
                if s.amount:
                    line += f" {s.amount}"
                lines.append(f"- {line}")
            lines.append("")

        if recipe.equipment:
            lines.append(f"器具：{'、'.join(recipe.equipment)}")
            lines.append("")

        if recipe.steps:
            lines.append("## 步骤")
            nums = ["①", "②", "③", "④", "⑤", "⑥", "⑦", "⑧", "⑨", "⑩"]
            for i, step in enumerate(recipe.steps):
                num = nums[i] if i < len(nums) else f"{i+1}."
                label = step.title if step.title else f"步骤{i+1}"
                time_str = f"（{step.time}）" if step.time else ""
                lines.append(f"")
                lines.append(f"{num} {label}{time_str}")
                for line in step.content.split("\n"):
                    line = line.strip()
                    if line:
                        lines.append(f"  {line}")
            lines.append("")

        if recipe.tips:
            lines.append("## 小贴士")
            for tip in recipe.tips:
                lines.append(f"- {tip}")
            lines.append("")

        output_path.write_text("\n".join(lines), encoding="utf-8")

    console.print(f"\n[green]✓ 已保存到 {output_path}[/green]")


@app.command()
def extract(
    url: str = typer.Argument(
        ...,
        help="小红书笔记链接（支持 xhslink.com 短链接和完整链接）",
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
        help="模型名称（默认 deepseek-chat，也支持 deepseek-reasoner 等）",
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
    """从小红书链接提取菜谱。"""

    # 验证链接
    if not xhs_fetcher.is_xhs_url(url) and "xhslink.com" not in url:
        console.print("[red]错误: 请提供有效的小红书链接[/red]")
        raise typer.Exit(1)

    console.print(f"\n[bold]🔍 正在处理:[/bold] {url}")

    # Step 1: 抓取内容
    with console.status("正在抓取小红书内容..."):
        try:
            content = asyncio.run(xhs_fetcher.fetch(url))
        except PermissionError as e:
            console.print(f"[yellow]⛔ {e}[/yellow]")
            console.print("\n[yellow]提示: 小红书页面需要登录态才能查看内容[/yellow]")
            if _has_login_cookie():
                console.print("  已保存的 Cookie 可能已过期，请删除后重试：")
                console.print("  [bold]xhs-recipe logout[/bold]")
            console.print("  或者尝试手动扫码登录：")
            console.print("  [bold]xhs-recipe login[/bold]")
            raise typer.Exit(1)
        except Exception as e:
            console.print(f"[red]抓取失败: {e}[/red]")
            raise typer.Exit(1)

    console.print(f"  [green]✓[/green] 标题: {content.title}")
    console.print(f"  [green]✓[/green] 类型: {'视频笔记' if content.note_type == 'video' else '图文笔记'}")
    console.print(f"  [green]✓[/green] 图片: {len(content.images)} 张")

    # Step 2: 视频转写
    transcript = ""
    if content.note_type == "video":
        with console.status("正在处理视频（下载 → 转写）..."):
            transcript = asyncio.run(
                transcriber.process_video(content, url, whisper_model=whisper_model)
            )
        if transcript:
            console.print(f"  [green]✓[/green] 转写完成 ({len(transcript)} 字)")
        else:
            console.print("  [yellow]⚠ 视频转写未产生文字[/yellow]")

    # Step 3: 提取菜谱
    with console.status("正在用 AI 分析内容..."):
        try:
            recipe = asyncio.run(
                extract_recipe(
                    content,
                    transcript=transcript,
                    max_images=3 if images else 0,
                    model=model,
                )
            )
            recipe.source_url = url
        except ValueError as e:
            console.print(f"[red]{e}[/red]")
            console.print("请设置 DEEPSEEK_API_KEY 环境变量或在 .env 文件中配置")
            raise typer.Exit(1)
        except Exception as e:
            console.print(f"[red]AI 分析失败: {e}[/red]")
            raise typer.Exit(1)

    # 输出
    _render_recipe(recipe)

    # 保存
    if output:
        fmt = "json" if output.suffix == ".json" else "md"
        _save_recipe(recipe, output, fmt)


@app.command()
def setup():
    """初始化项目环境。"""
    console.print("[bold]📦 检查系统依赖...[/bold]")

    # 检查 ffmpeg
    import shutil
    ffmpeg_path = shutil.which("ffmpeg")
    if ffmpeg_path:
        console.print("  [green]✓[/green] ffmpeg 已安装")
    else:
        console.print("  [red]✗[/red] ffmpeg 未安装")
        console.print("    macOS: brew install ffmpeg")
        console.print("    Ubuntu: sudo apt install ffmpeg")

    # 检查 yt-dlp
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
    console.print("完成！运行 [bold]xhs-recipe extract <小红书链接>[/bold] 开始使用")


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
    """📱 扫码登录小红书，保存 Cookie 供后续使用。

    会打开浏览器窗口显示二维码，用小红书 App 扫码即可完成登录。
    """
    console.print("[bold]📱 小红书登录[/bold]")
    console.print("即将打开浏览器窗口...")
    console.print("请用小红书 App 扫描二维码完成登录\n")

    success = asyncio.run(xhs_login.login(headless=headless, timeout=timeout))

    if success:
        console.print("\n现在可以运行 [bold]xhs-recipe extract[/bold] 来提取菜谱了！")
    else:
        raise typer.Exit(1)


@app.command()
def logout():
    """清除已保存的小红书登录 Cookie。"""
    asyncio.run(xhs_login.logout())


if __name__ == "__main__":
    app()
