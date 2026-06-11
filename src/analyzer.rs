use crate::models::{Ingredient, Recipe, Step};
use serde_json::{json, Value};
use std::sync::OnceLock;

/// Extract a structured recipe from text + optional images using LLM function calling.
pub async fn extract_recipe(
    text: &str,
    _title: &str,
    image_urls: &[String],
    model: &str,
    api_key: Option<&str>,
) -> Result<Recipe, AnalyzerError> {
    let api_key = match api_key {
        Some(k) => k.to_string(),
        None => resolve_api_key()?,
    };

    let base_url = "https://api.deepseek.com";

    let msg_content = build_message_content(text, image_urls).await;

    let model_label = model;
    let img_count = if !image_urls.is_empty() {
        format!(" | 图片: {}", image_urls.len().min(3))
    } else {
        String::new()
    };
    println!("  → 发送给 {} 分析... ({} 字{})", model_label, text.chars().count(), img_count);

    let request_body = json!({
        "model": model,
        "max_tokens": 2000,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": msg_content},
        ],
        "tools": [{
            "type": "function",
            "function": {
                "name": "output_recipe",
                "description": "输出从内容中提取的菜谱信息",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "description": "菜谱名称"},
                        "total_time": {"type": "string", "description": "总耗时"},
                        "ingredients": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": {"type": "string"},
                                    "amount": {"type": "string"},
                                    "prep": {"type": "string"}
                                },
                                "required": ["name"]
                            }
                        },
                        "seasonings": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": {"type": "string"},
                                    "amount": {"type": "string"}
                                },
                                "required": ["name"]
                            }
                        },
                        "equipment": {"type": "array", "items": {"type": "string"}},
                        "steps": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "title": {"type": "string"},
                                    "time": {"type": "string"},
                                    "content": {"type": "string"}
                                },
                                "required": ["title", "content"]
                            }
                        },
                        "tips": {"type": "array", "items": {"type": "string"}},
                        "is_food": {"type": "boolean"},
                        "reason": {"type": "string"}
                    },
                    "required": ["name", "ingredients", "steps", "is_food"]
                }
            }
        }],
        "tool_choice": "required",
    });

    let client = shared_client();
    let response = client
        .post(format!("{}/chat/completions", base_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| AnalyzerError::ApiError(format!("request failed: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AnalyzerError::ApiError(format!(
            "HTTP {}: {}",
            status,
            &body[..body.len().min(200)]
        )));
    }

    let response_json: Value = response
        .json()
        .await
        .map_err(|e| AnalyzerError::ParseError(format!("invalid JSON response: {}", e)))?;

    let recipe = parse_response(response_json)?;
    let recipe_name = if recipe.name.is_empty() { "未识别" } else { &recipe.name };
    let suffix = if !recipe.is_food { " (非美食)" } else { "" };
    println!("  ✓ AI 分析完成: {}{}", recipe_name, suffix);
    Ok(recipe)
}

// ── API Key ────────────────────────────────────────────────────────

fn resolve_api_key() -> Result<String, AnalyzerError> {
    if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
        if !key.is_empty() {
            return Ok(key);
        }
    }
    // Try macOS keychain
    if let Ok(output) = std::process::Command::new("security")
        .args([
            "find-generic-password",
            "-a",
            &std::env::var("USER").unwrap_or_default(),
            "-s",
            "deepseek-api",
            "-w",
        ])
        .output()
    {
        if output.status.success() {
            let key = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !key.is_empty() {
                return Ok(key);
            }
        }
    }
    Err(AnalyzerError::MissingApiKey)
}

// ── System Prompt ──────────────────────────────────────────────────

const SYSTEM_PROMPT: &str = r#"你是专业厨师和食谱分析师。你擅长从小红书（RedNote）的美食内容中提取结构化菜谱信息。

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
- 用量单位保持原文（如克、毫升、勺、碗等）"#;

// ── Image Handling ─────────────────────────────────────────────────

async fn build_message_content(text: &str, image_urls: &[String]) -> Vec<Value> {
    let mut content: Vec<Value> = vec![json!({"type": "text", "text": text})];

    let max_images = 3;
    for url in image_urls.iter().take(max_images) {
        if let Some(data) = download_image(url).await {
            let block = make_image_block(&data);
            content.push(block);
        }
    }
    content
}

fn shared_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| reqwest::Client::new())
}

