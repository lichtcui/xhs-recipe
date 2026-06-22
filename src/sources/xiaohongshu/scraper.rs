/// Xiaohongshu page scraper using reqwest HTTP (no browser automation).
///
/// Strategy:
/// 1. __NEXT_DATA__ JSON via HTML parsing (primary)
/// 2. OG meta tags via HTML parsing (fallback)
use crate::models::{ContentType, RawContent};
use super::super::SourceError;
use super::url::extract_note_id;
use std::time::Duration;

pub async fn scrape(url: &str) -> Result<RawContent, SourceError> {
    let mut raw = scrape_http(url).await?;
    // Auto-detect content type
    raw.content_type = detect_content_type(&raw);
    Ok(raw)
}

/// Heuristic content type detection based on scraped data.
fn detect_content_type(raw: &RawContent) -> ContentType {
    if raw.has_video {
        return ContentType::Video;
    }
    // Collection detection: multiple images + keywords in title/description
    if raw.image_urls.len() > 1 {
        let combined = format!("{} {}", raw.title, raw.text_content);
        let keywords = ["合集", "第", "道", "款", "种", "家常菜", "做法"];
        if keywords.iter().any(|k| combined.contains(*k)) {
            return ContentType::Collection;
        }
    }
    ContentType::Image
}

/// reqwest direct HTTP client with saved Xiaohongshu cookies.
fn xhs_http_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        let jar = super::auth::build_cookie_jar();
        reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
            .redirect(reqwest::redirect::Policy::limited(10))
            .timeout(Duration::from_secs(30))
            .cookie_provider(jar)
            .build()
            .expect("reqwest client build")
    })
}

async fn scrape_http(url: &str) -> Result<RawContent, SourceError> {
    let client = xhs_http_client();

    let resp = client
        .get(url)
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("Referer", "https://www.xiaohongshu.com/")
        .send()
        .await
        .map_err(|e| SourceError::FetchFailed(format!("http get: {}", e)))?;

    if !resp.status().is_success() {
        return Err(SourceError::FetchFailed(format!("HTTP {}", resp.status())));
    }
    // Use final URL after any redirects (e.g. xhslink.com → xiaohongshu.com/explore/...)
    // Normalize to clean canonical URL by extracting the note ID.
    let canonical_url = resp.url().as_str().to_string();
    let source_url = if let Some(note_id) = extract_note_id(&canonical_url) {
        format!("https://www.xiaohongshu.com/explore/{}", note_id)
    } else {
        canonical_url
    };
    let body = resp
        .text()
        .await
        .map_err(|e| SourceError::FetchFailed(format!("http body: {}", e)))?;

    if let Some(raw) = extract_next_data_from_html(&body) {
        if let Ok(parsed) = parse_note_from_state(&raw) {
            if !parsed.title.is_empty() {
                return Ok(RawContent {
                    title: parsed.title,
                    text_content: parsed.description,
                    image_urls: parsed.images,
                    has_video: parsed.has_video,
                    video_url: parsed.video_url,
                    source: "xiaohongshu".into(),
                    source_url: source_url,
                    content_type: Default::default(),
                });
            }
        }
    }
    if let Some(title) = extract_og_title(&body) {
        return Ok(RawContent {
            title,
            text_content: extract_og_description(&body).unwrap_or_default(),
            image_urls: extract_og_image(&body).into_iter().collect(),
            has_video: body.contains("<video") || body.contains("\"type\":\"video\""),
            video_url: None,
            source: "xiaohongshu".into(),
            source_url: source_url,
            content_type: Default::default(),
        });
    }
    Err(SourceError::FetchFailed("无法从 HTML 中提取数据".into()))
}

// ── HTML parsing ────────────────────────────────────────────────

fn extract_next_data_from_html(html: &str) -> Option<serde_json::Value> {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    static RE2: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(r#"<script id="__NEXT_DATA__"[^>]*>(.*?)</script>"#).expect("regex: next data")
    });
    if let Some(cap) = re.captures(html) {
        let raw = cap
            .get(1)?
            .as_str()
            .replace("&quot;", "\"")
            .replace("&amp;", "&");
        return serde_json::from_str::<serde_json::Value>(&raw).ok();
    }
    // Fallback: __INITIAL_STATE__ — captures everything between ={ and }</script>
    // Greedy match because the JSON ends right before </script>.
    let re2 = RE2.get_or_init(|| {
        regex::Regex::new(r"(?s)window\.__INITIAL_STATE__\s*=\s*(\{.*\})\s*;?\s*</script>").expect("regex: initial state")
    });
    if let Some(cap) = re2.captures(html) {
        let raw = cap.get(1)?.as_str();
        // Sanitize JS-specific constructs
        let cleaned = raw.replace(":undefined", ":null");
        return serde_json::from_str::<serde_json::Value>(&cleaned).ok();
    }
    None
}

