use crate::models::{Ingredient, Recipe, Step};
use serde_json::{json, Value};
use std::sync::OnceLock;

// ── HTTP Client Trait ────────────────────────────────────────────

/// HTTP abstraction for the analyzer, enabling unit tests without network calls.
#[allow(async_fn_in_trait)]
pub trait HttpClient: Send + Sync {
    /// POST JSON to a URL with Bearer auth, return parsed JSON response.
    async fn post_json(&self, url: &str, api_key: &str, body: Value) -> Result<Value, AnalyzerError>;
}

/// Production HTTP client wrapping reqwest.
pub struct RealHttpClient {
    inner: reqwest::Client,
}

impl RealHttpClient {
    pub fn new() -> Self {
        Self { inner: reqwest::Client::new() }
    }
}

impl Default for RealHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient for RealHttpClient {
    async fn post_json(&self, url: &str, api_key: &str, body: Value) -> Result<Value, AnalyzerError> {
        let response = self.inner
            .post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
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

        response.json().await.map_err(|e| AnalyzerError::ParseError(format!("invalid JSON response: {}", e)))
    }
}

/// Shared static client, used in production.
pub(crate) fn shared_client() -> &'static RealHttpClient {
    static CLIENT: OnceLock<RealHttpClient> = OnceLock::new();
    CLIENT.get_or_init(RealHttpClient::new)
}

// ── Extract Recipe ──────────────────────────────────────────────

/// Extract structured recipe(s) from text + optional images using LLM function calling.
/// Returns multiple recipes when the content is a collection (e.g. multi-recipe post).
pub async fn extract_recipe(
    client: &impl HttpClient,
    text: &str,
    _image_urls: &[String],
    model: &str,
    api_key: Option<&str>,
) -> Result<Vec<Recipe>, AnalyzerError> {
    let api_key = match api_key {
        Some(k) => k.to_string(),
        None => resolve_api_key()?,
    };

    let base_url = "https://api.deepseek.com";

    // Images are OCR'd into text_content at the textifier stage, not sent to LLM directly.
    println!("  → 发送给 {} 分析... ({} 字, {} 字节)", model, text.chars().count(), text.len());
    crate::vprintln!("  ⚠ 完整内容:\n{}", text);
    let msg_content = vec![json!({"type": "text", "text": text})];

    // ── Recipe item schema (shared by single and multi output) ────
    let recipe_schema = json!({
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
    });

    let request_body = json!({
        "model": model,
        "max_tokens": 8192,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": msg_content},
        ],
        "tools": [{
            "type": "function",
            "function": {
                "name": "output_recipes",
                "description": "输出从内容中提取的一个或多个菜谱信息",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "recipes": {
                            "type": "array",
                            "description": "菜谱列表（单个菜谱也放在数组中）",
                            "items": recipe_schema
                        }
                    },
                    "required": ["recipes"]
                }
            }
        }],
        "tool_choice": "required",
    });

    let response_json = client.post_json(
        &format!("{}/chat/completions", base_url),
        &api_key,
        request_body,
    ).await?;

    let recipes = parse_response(response_json)?;

    let count = recipes.len();
    let first_name = recipes.first().map(|r| r.name.as_str()).unwrap_or("");
    let suffix = if count == 1 {
        let is_food = recipes[0].is_food;
        let note = if !is_food { " (非美食)" } else { "" };
        format!("{}{}", first_name, note)
    } else {
        let food_count = recipes.iter().filter(|r| r.is_food).count();
        format!("{} ({}个菜谱)", first_name, food_count)
    };
    println!("  ✓ AI 分析完成: {}", suffix);
    Ok(recipes)
}

// ── API Key ────────────────────────────────────────────────────────

fn resolve_api_key() -> Result<String, AnalyzerError> {
    if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
        if !key.is_empty() {
            return Ok(key);
        }
    }
    // Try macOS keychain
    if cfg!(target_os = "macos") {
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
    }
    Err(AnalyzerError::MissingApiKey(platform_api_key_hint().to_string()))
}

