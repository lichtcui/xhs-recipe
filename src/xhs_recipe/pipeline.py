"""流程编排层。

串联 Source Adapter → Textifier → Analyzer 三个步骤。
"""

from typing import Optional

from rich.console import Console

from . import sources
from . import textifier
from .analyzers import extract_recipe
from .models import Recipe

console = Console()


async def extract(
    url: str,
    whisper_model: str = "medium",
    model: str = "deepseek-chat",
    send_images: bool = True,
    api_key: Optional[str] = None,
) -> Recipe:
    """完整提取流程：抓取 → 转文字 → AI 分析。"""
    # Step 1: 抓取
    console.print("  ↓ 抓取页面内容...")
    raw = await sources.fetch(url)
    console.print(f"  ✓ 标题: {raw.title}")
    console.print(f"  ✓ 类型: {'视频笔记' if raw.has_video else '图文笔记'}")
    console.print(f"  ✓ 图片: {len(raw.image_urls)} 张")

    # Step 2: 转文字
    text = await textifier.process(raw, whisper_model=whisper_model)

    # Step 3: AI 分析
    recipe = await extract_recipe(
        text=text.full_text,
        title=text.title,
        image_urls=raw.image_urls if send_images else [],
        model=model,
        api_key=api_key,
    )
    recipe.source_url = raw.source_url

    return recipe