fn extract_og_title(html: &str) -> Option<String> {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(r#"<meta[^>]*(?:name|property)="og:title"[^>]*content="([^"]*)"[^>]*/?>"#).expect("regex: og:title")
    });
    let t = re.captures(html)?.get(1)?.as_str().to_string();
    Some(t.trim_end_matches(" - 小红书").to_string())
}

fn extract_og_description(html: &str) -> Option<String> {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(r#"<meta[^>]*(?:name|property)="og:description"[^>]*content="([^"]*)"[^>]*/?>"#).expect("regex: og:description")
    });
    Some(re.captures(html)?.get(1)?.as_str().to_string())
}

fn extract_og_image(html: &str) -> Vec<String> {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(r#"<meta[^>]*(?:name|property)="og:image"[^>]*content="([^"]*)"[^>]*/?>"#).expect("regex: og:image")
    });
    re.captures_iter(html)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

fn note_to_pagedata(note: &serde_json::Value) -> PageData {
    PageData {
        title: note
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        description: note
            .get("desc")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        images: note
            .get("imageList")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|i| {
                        i.get("url")
                            .and_then(|u| u.as_str())
                            .or_else(|| {
                                i.get("infoList")
                                    .and_then(|il| il.as_array())
                                    .and_then(|il| il.last())
                                    .and_then(|l| l.get("url").and_then(|u| u.as_str()))
                            })
                            .map(String::from)
                    })
                    .collect()
            })
            .unwrap_or_default(),
        has_video: note.get("video").is_some_and(|v| !v.is_null()),
        video_url: note
            .get("video")
            .and_then(|v| v.get("media"))
            .and_then(|m| m.get("stream"))
            .and_then(|s| {
                // Try master_url/masterUrl first, fall back to backupUrls
                let try_codec = |codec: &str| -> Option<String> {
                    let arr = s.get(codec)?.as_array()?;
                    let first = arr.first()?;
                    // Try both snake_case and camelCase for master URL
                    for key in &["master_url", "masterUrl"] {
                        if let Some(u) = first.get(*key).and_then(|v| v.as_str()) {
                            if !u.is_empty() && u != "??" {
                                return Some(u.to_string());
                            }
                        }
                    }
                    // fallback: backupUrls
                    first.get("backupUrls")
                        .and_then(|b| b.as_array())
                        .and_then(|arr| arr.iter().find_map(|bu| {
                            let u = bu.as_str()?;
                            if !u.is_empty() { Some(u.to_string()) } else { None }
                        }))
                };
                try_codec("h264")
                    .or_else(|| try_codec("h265"))
            })
            .map(String::from),
    }
}