async fn download_image(url: &str) -> Option<Vec<u8>> {
    let resp = shared_client()
        .get(url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    let max_size = 5 * 1024 * 1024;
    if bytes.len() > max_size {
        return None;
    }
    Some(bytes.to_vec())
}

fn make_image_block(data: &[u8]) -> Value {
    let media_type = if data.len() > 4 && &data[..4] == b"\x89PNG" {
        "image/png"
    } else if data.len() > 2 && &data[..2] == b"\xff\xd8" {
        "image/jpeg"
    } else if data.len() > 4 && &data[..4] == b"RIFF" {
        "image/webp"
    } else {
        "image/jpeg"
    };

    let b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        data,
    );

    json!({
        "type": "image_url",
        "image_url": {
            "url": format!("data:{};base64,{}", media_type, b64)
        }
    })
}

// ── Response Parsing ───────────────────────────────────────────────

fn parse_response(response: Value) -> Result<Recipe, AnalyzerError> {
    let choice = response["choices"][0]
        .as_object()
        .ok_or_else(|| AnalyzerError::ParseError("no choices in response".into()))?;

    // Try tool_calls first
    if let Some(tool_calls) = choice.get("message").and_then(|m| m.get("tool_calls")) {
        if let Some(tc_array) = tool_calls.as_array() {
            for tc in tc_array {
                if tc["function"]["name"].as_str() == Some("output_recipe") {
                    let args = tc["function"]["arguments"]
                        .as_str()
                        .ok_or_else(|| AnalyzerError::ParseError("missing arguments".into()))?;
                    // Try direct parse first, then repair if needed
                    let data = serde_json::from_str::<Value>(args)
                        .or_else(|first_err| {
                            println!("  ⚠ JSON 格式错误 ({}), 尝试修复...", first_err);
                            repair_json(args)
                        })
                        .map_err(|e| AnalyzerError::ParseError(format!("invalid JSON in tool call: {}", e)))?;
                    return Ok(parse_recipe_data(&data));
                }
            }
        }
    }

    // Fallback: try to extract from text content
    if let Some(content) = choice.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
        if content.len() > 50 {
            return Ok(fallback_parse(content));
        }
    }

    Err(AnalyzerError::ParseError("no structured data in response".into()))
}

/// Repair common LLM JSON errors: truncation, trailing commas, unclosed brackets.
fn repair_json(raw: &str) -> Result<Value, serde_json::Error> {
    let trimmed = raw.trim();

    // Try direct parse first
    if let Ok(v) = serde_json::from_str(trimmed) {
        return Ok(v);
    }

    // Common fix 1: remove trailing commas before ] or }
    let fixed = regex::Regex::new(r",(\s*[}\]])")
        .ok()
        .map(|re| re.replace_all(trimmed, "$1").to_string());

    if let Some(ref s) = fixed {
        if let Ok(v) = serde_json::from_str(s.as_str()) {
            println!("  ✓ 修复 trailing comma 成功");
            return Ok(v);
        }
    }

    // Common fix 2: close unclosed strings, arrays, objects
    let repaired = close_unclosed(trimmed);
    if let Ok(v) = serde_json::from_str(&repaired) {
        println!("  ✓ 修复未闭合 JSON 成功");
        return Ok(v);
    }

    // One more attempt: remove trailing comma THEN close
    if let Some(ref s) = fixed {
        let repaired = close_unclosed(s);
        if let Ok(v) = serde_json::from_str(&repaired) {
            println!("  ✓ 修复 JSON 成功 (trailing comma + unclosed)");
            return Ok(v);
        }
    }

    // Return original error
    serde_json::from_str(trimmed)
}

/// Close unclosed strings, arrays, and objects in truncated JSON.
fn close_unclosed(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 10);
    let mut in_string = false;
    let mut escape = false;
    let mut stack: Vec<char> = Vec::new();

    for ch in s.chars() {
        result.push(ch);

        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' && in_string {
            escape = true;
            continue;
        }
        if ch == '"' && !escape {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match ch {
            '{' => stack.push('}'),
            '[' => stack.push(']'),
            '}' | ']' => {
                if stack.last() == Some(&ch) {
                    stack.pop();
                }
            }
            _ => {}
        }
    }

    // Close unclosed string
    if in_string {
        result.push('"');
    }

    // Remove trailing comma before closing brackets
    while result.ends_with(',') || result.ends_with('\n') || result.ends_with(' ') {
        result.pop();
    }

    // Close unclosed brackets
    while let Some(ch) = stack.pop() {
        result.push(ch);
    }

    result
}

