/// Xiaohongshu page scraper using zendriver-rs (stealth-by-default).
///
/// Strategy:
/// 1. zendriver-rs with stealth (primary — replaces Python bridge + playwright-rs)
/// 2. reqwest direct HTTP (fallback — fast, no browser needed)
use crate::models::RawContent;
use super::super::SourceError;
use std::time::Duration;

pub async fn scrape(url: &str) -> Result<RawContent, SourceError> {
    // Require saved cookies — login is a separate step, extract does not auto-login.
    if !super::auth::has_cookies() {
        return Err(SourceError::FetchFailed(
            "未找到登录缓存，请先执行: xhs-recipe login".into(),
        ));
    }

    // 1. zendriver-rs (stealth built-in, replaces Python bridge entirely)
    match scrape_zendriver(url).await {
        Ok(raw) if is_valid(&raw.title) => return Ok(raw),
        Ok(_) => {}  // fall through to HTTP fallback
        Err(ref e) if is_auth_error(e) => return Err(e.clone()),
        Err(_) => {}  // fall through to HTTP fallback
    }
    // 2. reqwest direct HTTP (fastest path when it works)
    scrape_http(url).await
}

fn is_auth_error(err: &SourceError) -> bool {
    matches!(err, SourceError::FetchFailed(msg) if msg.contains("需要登录") || msg.contains("登录才能"))
}

fn is_valid(title: &str) -> bool {
    !title.is_empty()
        && title != "安全限制"
        && title != "403"
        && title != "当前笔记暂时无法浏览"
        && title != "手机号登录"
        && title != "登录"
        && title != "小红书"
}

// ── 1. zendriver-rs (stealth built-in) ───────────────────────────────

async fn scrape_zendriver(url: &str) -> Result<RawContent, SourceError> {
    let browser = zendriver::Browser::builder()
        .headless(true)
        .lang(String::from("zh-CN"))
        .launch()
        .await
        .map_err(|e| SourceError::FetchFailed(format!("zendriver launch: {}", e)))?;

    let jar = browser.cookies();

    // Load saved cookies before navigation
    let saved = super::auth::load_cookies();
    if !saved.is_empty() {
        if let Err(e) = jar.set_many(saved).await {
            println!("  ⚠ 设置 Cookie 失败: {}", e);
        } else {
            crate::vprintln!("  ✓ 已设置 Cookie ({} 个)", jar.all().await.map(|c| c.len()).unwrap_or(0));
        }
    }

    let tab = browser.main_tab();

    tab.goto(url)
        .await
        .map_err(|e| SourceError::FetchFailed(format!("zendriver goto: {}", e)))?;
    tab.wait_for_load().await.ok();
    crate::vprintln!("  ✓ 页面加载完成");

    // Short delay for dynamic content
    tokio::time::sleep(Duration::from_secs(2)).await;

    let (title, desc, images, has_video) = extract_data(&tab).await?;

    if title.is_empty() || title == "手机号登录" || title == "登录" || title == "小红书" {
        browser.close().await.ok();
        return Err(SourceError::FetchFailed("需要登录才能查看内容".into()));
    }
    if title == "当前笔记暂时无法浏览" {
        browser.close().await.ok();
        return Err(SourceError::FetchFailed(
            "当前笔记暂时无法浏览（IP 被限流，尝试扫码登录或更换网络）".into(),
        ));
    }

    browser.close().await.ok();

    Ok(RawContent {
        title,
        text_content: desc,
        image_urls: images,
        has_video,
        video_url: None,
        source: "xiaohongshu".into(),
        source_url: url.to_string(),
    })
}

// ── Page data extraction ─────────────────────────────────────────────

async fn extract_data(
    tab: &zendriver::Tab,
) -> Result<(String, String, Vec<String>, bool), SourceError> {
    // Try __NEXT_DATA__ via evaluate_main
    let js = r#"(()=>{try{const el=document.getElementById('__NEXT_DATA__');if(el)return el.textContent;}catch(e){}try{if(window.__INITIAL_STATE__)return JSON.stringify(window.__INITIAL_STATE__);}catch(e){}return null;})()"#;

    let json_str: Option<String> = tab.evaluate_main(js).await.ok();

    if let Some(ref s) = json_str {
        if let Some(parsed) = parse_next_data(s) {
            return Ok(parsed);
        }
    }
    const DOM_EXTRACT_JS: &str = r#"(()=>{const r={title:'',description:'',images:[],hasVideo:false};const note=document.querySelector('#noteContainer')||document.querySelector('[class*="note"]');const og=document.querySelector('meta[property="og:title"]');if(og)r.title=og.getAttribute('content')||'';const od=document.querySelector('meta[property="og:description"]');if(od)r.description=od.getAttribute('content')||'';if(!r.title){const scope=note||document;for(const s of['#detail-title','.title','h1.title','[class*="title"]']){const e=scope.querySelector(s);if(e&&e.innerText){r.title=e.innerText.trim();break;}}}const scope=note||document;const seen=new Set();const com=scope.querySelector('.comments-el');scope.querySelectorAll('img').forEach(i=>{if(com&&com.contains(i))return;const src=i.getAttribute('src')||i.getAttribute('data-src')||'';if(src&&src.includes('https://')&&!seen.has(src)&&!src.includes('avatar')&&!src.includes('icon')&&!src.includes('emoji')){r.images.push(src);seen.add(src);}});r.hasVideo=!!(scope.querySelector('video')||scope.querySelector('[class*="player"]'));return JSON.stringify(r);})()"#;

    let dom_json: String = tab
        .evaluate_main(DOM_EXTRACT_JS)
        .await
        .map_err(|e| SourceError::FetchFailed(format!("DOM extract: {}", e)))?;

    let data: PageData = serde_json::from_str(&dom_json)
        .map_err(|e| SourceError::FetchFailed(format!("DOM JSON: {}", e)))?;

    if !data.title.is_empty() {
        return Ok((data.title, data.description, data.images, data.has_video));
    }

    Ok((String::new(), String::new(), vec![], false))
}

