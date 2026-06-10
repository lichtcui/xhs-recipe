"""菜谱提取分析器。

接收纯文本（+ 可选图片），通过 LLM function calling 提取结构化菜谱。
"""

import asyncio
import base64
import json
import os
import re
import subprocess
from typing import Optional

import httpx
from openai import OpenAI

from ..models import Recipe, Ingredient, Step

# ── Tool / Prompt 定义 ──────────────────────────────────────────────

EXTRACT_TOOLS = [
    {
        "type": "function",
        "function": {
            "name": "output_recipe",
            "description": "输出从内容中提取的菜谱信息",
            "parameters": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "菜谱名称，如「蒜香椒盐烤排骨」",
                    },
                    "total_time": {
                        "type": "string",
                        "description": "总耗时，如「1小时25分钟」「30分钟」",
                    },
                    "ingredients": {
                        "type": "array",
                        "description": "主要食材（肉、蔬菜、豆制品等核心食材）",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string", "description": "食材名称"},
                                "amount": {"type": "string", "description": "用量，如「300克」「2勺」「1头」「适量」"},
                                "prep": {"type": "string", "description": "处理方式，如「切段」「剁碎」「切片」「提前泡发」"},
                            },
                            "required": ["name"],
                        },
                    },
                    "seasonings": {
                        "type": "array",
                        "description": "调味料（酱油、蚝油、盐、糖、香料等）",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string", "description": "调料名称"},
                                "amount": {"type": "string", "description": "用量"},
                            },
                            "required": ["name"],
                        },
                    },
                    "equipment": {
                        "type": "array",
                        "description": "所需器具，如「空气炸锅」「厨房纸」「烤箱」「不粘锅」",
                        "items": {"type": "string"},
                    },
                    "steps": {
                        "type": "array",
                        "description": "烹饪步骤（按顺序），每步必须有标题、耗时和详细说明",
                        "items": {
                            "type": "object",
                            "properties": {
                                "title": {"type": "string", "description": "步骤名称，如「清洗」「腌制」「烤制」"},
                                "time": {"type": "string", "description": "该步骤耗时，如「约3分钟」「1小时」「25分钟」"},
                                "content": {"type": "string", "description": "详细操作说明，包含具体用量、时间、温度、判断标准"},
                            },
                            "required": ["title", "content"],
                        },
                    },
                    "tips": {
                        "type": "array",
                        "description": "小贴士/注意事项/替换建议",
                        "items": {"type": "string"},
                    },
                    "is_food": {
                        "type": "boolean",
                        "description": "此内容是否与美食/菜谱相关",
                    },
                    "reason": {
                        "type": "string",
                        "description": "如果 is_food 为 false，解释为什么不是美食内容",
                    },
                },
                "required": ["name", "ingredients", "steps", "is_food"],
            },
        },
    }
]

SYSTEM_PROMPT = """你是专业厨师和食谱分析师。你擅长从小红书（RedNote）的美食内容中提取结构化菜谱信息。

你可以分析的内容包括：
- 笔记的文字描述（标题 + 正文）
- 视频的语音转写文本（博主的口述内容）
- 图片（菜肴成品图、步骤图）

请提取信息并严格按照 output_recipe 工具的格式输出，要求如下：

1. **菜名 (name)**：菜肴名称
2. **总时间 (total_time)**：估算总耗时
3. **食材 (ingredients)**：列出主要食材及其用量和处理方式
4. **调料 (seasonings)**：列出所有调味料
5. **器具 (equipment)**：列出所需厨具和工具
6. **步骤 (steps)**：按顺序排列，**每步必须包含**：
   - `title`: 步骤名称（如「清洗」「腌制」「烤制」）
   - `time`: 该步骤耗时
   - `content`: 详细操作说明，包含具体用量、时间、温度、判断标准
7. **小贴士 (tips)**：注意事项和替换建议

注意事项：
- 如果内容与美食/菜谱无关，设置 is_food=false 并说明原因
- 如果某些信息在内容中没有明确提及，**不要编造**
- 用量单位保持原文（如克、毫升、勺、碗等）"""


# ── API 配置 ────────────────────────────────────────────────────────


def _get_api_key() -> str:
    key = os.getenv("DEEPSEEK_API_KEY") or os.getenv("ANTHROPIC_API_KEY")
    if key:
        return key
    try:
        result = subprocess.run(
            ["security", "find-generic-password", "-a", os.environ.get("USER", ""),
             "-s", "deepseek-api", "-w"],
            capture_output=True, text=True, timeout=5,
        )
        if result.returncode == 0 and result.stdout.strip():
            return result.stdout.strip()
    except Exception:
        pass
    raise ValueError(
        "未设置 DEEPSEEK_API_KEY。请通过以下方式之一配置：\n"
        "  1. 设置环境变量：export DEEPSEEK_API_KEY=sk-...\n"
        "  2. 存入 macOS 钥匙串：security add-generic-password -a \"$USER\" "
        "-s deepseek-api -w \"sk-...\""
    )


def _get_base_url() -> str:
    return os.getenv("DEEPSEEK_BASE_URL", "https://api.deepseek.com")


# ── 图片处理 ────────────────────────────────────────────────────────


