from pydantic import BaseModel
from typing import Optional


class XHSContent(BaseModel):
    """从小红书页面提取的原始内容。"""
    title: str
    description: str
    images: list[str] = []
    video_url: Optional[str] = None
    note_type: str = "image"  # "image" or "video"


class Ingredient(BaseModel):
    name: str
    amount: Optional[str] = None
    prep: Optional[str] = None
    category: Optional[str] = None  # "食材" or "调料"


class Step(BaseModel):
    title: str  # e.g. "清洗", "腌制", "烤制"
    time: Optional[str] = None  # e.g. "约3分钟", "1小时"
    content: str  # detailed instructions


class Recipe(BaseModel):
    """最终输出的结构化菜谱。"""
    name: str
    total_time: Optional[str] = None  # e.g. "1小时25分钟"
    ingredients: list[Ingredient] = []  # 食材
    seasonings: list[Ingredient] = []  # 调料
    equipment: list[str] = []  # 器具
    steps: list[Step] = []
    tips: list[str] = []
    source_url: str
    is_food: bool = True
    reason: Optional[str] = None
