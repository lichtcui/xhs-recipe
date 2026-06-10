from ..models import RawContent

SUPPORTED_DOMAINS = [
    "xiaohongshu.com",
    "xhslink.com",
]


async def fetch(url: str) -> RawContent:
    """根据 URL 自动路由到对应的 Source Adapter 并返回平台无关的 RawContent。"""
    if "xiaohongshu.com" in url or "xhslink.com" in url:
        from .xiaohongshu import fetch as xhs_fetch
        return await xhs_fetch(url)
    raise ValueError(f"不支持的内容来源: {url}")


def supports_url(url: str) -> bool:
    return any(domain in url for domain in SUPPORTED_DOMAINS)