fn parse_recipe_data(data: &Value) -> Recipe {
    let ingredients: Vec<Ingredient> = data["ingredients"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|i| Ingredient {
                    name: i["name"].as_str().unwrap_or_default().to_string(),
                    amount: i.get("amount").and_then(|v| v.as_str()).map(String::from),
                    prep: i.get("prep").and_then(|v| v.as_str()).map(String::from),
                    category: Some("食材".to_string()),
                })
                .collect()
        })
        .unwrap_or_default();

    let seasonings: Vec<Ingredient> = data["seasonings"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|i| Ingredient {
                    name: i["name"].as_str().unwrap_or_default().to_string(),
                    amount: i.get("amount").and_then(|v| v.as_str()).map(String::from),
                    prep: None,
                    category: Some("调料".to_string()),
                })
                .collect()
        })
        .unwrap_or_default();

    let steps: Vec<Step> = data["steps"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|s| {
                    // Steps can be objects {"title": ..., "content": ...} or plain strings
                    if let Some(obj) = s.as_object() {
                        Step {
                            title: obj.get("title").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                            time: obj.get("time").and_then(|v| v.as_str()).map(String::from),
                            content: obj.get("content").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                        }
                    } else if let Some(text) = s.as_str() {
                        Step {
                            title: String::new(),
                            time: None,
                            content: text.to_string(),
                        }
                    } else {
                        Step {
                            title: String::new(),
                            time: None,
                            content: s.to_string(),
                        }
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Recipe {
        name: data["name"].as_str().unwrap_or_default().to_string(),
        total_time: data.get("total_time").and_then(|v| v.as_str()).map(String::from),
        ingredients,
        seasonings,
        equipment: data["equipment"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default(),
        steps,
        tips: data["tips"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default(),
        source_url: String::new(),
        is_food: data["is_food"].as_bool().unwrap_or(true),
        reason: data.get("reason").and_then(|v| v.as_str()).map(String::from),
    }
}

fn fallback_parse(text: &str) -> Recipe {
    let name = extract_name(text);
    let is_food = text.contains("美食") || text.contains("菜谱") || text.contains("食材")
        || text.contains("排骨") || text.contains("鸡") || text.contains("肉")
        || text.contains("鱼") || text.contains("烹饪") || text.contains("烤")
        || text.contains("炒") || text.contains("煮") || text.contains("蒸");
    let recipe_name = if name.is_empty() { "未识别" } else { name };
    println!("  ✓ AI 文本回退解析: {}", recipe_name);
    Recipe {
        name: recipe_name.to_string(),
        source_url: String::new(),
        is_food,
        ..Default::default()
    }
}

fn extract_name(text: &str) -> &str {
    for prefix in &["菜名", "菜品", "名称"] {
        for sep in &['：', ':'] {
            if let Some(pos) = text.find(&format!("{}{}", prefix, sep)) {
                let start = pos + prefix.len() + sep.len_utf8();
                let rest = &text[start..];
                if let Some(end) = rest.find('\n') {
                    let name = rest[..end].trim();
                    if !name.is_empty() {
                        return name;
                    }
                } else {
                    let name = rest.trim();
                    if !name.is_empty() {
                        return name;
                    }
                }
            }
        }
    }
    // Try "### xxx"
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("###") {
            let name = rest.trim();
            if !name.is_empty() {
                return name;
            }
        }
    }
    ""
}

// ── Errors ─────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum AnalyzerError {
    NotImplemented,
    ApiError(String),
    ParseError(String),
    MissingApiKey,
}

impl std::fmt::Display for AnalyzerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotImplemented => write!(f, "analyzer not yet implemented"),
            Self::ApiError(msg) => write!(f, "API error: {}", msg),
            Self::ParseError(msg) => write!(f, "parse error: {}", msg),
            Self::MissingApiKey => write!(
                f,
                "未设置 DEEPSEEK_API_KEY。请通过以下方式之一配置：\n\
                 1. 设置环境变量：export DEEPSEEK_API_KEY=sk-...\n\
                 2. 存入 macOS 钥匙串：security add-generic-password -a \"$USER\" \
                 -s deepseek-api -w \"sk-...\""
            ),
        }
    }
}