fn parse_note_from_state(state: &serde_json::Value) -> Result<PageData, ()> {
    // INITIAL_STATE structure: state.note.noteDetailMap[firstKey].note
    if let Some(note_wrapper) = state.get("note") {
        if let Some(nd_map) = note_wrapper.get("noteDetailMap") {
            if let Some(obj) = nd_map.as_object() {
                if let Some((_key, nd)) = obj.iter().next() {
                    if let Some(inner_note) = nd.get("note") {
                        if inner_note.get("title").and_then(|t| t.as_str()).map_or(false, |t| !t.is_empty()) {
                            return Ok(note_to_pagedata(inner_note));
                        }
                    }
                }
            }
        }
        // Fallback: note data is directly under state.note
        if note_wrapper.get("title").and_then(|t| t.as_str()).map_or(false, |t| !t.is_empty()) {
            return Ok(note_to_pagedata(note_wrapper));
        }
    }
    for key in &["noteDetail", "noteData", "currentNote"] {
        if let Some(note) = state.get(*key) {
            return Ok(note_to_pagedata(note));
        }
    }
    // Next.js explore page: __NEXT_DATA__ nests note under props.pageProps.*
    if let Some(page_props) = state
        .get("props")
        .and_then(|p| p.get("pageProps"))
    {
        for key in &["noteData", "noteDetail", "currentNote", "note"] {
            if let Some(note) = page_props.get(key) {
                if note.get("title").and_then(|t| t.as_str()).map_or(false, |t| !t.is_empty()) {
                    return Ok(note_to_pagedata(note));
                }
            }
        }
        // Some pages nest the actual note inside noteData.note
        if let Some(note_data) = page_props.get("noteData") {
            if let Some(note) = note_data.get("note") {
                if note.get("title").and_then(|t| t.as_str()).map_or(false, |t| !t.is_empty()) {
                    return Ok(note_to_pagedata(note));
                }
            }
        }
    }
    // Debug: log available top-level keys when all lookups fail
    if let Some(obj) = state.as_object() {
        let keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
        crate::vprintln!("  ⚠ __NEXT_DATA__ 顶层键: {:?}", keys);
    }
    Err(())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageData {
    title: String,
    description: String,
    images: Vec<String>,
    has_video: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    video_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_next_data_from_html() {
        let html = r#"<script id="__NEXT_DATA__" type="application/json">{"page":"/explore/[id]"}</script>"#;
        assert_eq!(
            extract_next_data_from_html(html).unwrap()["page"]
                .as_str()
                .unwrap(),
            "/explore/[id]"
        );
    }

    #[test]
    fn test_extract_initial_state_multiline() {
        let html = "<script>window.__INITIAL_STATE__={\"note\":{\n  \"title\":\"测试标题\",\n  \"desc\":\"描述内容\"\n},\"user\":{}};</script>";
        let val = extract_next_data_from_html(html).unwrap();
        assert_eq!(val["note"]["title"].as_str().unwrap(), "测试标题");
    }

    #[test]
    fn test_extract_og_title_property() {
        let html = r#"<meta property="og:title" content="蒜香椒盐烤排骨" />"#;
        assert_eq!(extract_og_title(html).unwrap(), "蒜香椒盐烤排骨");
    }

    #[test]
    fn test_extract_og_title_name() {
        let html = r#"<meta name="og:title" content="红烧肉 - 小红书" />"#;
        assert_eq!(extract_og_title(html).unwrap(), "红烧肉");
    }

    #[test]
    fn test_parse_note_from_state() {
        let s = serde_json::json!({"note":{"title":"红烧肉","desc":"做法","imageList":[{"url":"https://x.com/1.jpg"}],"video":{"id":"xxx"},"type":"video"}});
        let p = parse_note_from_state(&s).unwrap();
        assert_eq!(p.title, "红烧肉");
        assert!(p.has_video);
        let s2 = serde_json::json!({"note":{"title":"菜饭","desc":"做法","imageList":[{"url":"https://x.com/1.jpg"},{"url":"https://x.com/2.jpg"}],"type":"video"}});
        let p2 = parse_note_from_state(&s2).unwrap();
        assert!(!p2.has_video);
    }

    #[test]
    fn test_parse_note_from_state_nested_page_props() {
        // Simulate __NEXT_DATA__ from explore page: props.pageProps.noteData
        let s = serde_json::json!({
            "props": {
                "pageProps": {
                    "noteData": {
                        "title": "番茄炒蛋",
                        "desc": "这大概是最好吃的番茄炒蛋了",
                        "imageList": [{"url": "https://x.com/1.jpg"}]
                    }
                }
            },
            "page": "/explore/672acc30000000003c01438b"
        });
        let p = parse_note_from_state(&s).unwrap();
        assert_eq!(p.title, "番茄炒蛋");
        assert_eq!(p.description, "这大概是最好吃的番茄炒蛋了");
        assert!(!p.has_video);
    }

    #[test]
    fn test_parse_note_from_state_deeply_nested() {
        // Nested: props.pageProps.noteData.note
        let s = serde_json::json!({
            "props": {
                "pageProps": {
                    "noteData": {
                        "note": {
                            "title": "糖醋排骨",
                            "desc": "酸甜可口"
                        }
                    }
                }
            }
        });
        let p = parse_note_from_state(&s).unwrap();
        assert_eq!(p.title, "糖醋排骨");
        assert_eq!(p.description, "酸甜可口");
    }
}