fn platform_api_key_hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "未设置 DEEPSEEK_API_KEY。请通过以下方式之一配置：\n\
         1. 设置环境变量：export DEEPSEEK_API_KEY=sk-...\n\
         2. 存入 macOS 钥匙串：security add-generic-password -a \"$USER\" -s deepseek-api -w \"sk-...\""
    } else if cfg!(target_os = "windows") {
        "未设置 DEEPSEEK_API_KEY。请通过以下方式配置：\n\
         1. 设置环境变量：set DEEPSEEK_API_KEY=sk-...\n\
         2. 添加到 .env 文件：DEEPSEEK_API_KEY=sk-..."
    } else {
        "未设置 DEEPSEEK_API_KEY。请通过以下方式配置：\n\
         1. 设置环境变量：export DEEPSEEK_API_KEY=sk-...\n\
         2. 添加到 .env 文件：DEEPSEEK_API_KEY=sk-..."
    }
}

// ── System Prompt ──────────────────────────────────────────────────

const SYSTEM_PROMPT: &str = r#"你是专业厨师和食谱分析师。你擅长从小红书（RedNote）的美食内容中提取结构化菜谱信息。

你可以分析的内容包括：
- 笔记的文字描述（标题 + 正文）
- 视频的语音转写文本（博主的口述内容）
- 视频画面 OCR 文字（视频中显示的操作说明、食材列表等）
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
- 用量单位保持原文（如克、毫升、勺、碗等）
- **重要：如果内容中包含多个菜谱（合集、多图对应多菜谱等），请将每个菜谱分别提取为独立的菜谱对象，放入 recipes 数组中**"#;

// ── Image Handling ─────────────────────────────────────────────────

// Image-to-LLM code was removed because DeepSeek does not support image input.
// Images are OCR'd at the textifier stage and included in the text content.

// ── Response Parsing ───────────────────────────────────────────────

fn parse_response(response: Value) -> Result<Vec<Recipe>, AnalyzerError> {
    let choice = response["choices"][0]
        .as_object()
        .ok_or_else(|| AnalyzerError::ParseError("no choices in response".into()))?;

    // Try tool_calls first
    if let Some(tool_calls) = choice.get("message").and_then(|m| m.get("tool_calls")) {
        if let Some(tc_array) = tool_calls.as_array() {
            for tc in tc_array {
                let func_name = tc["function"]["name"].as_str().unwrap_or("");
                if func_name == "output_recipes" || func_name == "output_recipe" {
                    let args = tc["function"]["arguments"]
                        .as_str()
                        .ok_or_else(|| AnalyzerError::ParseError("missing arguments".into()))?;
                    // Diagnostic: show finish_reason and raw count
                    if let Some(reason) = choice.get("finish_reason").and_then(|v| v.as_str()) {
                        crate::vprintln!("  ⚠ finish_reason: {}", reason);
                    }
                    crate::vprintln!("  ⚠ 原始参数长度: {} 字符, {} 字节", args.chars().count(), args.len());
                    let raw_count = args.matches(r#""name""#).count();
                    crate::vprintln!("  ⚠ 原始参数中 \"name\" 出现次数: {}", raw_count);
                    let data = serde_json::from_str::<Value>(args)
                        .or_else(|first_err| {
                            println!("  ⚠ JSON 格式错误 ({}), 尝试修复...", first_err);
                            repair_json(args)
                        })
                        .map_err(|e| AnalyzerError::ParseError(format!("invalid JSON in tool call: {}", e)))?;

                    if func_name == "output_recipes" {
                        if let Some(recipes_array) = data["recipes"].as_array() {
                            if !recipes_array.is_empty() {
                                return Ok(recipes_array.iter().map(parse_recipe_data).collect());
                            }
                        }
                        // Fallback: if recipes array is missing/empty, try top-level keys (single recipe)
                        if data.get("name").is_some() || data.get("ingredients").is_some() {
                            return Ok(vec![parse_recipe_data(&data)]);
                        }
                    } else {
                        // Old output_recipe: single recipe
                        return Ok(vec![parse_recipe_data(&data)]);
                    }
                }
            }
        }
    }

    // Fallback: try to extract from text content
    if let Some(content) = choice.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
        if content.len() > 50 {
            return Ok(vec![fallback_parse(content)]);
        }
    }

    // Debug: log bad JSON for diagnosis
    if let Some(content) = choice.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
        if content.len() > 200 {
            crate::vprintln!("  ⚠ LLM 原始响应 (前200字): {}...", &content[..200]);
        }
    }
    // Show tool call arguments for debugging
    if let Some(tool_calls) = choice.get("message").and_then(|m| m.get("tool_calls")) {
        if let Some(tc_array) = tool_calls.as_array() {
            for tc in tc_array {
                if let Some(args) = tc["function"]["arguments"].as_str() {
                    if args.len() > 100 {
                        crate::vprintln!("  ⚠ 工具参数 (前100字): {}...", &args[..100]);
                        crate::vprintln!("  ⚠ 工具参数 (后100字): ...{}", &args[args.len().saturating_sub(100)..]);
                    }
                }
            }
        }
    }

    Err(AnalyzerError::ParseError("no structured data in response".into()))
}