impl std::error::Error for AnalyzerError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_recipe_data_full() {
        let data = json!({
            "name": "蒜香椒盐烤排骨",
            "total_time": "1小时25分钟",
            "ingredients": [
                {"name": "排骨", "amount": "适量", "prep": "清洗干净"}
            ],
            "seasonings": [
                {"name": "蒜", "amount": "适量"}
            ],
            "equipment": ["空气炸锅"],
            "steps": [
                {"title": "清洗", "time": "约5分钟", "content": "排骨清洗干净"}
            ],
            "tips": ["腌制时间建议至少1小时"],
            "is_food": true,
            "reason": null
        });
        let recipe = parse_recipe_data(&data);
        assert_eq!(recipe.name, "蒜香椒盐烤排骨");
        assert_eq!(recipe.total_time, Some("1小时25分钟".into()));
        assert_eq!(recipe.ingredients.len(), 1);
        assert_eq!(recipe.ingredients[0].name, "排骨");
        assert_eq!(recipe.ingredients[0].category, Some("食材".into()));
        assert_eq!(recipe.seasonings[0].name, "蒜");
        assert_eq!(recipe.equipment[0], "空气炸锅");
        assert_eq!(recipe.steps[0].title, "清洗");
        assert!(recipe.is_food);
    }

    #[test]
    fn test_parse_recipe_data_not_food() {
        let data = json!({
            "name": "旅游攻略",
            "is_food": false,
            "reason": "这是一篇旅游攻略，不是美食内容"
        });
        let recipe = parse_recipe_data(&data);
        assert!(!recipe.is_food);
        assert!(recipe.reason.unwrap().contains("旅游"));
    }

    #[test]
    fn test_parse_recipe_data_empty_arrays() {
        let data = json!({
            "name": "test",
            "is_food": true
        });
        let recipe = parse_recipe_data(&data);
        assert!(recipe.ingredients.is_empty());
        assert!(recipe.seasonings.is_empty());
        assert!(recipe.equipment.is_empty());
        assert!(recipe.steps.is_empty());
        assert!(recipe.tips.is_empty());
    }

    #[test]
    fn test_fallback_parse_with_name() {
        let text = "菜名：红烧肉\n步骤：...";
        let recipe = fallback_parse(text);
        assert_eq!(recipe.name, "红烧肉");
        assert!(recipe.is_food);
    }

    #[test]
    fn test_fallback_parse_no_name() {
        let text = "这是一篇旅游攻略，介绍景点";
        let recipe = fallback_parse(text);
        assert_eq!(recipe.name, "未识别");
        assert!(!recipe.is_food);
    }

    #[test]
    fn test_fallback_parse_with_markdown_heading() {
        let text = "### 鱼香肉丝\n\n食材：...";
        let recipe = fallback_parse(text);
        assert_eq!(recipe.name, "鱼香肉丝");
    }

    #[test]
    fn test_extract_name_with_colon() {
        let text = "菜品：宫保鸡丁\n材料：...";
        assert_eq!(extract_name(text), "宫保鸡丁");
    }

    #[test]
    fn test_extract_name_with_chinese_colon() {
        let text = "名称：麻婆豆腐\n步骤：...";
        assert_eq!(extract_name(text), "麻婆豆腐");
    }

    #[test]
    fn test_make_image_block_png() {
        let png_header = b"\x89PNG\r\n\x1a\n";
        let block = make_image_block(png_header);
        assert_eq!(block["image_url"]["url"].as_str().unwrap().split(';').next().unwrap(), "data:image/png");
    }

    #[test]
    fn test_make_image_block_jpeg() {
        let jpeg_header = b"\xff\xd8\xff\xe0";
        let block = make_image_block(jpeg_header);
        assert_eq!(block["image_url"]["url"].as_str().unwrap().split(';').next().unwrap(), "data:image/jpeg");
    }

    #[test]
    fn test_make_image_block_webp() {
        let webp_header = b"RIFFxxxxWEBP";
        let block = make_image_block(webp_header);
        assert_eq!(block["image_url"]["url"].as_str().unwrap().split(';').next().unwrap(), "data:image/webp");
    }

    #[test]
    fn test_make_image_block_default_jpeg() {
        let unknown = b"\x00\x00\x00\x00random";
        let block = make_image_block(unknown);
        assert_eq!(block["image_url"]["url"].as_str().unwrap().split(';').next().unwrap(), "data:image/jpeg");
    }

    #[test]
    fn test_parse_recipe_minimal() {
        // Only required fields
        let data = json!({
            "name": "炒鸡蛋",
            "ingredients": [{"name": "鸡蛋"}],
            "steps": [{"title": "炒", "content": "打散鸡蛋炒熟"}],
            "is_food": true
        });
        let recipe = parse_recipe_data(&data);
        assert_eq!(recipe.name, "炒鸡蛋");
        assert_eq!(recipe.ingredients.len(), 1);
        assert_eq!(recipe.ingredients[0].amount, None);
        assert_eq!(recipe.ingredients[0].prep, None);
        assert_eq!(recipe.seasonings.len(), 0);
        assert_eq!(recipe.equipment.len(), 0);
        assert_eq!(recipe.tips.len(), 0);
        assert_eq!(recipe.total_time, None);
    }

    #[test]
    fn test_parse_recipe_step_as_string() {
        let data = json!({
            "name": "汤",
            "ingredients": [{"name": "水"}],
            "steps": [
                {"title": "煮", "content": "烧开水"},
                "直接放盐"
            ],
            "is_food": true
        });
        let recipe = parse_recipe_data(&data);
        assert_eq!(recipe.steps.len(), 2);
        assert_eq!(recipe.steps[0].title, "煮");
        assert_eq!(recipe.steps[1].title, "");
        assert_eq!(recipe.steps[1].content, "直接放盐");
    }

    #[test]
    fn test_parse_recipe_empty_data() {
        let data = json!({});
        let recipe = parse_recipe_data(&data);
        assert!(recipe.name.is_empty());
        assert!(recipe.ingredients.is_empty());
        assert!(recipe.steps.is_empty());
        assert!(recipe.is_food);
    }

    #[test]
    fn test_parse_recipe_null_fields() {
        let data = json!({
            "name": "菜",
            "ingredients": null,
            "steps": null,
            "tips": null,
            "equipment": null,
            "seasonings": null,
            "is_food": null
        });
        let recipe = parse_recipe_data(&data);
        assert_eq!(recipe.name, "菜");
        assert!(recipe.ingredients.is_empty());
        assert!(recipe.steps.is_empty());
        assert!(recipe.tips.is_empty());
        assert!(recipe.equipment.is_empty());
        assert!(recipe.seasonings.is_empty());
        assert!(recipe.is_food);
    }

    #[test]
    fn test_parse_response_tool_calls() {
        let response = json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "function": {
                            "name": "output_recipe",
                            "arguments": r#"{"name": "测试菜", "ingredients": [{"name": "肉"}], "steps": [{"title": "步骤1", "content": "做"}], "is_food": true}"#
                        }
                    }]
                }
            }]
        });
        let recipe = parse_response(response).unwrap();
        assert_eq!(recipe.name, "测试菜");
        assert_eq!(recipe.ingredients[0].name, "肉");
    }

    #[test]
    fn test_parse_response_no_tool_calls() {
        let response = json!({
            "choices": [{
                "message": {
                    "content": "菜名：红烧肉\n这是一道美食，需要炖煮。美食推荐。"
                }
            }]
        });
        let recipe = parse_response(response).unwrap();
        assert_eq!(recipe.name, "红烧肉");
        assert!(recipe.is_food);
    }

    #[test]
    fn test_parse_response_no_choices() {
        let response = json!({});
        let result = parse_response(response);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no choices"));
    }

    #[test]
    fn test_extract_name_with_mixed_content() {
        let text = "一些前缀文本\n菜名：京酱肉丝\n然后是一些操作步骤";
        assert_eq!(extract_name(text), "京酱肉丝");
    }

    #[test]
    fn test_extract_name_no_match() {
        let text = "这是一篇没有菜名的文章";
        assert_eq!(extract_name(text), "");
    }

    #[test]
    fn test_fallback_parse_detects_food_keywords() {
        let tests = vec![
            ("清蒸鲈鱼的做法", true),
            ("红烧排骨", true),
            ("今天天气真好", false),
            ("如何更换轮胎", false),
        ];
        for (text, expected_food) in tests {
            let recipe = fallback_parse(text);
            assert_eq!(recipe.is_food, expected_food, "text: {}", text);
        }
    }
}
