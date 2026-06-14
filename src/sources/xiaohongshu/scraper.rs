/// Xiaohongshu page scraper using reqwest HTTP (no browser automation).
///
/// Strategy:
/// 1. __NEXT_DATA__ JSON via HTML parsing (primary)
/// 2. OG meta tags via HTML parsing (fallback)
use crate::models::RawContent;
use super::super::SourceError;
use std::time::Duration;

pub async fn scrape(url: &str) -> Result<RawContent, SourceError> {
    scrape_http(url).await
}

/// reqwest direct HTTP client
fn xhs_http_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
            .redirect(reqwest::redirect::Policy::limited(10))
            .timeout(Duration::from_secs(30))
            .cookie_store(true)
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
                    source_url: url.to_string(),
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
            source_url: url.to_string(),
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
    let re2 = RE2.get_or_init(|| {
        regex::Regex::new(r"(?s)window\.__INITIAL_STATE__\s*=\s*(\{.*?\});").expect("regex: initial state")
    });
    if let Some(cap) = re2.captures(html) {
        return serde_json::from_str::<serde_json::Value>(cap.get(1)?.as_str()).ok();
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
                s.get("h264")
                    .and_then(|a| a.as_array())
                    .and_then(|a| a.first())
                    .and_then(|e| e.get("master_url").and_then(|u| u.as_str()))
                    .or_else(|| {
                        s.get("h265")
                            .and_then(|a| a.as_array())
                            .and_then(|a| a.first())
                            .and_then(|e| e.get("master_url").and_then(|u| u.as_str()))
                    })
            })
            .map(String::from),
    }
}

fn parse_note_from_state(state: &serde_json::Value) -> Result<PageData, ()> {
    if let Some(note) = state.get("note") {
        return Ok(note_to_pagedata(note));
    }
    for key in &["noteDetail", "noteData", "currentNote"] {
        if let Some(note) = state.get(*key) {
            return Ok(note_to_pagedata(note));
        }
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
}