/// Repair common LLM JSON errors: truncation, trailing commas, unclosed brackets, missing commas.
fn repair_json(raw: &str) -> Result<Value, serde_json::Error> {
    // Helper: try parse, fallback to extract_balanced_json
    let try_val = |s: &str| -> Option<Value> {
        serde_json::from_str(s).ok().or_else(|| {
            extract_balanced_json(s).and_then(|e| serde_json::from_str(&e).ok())
        })
    };

    let trimmed = raw.trim();

    // Try direct parse first
    let first_err = match serde_json::from_str(trimmed) {
        Ok(v) => return Ok(v),
        Err(e) => e,
    };

    // Show context around error position for debugging
    let col = first_err.column();
    if col > 0 && !trimmed.is_empty() {
        let byte_pos = col.saturating_sub(1).min(trimmed.len());
        let ctx_start = byte_pos.saturating_sub(80);
        let ctx_start = (ctx_start..=byte_pos)
            .rfind(|&i| trimmed.is_char_boundary(i))
            .unwrap_or(0);
        if ctx_start < trimmed.len() {
            let snippet = &trimmed[ctx_start..];
            let end = snippet.char_indices().nth(160).map(|(i, _)| i).unwrap_or(snippet.len());
            eprintln!("  ⚠ 错误位置 {} 附近: ...{}...", col, &snippet[..end]);
        }
    }

    // Strip markdown code fences if present (```json ... ```)
    let trimmed = strip_code_fence(trimmed);
    let trimmed = trimmed.as_str();

    // Step 0a: sanitize unescaped control characters in JSON strings
    let sanitized = sanitize_control_chars(trimmed);
    if sanitized != trimmed {
        if let Some(v) = try_val(&sanitized) {
            println!("  ✓ 清理控制字符成功");
            return Ok(v);
        }
    }

    // Step 0b: escape unescaped double quotes in string values (common LLM mistake)
    let escaped_quotes = escape_unescaped_quotes(trimmed);
    if escaped_quotes != trimmed {
        if let Some(v) = try_val(&escaped_quotes) {
            println!("  ✓ 转义未转义引号成功");
            return Ok(v);
        }
        // Also try with close_unclosed (handles truncation + unescaped quotes)
        let escaped_closed = close_unclosed(&escaped_quotes);
        if escaped_closed != escaped_quotes {
            if let Some(v) = try_val(&escaped_closed) {
                println!("  ✓ 转义引号 + 闭合 JSON 成功");
                return Ok(v);
            }
        }
    }

    // Step 0: try extracting balanced JSON from the content (handles extra text after JSON)
    if let Some(v) = try_val(trimmed) {
        println!("  ✓ 提取平衡 JSON 成功");
        return Ok(v);
    }

    // Step 1: remove trailing commas before ] or }
    static TRAILING_COMMA_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let trailing_re = TRAILING_COMMA_RE.get_or_init(|| {
        regex::Regex::new(r",(\s*[}\]])").expect("invalid trailing comma regex")
    });
    let fixed = trailing_re.replace_all(trimmed, "$1").to_string();

    if let Some(v) = try_val(&fixed) {
        println!("  ✓ 修复 trailing comma 成功");
        return Ok(v);
    }

    // Step 2: insert missing commas between adjacent structural tokens
    let with_commas = add_missing_commas(trimmed);
    if with_commas != trimmed {
        if let Some(v) = try_val(&with_commas) {
            println!("  ✓ 修复缺失逗号成功");
            return Ok(v);
        }
    }

    // Step 3: trailing comma + missing commas together
    let merged = add_missing_commas(&fixed);
    if merged != fixed {
        if let Some(v) = try_val(&merged) {
            println!("  ✓ 修复 JSON 成功 (trailing comma + missing comma)");
            return Ok(v);
        }
    }

    // Step 4: close unclosed strings, arrays, objects
    let repaired = close_unclosed(trimmed);
    if let Some(v) = try_val(&repaired) {
        println!("  ✓ 修复未闭合 JSON 成功");
        return Ok(v);
    }

    // Step 5: trailing comma + close unclosed
    let repaired = close_unclosed(&fixed);
    if let Some(v) = try_val(&repaired) {
        println!("  ✓ 修复 JSON 成功 (trailing comma + unclosed)");
        return Ok(v);
    }

    // Step 6: missing commas + close unclosed
    let repaired = close_unclosed(&with_commas);
    if let Some(v) = try_val(&repaired) {
        println!("  ✓ 修复 JSON 成功 (missing comma + unclosed)");
        return Ok(v);
    }

    // Step 7: last resort — try trimming trailing characters one at a time
    // (handles truncation artifacts that close_unclosed doesn't cover)
    let trimmed_chars: Vec<(usize, char)> = trimmed.char_indices().collect();
    for n in 1..trimmed_chars.len().min(50) {
        let (end_idx, _) = trimmed_chars[trimmed_chars.len() - n];
        let candidate = &trimmed[..end_idx];
        if let Some(v) = try_val(candidate) {
            println!("  ✓ 修复 JSON 成功 (trim trailing {} chars)", n);
            return Ok(v);
        }
        // Try with escaped quotes on the trimmed candidate
        let escaped = escape_unescaped_quotes(candidate);
        if let Some(v) = try_val(&escaped) {
            println!("  ✓ 修复 JSON 成功 (trim+escape {} chars)", n);
            return Ok(v);
        }
    }

    // Return original error
    serde_json::from_str(trimmed)
}