def _download_image(url: str, max_size: int = 5 * 1024 * 1024) -> Optional[bytes]:
    try:
        resp = httpx.get(url, follow_redirects=True, timeout=15)
        if resp.status_code == 200 and len(resp.content) <= max_size:
            return resp.content
    except Exception:
        return None
    return None


def _make_image_block(image_data: bytes) -> dict:
    media_type = "image/jpeg"
    if image_data[:4] == b"\x89PNG":
        media_type = "image/png"
    elif image_data[:2] == b"\xff\xd8":
        media_type = "image/jpeg"
    elif image_data[:4] == b"RIFF":
        media_type = "image/webp"

    b64 = base64.b64encode(image_data).decode()
    return {
        "type": "image_url",
        "image_url": {"url": f"data:{media_type};base64,{b64}"},
    }


# ── 主入口 ──────────────────────────────────────────────────────────


async def extract_recipe(
    text: str,
    title: str = "",
    image_urls: list[str] | None = None,
    max_images: int = 3,
    model: str = "deepseek-chat",
    api_key: Optional[str] = None,
) -> Recipe:
    """使用 LLM 从文本（+ 可选图片）中提取菜谱。

    Args:
        text: 文本内容（标题 + 描述 + 转写等，全量合并）
        title: 原标题（备用）
        image_urls: 图片 URL 列表（发给 LLM 做视觉分析）
        max_images: 最多发送的图片数
        model: 模型名称
        api_key: API Key

    Returns:
        结构化菜谱
    """
    api_key = api_key or _get_api_key()
    client = OpenAI(api_key=api_key, base_url=_get_base_url())

    msg_content: list[dict] = [
        {"type": "text", "text": text},
    ]

    # 添加图片
    if image_urls:
        loop = asyncio.get_event_loop()
        tasks = [
            loop.run_in_executor(None, _download_image, url)
            for url in image_urls[:max_images]
        ]
        image_data_list = await asyncio.gather(*tasks, return_exceptions=True)

        for img_data in image_data_list:
            if isinstance(img_data, bytes) and len(img_data) > 0:
                msg_content.append(_make_image_block(img_data))

    model_label = model
    print(f"  → 发送给 {model_label} 分析...")
    print(f"    文本长度: {len(text)} 字", end="")
    if image_urls:
        print(f", 图片: {len(image_urls[:max_images])} 张")
    else:
        print()
    try:
        response = client.chat.completions.create(
            model=model,
            max_tokens=2000,
            messages=[
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": msg_content},
            ],
            tools=EXTRACT_TOOLS,
            tool_choice="required",
        )
    except Exception as e:
        print(f"  ✗ API 调用失败: {e}")
        raise

    choice = response.choices[0]

    if choice.message.tool_calls:
        for tc in choice.message.tool_calls:
            if tc.function.name == "output_recipe":
                data = json.loads(tc.function.arguments)
                recipe = _parse_recipe(data)
                recipe_name = recipe.name or "未识别"
                print(f"  ✓ AI 分析完成: {recipe_name}" + (" (非美食)" if not recipe.is_food else ""))
                return recipe

        data = json.loads(choice.message.tool_calls[0].function.arguments)
        recipe = _parse_recipe(data)
        print(f"  ✓ AI 分析完成: {recipe.name or '未识别'}")
        return recipe

    # fallback: 从文本回复中解析
    if choice.message.content and len(choice.message.content) > 50:
        text = choice.message.content
        name = ""
        name_match = re.search(r"(?:菜名|菜品|名称)[：:]\s*(.+?)(?:\n|$)", text)
        if name_match:
            name = name_match.group(1).strip()
        if not name:
            name_match = re.search(r"###\s*(.+?)(?:\n|$)", text)
            if name_match:
                name = name_match.group(1).strip()
        if not name:
            name_match = re.search(r"[#*]{2,}\s*(.+?)[#*\n]", text)
            if name_match:
                name = name_match.group(1).strip()

        is_food = any(kw in text for kw in ["美食", "菜谱", "食材", "排骨", "鸡", "肉", "鱼", "烹饪", "烤", "炒", "煮", "蒸"])
        print(f"  ✓ AI 文本回退解析: {name or '未识别'}")
        return Recipe(name=name, source_url="", is_food=is_food, reason=None if is_food else text[:200])

    print("  ✗ AI 未返回结构化数据")
    return Recipe(name="", source_url="", is_food=False, reason="无法解析API返回")


def _parse_recipe(data: dict) -> Recipe:
    steps_raw = data.get("steps", [])
    steps = []
    for s in steps_raw:
        if isinstance(s, dict):
            steps.append(Step(title=s.get("title", ""), time=s.get("time"), content=s.get("content", "")))
        else:
            steps.append(Step(title="", content=str(s)))

    return Recipe(
        name=data.get("name", ""),
        total_time=data.get("total_time"),
        ingredients=[
            Ingredient(name=i["name"], amount=i.get("amount"), prep=i.get("prep"), category="食材")
            for i in data.get("ingredients", [])
        ],
        seasonings=[
            Ingredient(name=i["name"], amount=i.get("amount"), category="调料")
            for i in data.get("seasonings", [])
        ],
        equipment=data.get("equipment", []),
        steps=steps,
        tips=data.get("tips", []),
        source_url="",
        is_food=data.get("is_food", True),
        reason=data.get("reason"),
    )
