from pydantic import BaseModel
from typing import Optional


class RawContent(BaseModel):
    """平台无关的原始内容，由各个 Source Adapter 返回。"""
    title: str
    text_content: str          # 正文 / 描述文字
    image_urls: list[str] = []
    has_video: bool = False
    video_url: Optional[str] = None
    source: str                # "xiaohongshu"
    source_url: str


class TextContent(BaseModel):
    """所有媒体统一转为文字后的结果。"""
    full_text: str             # 标题 + 描述 + 视频转写 + 图片描述
    title: str
    source: str
    source_url: str


class Ingredient(BaseModel):
    name: str
    amount: Optional[str] = None
    prep: Optional[str] = None
    category: Optional[str] = None  # "食材" or "调料"


class Step(BaseModel):
    title: str
    time: Optional[str] = None
    content: str


class Recipe(BaseModel):
    """最终输出的结构化菜谱。"""
    name: str
    total_time: Optional[str] = None
    ingredients: list[Ingredient] = []
    seasonings: list[Ingredient] = []
    equipment: list[str] = []
    steps: list[Step] = []
    tips: list[str] = []
    source_url: str
    is_food: bool = True
    reason: Optional[str] = None