/// Try to extract a balanced JSON object or array from text that may have extra content.
/// Scans for the first `{` or `[`, then finds the matching closing brace/bracket.
fn extract_balanced_json(s: &str) -> Option<String> {
    let s = s.trim();
    // Find the first structural character
    let start = s.find(['{', '['])?;
    let target = &s[start..];
    let chars = target.char_indices().peekable();
    let mut in_string = false;
    let mut escape = false;
    let mut depth_obj: i32 = 0;
    let mut depth_arr: i32 = 0;
    let mut start_idx = 0;
    let mut end_idx = 0;

    for (i, ch) in chars {
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
            '{' => {
                if depth_obj == 0 && depth_arr == 0 {
                    start_idx = i;
                }
                depth_obj += 1;
            }
            '}' if depth_obj == 1 && depth_arr == 0 => {
                end_idx = i + 1; // include the closing brace
                break;
            }
            '}' => depth_obj -= 1,
            '[' => depth_arr += 1,
            ']' => depth_arr -= 1,
            _ => {}
        }
    }

    if end_idx > start_idx {
        Some(target[start_idx..end_idx].to_string())
    } else {
        None
    }
}

/// Strip markdown code fences and surrounding whitespace (```json ... ```, ~~~json ... ~~~).
fn strip_code_fence(s: &str) -> String {
    let s = s.trim();
    // ```json ... ``` or ``` ... ```
    if let Some(inner) = s.strip_prefix("```").or_else(|| s.strip_prefix("~~~")) {
        // Find the first newline to remove the opening line
        if let Some(nl) = inner.find('\n') {
            let after_open = inner[nl + 1..].trim();
            // Strip trailing ```
            if let Some(end) = after_open.rfind("```").or_else(|| after_open.rfind("~~~")) {
                return after_open[..end].trim().to_string();
            }
            return after_open.to_string();
        }
    }
    s.to_string()
}