fn parse_next_data(json_str: &str) -> Option<(String, String, Vec<String>, bool)> {
    if json_str.is_empty() || json_str == "null" || json_str == "undefined" {
        return None;
    }
    let val: serde_json::Value = serde_json::from_str(json_str).ok()?;
    parse_note_from_state(&val)
        .ok()
        .map(|pd| (pd.title, pd.description, pd.images, pd.has_video))
}

// ── 2. reqwest direct HTTP (fallback) ────────────────────────────────

async fn scrape_http(url: &str) -> Result<RawContent, SourceError> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(Duration::from_secs(30))
        .cookie_store(true)
        .build()
        .map_err(|e| SourceError::FetchFailed(format!("http client: {}", e)))?;

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
                    video_url: None,
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

// ── HTML parsing (shared) ────────────────────────────────────────────

fn extract_next_data_from_html(html: &str) -> Option<serde_json::Value> {
    let re =
        regex::Regex::new(r#"<script id="__NEXT_DATA__"[^>]*>(.*?)</script>"#).ok()?;
    if let Some(cap) = re.captures(html) {
        let raw = cap
            .get(1)?
            .as_str()
            .replace("&quot;", "\"")
            .replace("&amp;", "&");
        return serde_json::from_str::<serde_json::Value>(&raw).ok();
    }
    let re2 = regex::Regex::new(r#"window\.__INITIAL_STATE__\s*=\s*({.*?});"#).ok()?;
    if let Some(cap) = re2.captures(html) {
        return serde_json::from_str::<serde_json::Value>(cap.get(1)?.as_str()).ok();
    }
    None
}

fn extract_og_title(html: &str) -> Option<String> {
    let re =
        regex::Regex::new(r#"<meta[^>]*property="og:title"[^>]*content="([^"]*)"[^>]*/?>"#).ok()?;
    Some(re.captures(html)?.get(1)?.as_str().to_string())
}

fn extract_og_description(html: &str) -> Option<String> {
    let re = regex::Regex::new(
        r#"<meta[^>]*property="og:description"[^>]*content="([^"]*)"[^>]*/?>"#,
    )
    .ok()?;
    Some(re.captures(html)?.get(1)?.as_str().to_string())
}

fn extract_og_image(html: &str) -> Vec<String> {
    if let Some(re) =
        regex::Regex::new(r#"<meta[^>]*property="og:image"[^>]*content="([^"]*)"[^>]*/?>"#).ok()
    {
        re.captures_iter(html)
            .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
            .collect()
    } else {
        vec![]
    }
}

fn parse_note_from_state(state: &serde_json::Value) -> Result<PageData, ()> {
    if let Some(note) = state.get("note") {
        return Ok(PageData {
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
            has_video: note.get("type").and_then(|v| v.as_str()) == Some("video")
                || note.get("video").is_some(),
        });
    }
    for key in &["noteDetail", "noteData", "currentNote"] {
        if let Some(note) = state.get(*key) {
            return Ok(PageData {
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
                                            .and_then(|l| {
                                                l.get("url").and_then(|u| u.as_str())
                                            })
                                    })
                                    .map(String::from)
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                has_video: note.get("type").and_then(|v| v.as_str()) == Some("video")
                    || note.get("video").is_some(),
            });
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
    fn test_extract_og_title() {
        let html = r#"<meta property="og:title" content="蒜香椒盐烤排骨" />"#;
        assert_eq!(extract_og_title(html).unwrap(), "蒜香椒盐烤排骨");
    }

    #[test]
    fn test_parse_note_from_state() {
        let s = serde_json::json!({"note":{"title":"红烧肉","desc":"做法","imageList":[{"url":"https://x.com/1.jpg"}],"type":"video"}});
        let p = parse_note_from_state(&s).unwrap();
        assert_eq!(p.title, "红烧肉");
        assert!(p.has_video);
    }
}
