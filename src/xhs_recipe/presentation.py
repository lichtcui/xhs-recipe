"""输出渲染与文件保存。"""

import json
from pathlib import Path
from typing import Optional

from rich.console import Console
from rich.panel import Panel
from rich.table import Table

from .models import Recipe


def render_terminal(recipe: Recipe, console: Optional[Console] = None):
    """终端渲染菜谱。"""
    if console is None:
        console = Console()

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
        items = [
            f"{s.name}{' ' + s.amount if s.amount else ''}{'（' + s.prep + '）' if s.prep else ''}"
            for s in recipe.seasonings
        ]
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


def save_to_file(recipe: Recipe, output_path: Path):
    """保存菜谱到文件（根据后缀自动选择格式）。"""
    fmt = "json" if output_path.suffix == ".json" else "md"

    if fmt == "json":
        data = recipe.model_dump(exclude_none=True)
        output_path.write_text(
            json.dumps(data, ensure_ascii=False, indent=2),
            encoding="utf-8",
        )
    else:
        lines = []
        lines.append(f"# {recipe.name}")
        lines.append("")

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
                lines.append("")
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

    Console().print(f"\n[green]✓ 已保存到 {output_path}[/green]")


def render_and_save(recipe: Recipe, output_path: Optional[Path] = None):
    """终端显示 + 可选保存。"""
    render_terminal(recipe)
    if output_path:
        save_to_file(recipe, output_path)