/// Insert commas between adjacent structural tokens that are missing them.
/// Handles patterns like `}{`, `}[`, `]{`, `][` that appear outside strings.
fn add_missing_commas(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 16);
    let mut in_string = false;
    let mut escape = false;
    let mut prev_struct: Option<char> = None; // last non-whitespace structural char

    for ch in s.chars() {
        if escape {
            escape = false;
            result.push(ch);
            continue;
        }
        if ch == '\\' && in_string {
            escape = true;
            result.push(ch);
            continue;
        }
        if ch == '"' && !escape {
            in_string = !in_string;
            result.push(ch);
            if !in_string {
                // When a string closes, update prev_struct
                prev_struct = Some('"');
            }
            continue;
        }
        if in_string {
            result.push(ch);
            continue;
        }

        // Skip whitespace outside strings (don't push yet)
        if ch.is_ascii_whitespace() {
            result.push(ch);
            continue;
        }

        // Check if we need a comma between prev_struct and current char
        match (prev_struct, ch) {
            (Some('}'), '{') | (Some('}'), '[') | (Some('}'), '"')
            | (Some(']'), '{') | (Some(']'), '[') | (Some(']'), '"')
            | (Some('"'), '{') | (Some('"'), '[') | (Some('"'), '"') => {
                result.push(',');
            }
            _ => {}
        }

        prev_struct = Some(ch);
        result.push(ch);
    }

    result
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
            '}' | ']' if stack.last() == Some(&ch) => {
                stack.pop();
            }
            _ => {}
        }
    }

    // Close unclosed string
    if in_string {
        if escape {
            // Input ended inside an incomplete escape sequence (trailing \).
            // Remove the orphaned backslash so the closing quote isn't escaped.
            if result.ends_with('\\') {
                result.pop();
            }
        }
        result.push('"');
    }

    // Remove trailing comma, newline, space before closing brackets
    let trimmed_len = result.trim_end_matches(&[',', '\n', ' '][..]).len();
    result.truncate(trimmed_len);

    // Close unclosed brackets
    while let Some(ch) = stack.pop() {
        result.push(ch);
    }

    result
}

/// Escape unescaped control characters within JSON string values.
/// serde_json rejects literal newlines, tabs, etc. inside strings.
fn sanitize_control_chars(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_string = false;
    let mut escape = false;

    for ch in s.chars() {
        if escape {
            escape = false;
            result.push(ch);
            continue;
        }
        if ch == '\\' && in_string {
            escape = true;
            result.push(ch);
            continue;
        }
        if ch == '"' && !escape {
            in_string = !in_string;
            result.push(ch);
            continue;
        }
        if in_string && matches!(ch, '\n' | '\r' | '\t') {
            result.push_str(match ch {
                '\n' => "\\n",
                '\r' => "\\r",
                '\t' => "\\t",
                _ => unreachable!(),
            });
        } else {
            result.push(ch);
        }
    }

    result
}

