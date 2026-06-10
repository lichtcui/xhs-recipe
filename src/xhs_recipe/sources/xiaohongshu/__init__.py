"""小红书 Source Adapter。

提供从小红书笔记链接提取 RawContent 的能力。
"""

from ...models import RawContent


async def fetch(url: str) -> RawContent:
    """从小红书 URL 提取平台无关的原始内容。"""
    from .url import resolve_short_url, extract_note_id
    from .scraper import scrape

    url = await resolve_short_url(url)
    note_id = extract_note_id(url)
    if not note_id:
        raise ValueError(f"无法从 URL 中提取笔记 ID: {url}")

    return await scrape(url, note_id)