/// Try to fix unescaped double quotes inside JSON string values.
/// Common LLM mistake: `"content": "他说"好"。"` — the inner `"` should be `\"`.
/// Heuristic: a `"` is structural (ends the string) only if it's followed by `,`, `}`, `]`, or `:`
/// (past any whitespace). Otherwise it's content and should be escaped.
fn escape_unescaped_quotes(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 8);
    let mut in_string = false;
    let mut escape = false;
    let mut chars = s.char_indices().peekable();

    while let Some((_, ch)) = chars.next() {
        if escape {
            escape = false;
            result.push(ch);
            continue;
        }
        if ch == '\\' && in_string {
            escape = true;
            result.push(ch);
            continue;
        }
        if ch == '"' && !escape {
            if in_string {
                // Look ahead past whitespace to see if this is structural
                let is_structural = chars
                    .clone()
                    .find(|(_, c)| !c.is_ascii_whitespace())
                    .map(|(_, c)| matches!(c, ',' | '}' | ']' | ':'))
                    .unwrap_or(true); // end of input = structural (truncation)

                if is_structural {
                    in_string = false;
                    result.push('"');
                } else {
                    result.push_str("\\\"");
                }
            } else {
                in_string = true;
                result.push('"');
            }
            continue;
        }
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

#[derive(Debug, thiserror::Error)]
pub enum AnalyzerError {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("parse error: {0}")]
    ParseError(String),
    #[error("{0}")]
    MissingApiKey(String),
}

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
                            "name": "output_recipes",
                            "arguments": r#"{"recipes":[{"name": "测试菜", "ingredients": [{"name": "肉"}], "steps": [{"title": "步骤1", "content": "做"}], "is_food": true}]}"#
                        }
                    }]
                }
            }]
        });
        let recipes = parse_response(response).unwrap();
        assert_eq!(recipes.len(), 1);
        assert_eq!(recipes[0].name, "测试菜");
        assert_eq!(recipes[0].ingredients[0].name, "肉");
    }

    #[test]
    fn test_parse_response_multi_recipes() {
        let response = json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "function": {
                            "name": "output_recipes",
                            "arguments": r#"{"recipes":[{"name":"菜1","ingredients":[{"name":"肉"}],"steps":[{"title":"步骤","content":"做"}],"is_food":true},{"name":"菜2","ingredients":[{"name":"鱼"}],"steps":[{"title":"步骤","content":"煮"}],"is_food":true}]}"#
                        }
                    }]
                }
            }]
        });
        let recipes = parse_response(response).unwrap();
        assert_eq!(recipes.len(), 2);
        assert_eq!(recipes[0].name, "菜1");
        assert_eq!(recipes[1].name, "菜2");
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
        let recipes = parse_response(response).unwrap();
        assert_eq!(recipes.len(), 1);
        assert_eq!(recipes[0].name, "红烧肉");
        assert!(recipes[0].is_food);
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

    // ── Mock client + extract_recipe tests ─────────────────────────

    struct TestClient {
        post_response: Option<Value>,
        post_error: Option<String>,
    }

    impl TestClient {
        fn with_response(val: Value) -> Self {
            Self { post_response: Some(val), post_error: None }
        }
        fn with_error(msg: &str) -> Self {
            Self { post_response: None, post_error: Some(msg.into()) }
        }
    }

    impl HttpClient for TestClient {
        async fn post_json(&self, _url: &str, _api_key: &str, _body: Value) -> Result<Value, AnalyzerError> {
            if let Some(val) = &self.post_response {
                return Ok(val.clone());
            }
            if let Some(msg) = &self.post_error {
                return Err(AnalyzerError::ApiError(msg.clone()));
            }
            Err(AnalyzerError::ApiError("unexpected call".into()))
        }
    }

    #[test]
    fn test_extract_recipe_success() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = TestClient::with_response(json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "function": {
                            "name": "output_recipes",
                            "arguments": r#"{"recipes":[{"name":"测试菜","ingredients":[{"name":"肉"}],"steps":[{"title":"步骤1","content":"做"}],"is_food":true}]}"#
                        }
                    }]
                }
            }]
        }));
        let recipes = rt.block_on(extract_recipe(
            &client, "做菜步骤", &[], "deepseek-chat", Some("sk-test"),
        )).unwrap();
        assert_eq!(recipes.len(), 1);
        assert_eq!(recipes[0].name, "测试菜");
        assert_eq!(recipes[0].ingredients[0].name, "肉");
        assert!(recipes[0].is_food);
    }

    #[test]
    fn test_extract_recipe_api_error() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = TestClient::with_error("HTTP 500: Internal Server Error");
        let result = rt.block_on(extract_recipe(
            &client, "做菜步骤", &[], "deepseek-chat", Some("sk-test"),
        ));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AnalyzerError::ApiError(_)));
        assert!(err.to_string().contains("500"));
    }

    // ── repair_json / close_unclosed tests ─────────────────────

    #[test]
    fn test_repair_json_trailing_comma() {
        let raw = r#"{"name":"测试","ingredients":[{"name":"肉"},]}"#;
        let result = repair_json(raw).unwrap();
        assert_eq!(result["name"], "测试");
        assert_eq!(result["ingredients"][0]["name"], "肉");
    }

    #[test]
    fn test_repair_json_truncated_no_close_brace() {
        let raw = r#"{"name":"测试","ingredients":[{"name":"肉"}"#;
        let result = repair_json(raw).unwrap();
        assert_eq!(result["name"], "测试");
        assert_eq!(result["ingredients"][0]["name"], "肉");
    }

    #[test]
    fn test_repair_json_truncated_no_close_bracket() {
        let raw = r#"{"name":"测试","ingredients":[{"name":"肉"},{"name":"鱼"}"#;
        let result = repair_json(raw).unwrap();
        assert_eq!(result["ingredients"][1]["name"], "鱼");
    }

    #[test]
    fn test_repair_json_truncated_missing_string_end() {
        // String value's closing quote is missing at end of input
        let raw = r#"{"name":"测试菜"#;
        let result = repair_json(raw).unwrap();
        assert_eq!(result["name"], "测试菜");
    }

    #[test]
    fn test_repair_json_trailing_comma_then_truncated() {
        let raw = r#"{"name":"测试","ingredients":[{"name":"肉"},],"#;
        let result = repair_json(raw).unwrap();
        assert_eq!(result["ingredients"][0]["name"], "肉");
    }

    #[test]
    fn test_repair_json_valid_no_fix_needed() {
        let raw = r#"{"name":"ok","ingredients":[],"steps":[],"is_food":true}"#;
        assert!(repair_json(raw).is_ok());
    }

    #[test]
    fn test_repair_json_nested_unclosed() {
        let raw = r#"{"name":"a","steps":[{"title":"t","content":"c","time":"5"}"#;
        let result = repair_json(raw).unwrap();
        assert_eq!(result["steps"][0]["title"], "t");
        assert_eq!(result["steps"][0]["content"], "c");
    }

    #[test]
    fn test_repair_json_missing_comma() {
        // Missing comma between objects in array (common LLM error)
        let raw = r#"{"name":"烤排骨","ingredients":[{"name":"排骨"}{"name":"蒜"}],"is_food":true}"#;
        let result = repair_json(raw).unwrap();
        assert_eq!(result["ingredients"][0]["name"], "排骨");
        assert_eq!(result["ingredients"][1]["name"], "蒜");
    }

    #[test]
    fn test_close_unclosed_empty_string() {
        assert_eq!(close_unclosed(""), "");
    }

    #[test]
    fn test_close_unclosed_no_brackets() {
        assert_eq!(close_unclosed(r#"{"key": "value"}"#), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_close_unclosed_trailing_comma_removed() {
        // close_unclosed doesn't fix trailing commas (repair_json handles that via regex)
        let result = close_unclosed(r#"{"key": "value",}"#);
        assert_eq!(result, r#"{"key": "value",}"#);
    }

    #[test]
    fn test_close_unclosed_missing_array_close() {
        let result = close_unclosed(r#"{"key": [1, 2, 3"#);
        // 3 is not in a string, so just close array then object
        assert_eq!(result, r#"{"key": [1, 2, 3]}"#);
    }

    // ── parse_response error paths ──────────────────────────────

    #[test]
    fn test_parse_response_short_content_no_fallback() {
        // Content is too short (< 50 chars) for fallback_parse
        let response = json!({
            "choices": [{
                "message": {
                    "content": "short text"
                }
            }]
        });
        let result = parse_response(response);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no structured data"));
    }

    #[test]
    fn test_parse_response_empty_choices() {
        let response = json!({
            "choices": []
        });
        let result = parse_response(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_response_missing_choices_key() {
        let response = json!({"message": "unexpected"});
        let result = parse_response(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_repair_json_garbage_input() {
        let raw = "not json at all!@#$%";
        assert!(repair_json(raw).is_err());
    }
}
