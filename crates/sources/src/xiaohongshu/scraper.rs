/// Xiaohongshu page scraper.
///
/// Strategy (fastest first):
/// 1. reqwest GET → extract __NEXT_DATA__ from HTML (no browser needed)
/// 2. playwright-rs → goto + evaluate (official Playwright, best anti-detection)
use core::RawContent;
use crate::SourceError;
use playwright::{BrowserContextOptions, GotoOptions, LaunchOptions, Playwright, Viewport, WaitUntil};
use std::time::Duration;

pub async fn scrape(url: &str, note_id: &str) -> Result<RawContent, SourceError> {
    // 1. Python bridge (proven — exact same code as `xhs-recipe extract`)
    //    Must go first to avoid cumulative request rate-limiting from other methods.
    match scrape_python_bridge(url, note_id).await {
        Ok(raw) if is_valid(&raw.title) => return Ok(raw),
        Ok(_) => println!("  ⚠ Python bridge 返回无效内容"), // shouldn't happen
        Err(ref e) => println!("  ⚠ Python bridge 失败 ({}), 尝试 playwright-rs...", e),
    }
    // 2. playwright-rs (native Rust, no Python dependency)
    match scrape_playwright(url, note_id).await {
        Ok(raw) if is_valid(&raw.title) => return Ok(raw),
        Ok(_) | Err(_) => println!("  ⚠ playwright-rs 失败，尝试 HTTP..."),
    }
    // 3. reqwest direct HTTP (fastest, but most likely blocked)
    scrape_http(url, note_id).await
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

// ── 1. reqwest direct HTTP ────────────────────────────────────────

async fn scrape_http(url: &str, _note_id: &str) -> Result<RawContent, SourceError> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(Duration::from_secs(30))
        .cookie_store(true)
        .build()
        .map_err(|e| SourceError::FetchFailed(format!("http client: {}", e)))?;

    let resp = client
        .get(url)
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("Referer", "https://www.xiaohongshu.com/")
        .send()
        .await
        .map_err(|e| SourceError::FetchFailed(format!("http get: {}", e)))?;

    if !resp.status().is_success() {
        return Err(SourceError::FetchFailed(format!("HTTP {}", resp.status())));
    }
    let body = resp.text().await
        .map_err(|e| SourceError::FetchFailed(format!("http body: {}", e)))?;

    if let Some(raw) = extract_next_data_from_html(&body) {
        if let Ok(parsed) = parse_note_from_state(&raw) {
            if !parsed.title.is_empty() {
                println!("  ✓ 从 HTML __NEXT_DATA__ 提取数据 (HTTP)");
                return Ok(RawContent {
                    title: parsed.title, text_content: parsed.description,
                    image_urls: parsed.images, has_video: parsed.has_video,
                    video_url: None, source: "xiaohongshu".into(), source_url: url.to_string(),
                });
            }
        }
    }
    if let Some(title) = extract_og_title(&body) {
        return Ok(RawContent {
            title, text_content: extract_og_description(&body).unwrap_or_default(),
            image_urls: extract_og_image(&body).into_iter().collect(),
            has_video: body.contains("<video") || body.contains("\"type\":\"video\""),
            video_url: None, source: "xiaohongshu".into(), source_url: url.to_string(),
        });
    }
    Err(SourceError::FetchFailed("无法从 HTML 中提取数据".into()))
}

// ── 2. playwright-rs ─────────────────────────────────────────────

const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36";

async fn scrape_playwright(url: &str, _note_id: &str) -> Result<RawContent, SourceError> {
    super::url::ensure_driver_path(); // auto-detect driver path before launch
    let pw = Playwright::launch()
        .await
        .map_err(|e| SourceError::FetchFailed(format!("PW init: {}", e)))?;

    let chrome = super::url::chrome_path();
    let mut opts = LaunchOptions::default()
        .headless(true)
        .args(vec!["--no-sandbox".into()]);
    if !chrome.is_empty() {
        opts = opts.executable_path(chrome);
    }

    // Step A: check for cookies. If none, auto-fetch from login page.
    let saved = get_or_fetch_cookies(&pw).await?;

    // Step B: launch browser with cookies
    let browser = pw.chromium().launch_with_options(opts.clone()).await
        .map_err(|e| SourceError::FetchFailed(format!("PW launch: {}", e)))?;

    let context = browser.new_context_with_options(
        BrowserContextOptions::builder()
            .user_agent(UA.into())
            .locale("zh-CN".into())
            .viewport(Viewport { width: 1280, height: 800 })
            .build(),
    ).await
        .map_err(|e| SourceError::FetchFailed(format!("PW context: {}", e)))?;

    // Anti-detection
    context.add_init_script(
        r#"Object.defineProperty(navigator,'webdriver',{get:()=>undefined});"#,
    ).await.ok();
    context.add_init_script(r#"window.chrome={runtime:{}};"#).await.ok();

    // Set all cookies BEFORE any navigation
    if !saved.is_empty() {
        context.add_cookies(&saved).await.ok();
        println!("  ✓ 已设置 Cookie ({} 个)", saved.len());
    }

    println!("  ↓ 加载页面...");
    let page = context.new_page().await
        .map_err(|e| SourceError::FetchFailed(format!("PW page: {}", e)))?;

    let note_opts = GotoOptions::new()
        .wait_until(WaitUntil::DomContentLoaded)
        .timeout(Duration::from_secs(30));
    page.goto(url, Some(note_opts)).await
        .map_err(|e| SourceError::FetchFailed(format!("PW goto: {}", e)))?;
    println!("  ✓ 页面 DOM 加载完成");

    tokio::time::sleep(Duration::from_secs(2)).await;

    let (title, desc, images, has_video) = extract_data(&page).await?;

    if title.is_empty() || title == "手机号登录" || title == "登录" || title == "小红书" {
        return Err(SourceError::FetchFailed("需要登录才能查看内容".into()));
    }
    if title == "当前笔记暂时无法浏览" {
        return Err(SourceError::FetchFailed("当前笔记暂时无法浏览（IP 被限流，尝试扫码登录或更换网络）".into()));
    }

    println!("  ✓ 标题: {}", title);
    println!("  ✓ 类型: {}", if has_video { "视频笔记" } else { "图文笔记" });
    println!("  ✓ 图片: {} 张", images.len());

    Ok(RawContent {
        title, text_content: desc, image_urls: images, has_video,
        video_url: None, source: "xiaohongshu".into(), source_url: url.to_string(),
    })
}

// ── Cookie management ────────────────────────────────────────────

/// Get or auto-fetch session cookies (matching Python fetch_cookies_from_login + cookie loading).
async fn get_or_fetch_cookies(pw: &Playwright) -> Result<Vec<playwright::Cookie>, SourceError> {
    // 1. Check env var XHS_COOKIE
    if let Ok(raw) = std::env::var("XHS_COOKIE") {
        let cookies: Vec<playwright::Cookie> = raw.split(';')
            .filter_map(|part| {
                let part = part.trim();
                part.split_once('=').map(|(n, v)| playwright::Cookie {
                    name: n.trim().to_string(),
                    value: v.trim().to_string(),
                    domain: ".xiaohongshu.com".into(), path: "/".into(),
                    expires: -1.0, http_only: false, secure: true, same_site: None,
                })
            })
            .collect();
        if !cookies.is_empty() {
            return Ok(cookies);
        }
    }

    // 2. Check saved cookie file
    let saved = super::auth::load_cookies();
    if !saved.is_empty() {
        let cookies: Vec<playwright::Cookie> = saved.into_iter().map(|c| playwright::Cookie {
            name: c.name, value: c.value,
            // Normalize domain to .xiaohongshu.com for subdomain matching
            // Python: cookie_to_header() loses exact domains, scraper re-adds .xiaohongshu.com
            domain: if c.domain.starts_with('.') || c.domain == "www.xiaohongshu.com" {
                ".xiaohongshu.com".into()
            } else {
                c.domain.clone()
            },
            path: c.path,
            expires: -1.0, http_only: false, secure: true, same_site: None,
        }).collect();
        println!("  ✓ 使用已保存的 Cookie ({} 个)", cookies.len());
        return Ok(cookies);
    }

    // 3. Auto-fetch from login page
    println!("  ↓ 尝试自动扫码登录获取 Cookie...");
    match fetch_cookies_from_login(pw).await? {
        Some(cookies) => {
            println!("  ✓ 已获取 Cookie ({} 个)", cookies.len());
            Ok(cookies)
        }
        None => Ok(vec![]),
    }
}

/// Visit login page headlessly, capture session cookies, save to file.
/// Exact equivalent of Python `fetch_cookies_from_login()`.
async fn fetch_cookies_from_login(pw: &Playwright) -> Result<Option<Vec<playwright::Cookie>>, SourceError> {
    super::url::ensure_driver_path();
    let chrome = super::url::chrome_path();
    let mut opts = LaunchOptions::default()
        .headless(true)
        .args(vec!["--no-sandbox".into()]);
    if !chrome.is_empty() {
        opts = opts.executable_path(chrome);
    }

    let browser = pw.chromium().launch_with_options(opts).await
        .map_err(|e| SourceError::FetchFailed(format!("PW launch: {}", e)))?;

    let context = browser.new_context_with_options(
        BrowserContextOptions::builder()
            .user_agent(UA.into())
            .locale("zh-CN".into())
            .viewport(Viewport { width: 1280, height: 800 })
            .build(),
    ).await
        .map_err(|e| SourceError::FetchFailed(format!("PW context: {}", e)))?;

    let page = context.new_page().await
        .map_err(|e| SourceError::FetchFailed(format!("PW page: {}", e)))?;

    // Match Python: wait_until="networkidle", timeout=30000
    let login_opts = GotoOptions::new()
        .wait_until(WaitUntil::NetworkIdle)
        .timeout(Duration::from_secs(30));
    page.goto("https://www.xiaohongshu.com/login", Some(login_opts)).await
        .map_err(|e| SourceError::FetchFailed(format!("PW goto login: {}", e)))?;

    // Wait for cookies to settle (Python: wait_for_timeout(3000))
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Capture cookies from context (Python: await context.cookies())
    let cookies = context.cookies(None).await
        .map_err(|e| SourceError::FetchFailed(format!("get cookies: {}", e)))?;

    if cookies.is_empty() {
        return Ok(None);
    }

    // Save to file (Python: COOKIE_FILE.write_text)
    super::auth::save_cookies(&cookies);

    // Convert to playwright Cookie type, normalize domain
    let pw_cookies: Vec<playwright::Cookie> = cookies.into_iter().map(|c| playwright::Cookie {
        name: c.name, value: c.value,
        // Normalize to .xiaohongshu.com for subdomain matching (Python behavior)
        domain: if c.domain.starts_with('.') || c.domain == "www.xiaohongshu.com" {
            ".xiaohongshu.com".into()
        } else {
            c.domain.clone()
        },
        path: c.path,
        expires: -1.0, http_only: false, secure: true, same_site: None,
    }).collect();
    Ok(Some(pw_cookies))
}

// ── Page data extraction ─────────────────────────────────────────

async fn extract_data(page: &playwright::Page) -> Result<(String, String, Vec<String>, bool), SourceError> {
    // Try __NEXT_DATA__ via evaluate_value
    let js = r#"(()=>{try{const el=document.getElementById('__NEXT_DATA__');if(el)return el.textContent;}catch(e){}try{if(window.__INITIAL_STATE__)return JSON.stringify(window.__INITIAL_STATE__);}catch(e){}return null;})()"#;

    if let Ok(json_str) = page.evaluate_value(js).await {
        if let Some(parsed) = parse_next_data(&json_str) {
            println!("  ✓ 从 __NEXT_DATA__ 提取数据 (PW)");
            return Ok(parsed);
        }
    }

    // Fallback to DOM extraction
    println!("  ↓ __NEXT_DATA__ 未找到，尝试 DOM 提取...");
    let dom_js = r#"(()=>{const r={title:'',description:'',images:[],hasVideo:false};const og=document.querySelector('meta[property="og:title"]');if(og)r.title=og.getAttribute('content')||'';const od=document.querySelector('meta[property="og:description"]');if(od)r.description=od.getAttribute('content')||'';if(!r.title){for(const s of['#detail-title','.title','h1.title','[class*="title"]']){const e=document.querySelector(s);if(e&&e.innerText){r.title=e.innerText.trim();break;}}}const seen=new Set();for(const s of['.swiper-slide img','.carousel img','.note-image img']){document.querySelectorAll(s).forEach(i=>{const src=i.getAttribute('src')||i.getAttribute('data-src')||'';if(src&&src.includes('http')&&!seen.has(src)){r.images.push(src);seen.add(src);}});}r.hasVideo=!!document.querySelector('video');return JSON.stringify(r);})()"#;

    if let Ok(json_str) = page.evaluate_value(dom_js).await {
        if let Ok(data) = serde_json::from_str::<PageData>(&json_str) {
            if !data.title.is_empty() {
                println!("  ✓ 从 DOM 中提取数据 (PW)");
                return Ok((data.title, data.description, data.images, data.has_video));
            }
        }
    }
    Ok((String::new(), String::new(), vec![], false))
}

fn parse_next_data(json_str: &str) -> Option<(String, String, Vec<String>, bool)> {
    if json_str.is_empty() || json_str == "null" || json_str == "undefined" {
        return None;
    }
    let val: serde_json::Value = serde_json::from_str(json_str).ok()?;
    parse_note_from_state(&val).ok().map(|pd| (pd.title, pd.description, pd.images, pd.has_video))
}

// ── HTML parsing ─────────────────────────────────────────────────

fn extract_next_data_from_html(html: &str) -> Option<serde_json::Value> {
    let re = regex::Regex::new(r#"<script id="__NEXT_DATA__"[^>]*>(.*?)</script>"#).ok()?;
    if let Some(cap) = re.captures(html) {
        let raw = cap.get(1)?.as_str().replace("&quot;", "\"").replace("&amp;", "&");
        return serde_json::from_str::<serde_json::Value>(&raw).ok();
    }
    let re2 = regex::Regex::new(r#"window\.__INITIAL_STATE__\s*=\s*({.*?});"#).ok()?;
    if let Some(cap) = re2.captures(html) {
        return serde_json::from_str::<serde_json::Value>(cap.get(1)?.as_str()).ok();
    }
    None
}

fn extract_og_title(html: &str) -> Option<String> {
    let re = regex::Regex::new(r#"<meta[^>]*property="og:title"[^>]*content="([^"]*)"[^>]*/?>"#).ok()?;
    Some(re.captures(html)?.get(1)?.as_str().to_string())
}

fn extract_og_description(html: &str) -> Option<String> {
    let re = regex::Regex::new(r#"<meta[^>]*property="og:description"[^>]*content="([^"]*)"[^>]*/?>"#).ok()?;
    Some(re.captures(html)?.get(1)?.as_str().to_string())
}

fn extract_og_image(html: &str) -> Vec<String> {
    if let Some(re) = regex::Regex::new(r#"<meta[^>]*property="og:image"[^>]*content="([^"]*)"[^>]*/?>"#).ok() {
        re.captures_iter(html).filter_map(|c| c.get(1).map(|m| m.as_str().to_string())).collect()
    } else { vec![] }
}

fn parse_note_from_state(state: &serde_json::Value) -> Result<PageData, ()> {
    if let Some(note) = state.get("note") {
        return Ok(PageData {
            title: note.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            description: note.get("desc").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            images: note.get("imageList").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|i| i.get("url").and_then(|u| u.as_str()).map(String::from)).collect()).unwrap_or_default(),
            has_video: note.get("type").and_then(|v| v.as_str()) == Some("video"),
        });
    }
    for key in &["noteDetail", "noteData", "currentNote"] {
        if let Some(note) = state.get(*key) {
            return Ok(PageData {
                title: note.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                description: note.get("desc").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                images: note.get("imageList").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|i| {
                    i.get("url").and_then(|u| u.as_str()).or_else(|| i.get("infoList").and_then(|il| il.as_array()).and_then(|il| il.last()).and_then(|l| l.get("url").and_then(|u| u.as_str()))).map(String::from)
                }).collect()).unwrap_or_default(),
                has_video: note.get("type").and_then(|v| v.as_str()) == Some("video") || note.get("video").is_some(),
            });
        }
    }
    Err(())
}

// ── Python bridge fallback ──────────────────────────────────────

async fn scrape_python_bridge(url: &str, _note_id: &str) -> Result<RawContent, SourceError> {
    let python = find_python()?;
    let script = find_bridge_script()?;
    let url = url.to_string();

    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new(&python).args([&script, &url]).output()
    })
    .await
    .map_err(|e| SourceError::FetchFailed(format!("task: {}", e)))?
    .map_err(|e| SourceError::FetchFailed(format!("subprocess: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = stderr.trim().to_string();
        let msg_lower = msg.to_lowercase();
        if msg_lower.contains("permission") || msg.contains("需要登录") {
            return Err(SourceError::FetchFailed("需要登录才能查看内容".into()));
        }
        return Err(SourceError::FetchFailed(msg));
    }

    serde_json::from_str::<RawContent>(&String::from_utf8_lossy(&output.stdout))
        .map_err(|e| SourceError::ParseFailed(format!("JSON: {}", e)))
}

fn find_python() -> Result<String, SourceError> {
    for name in &["python3", "python"] {
        if std::process::Command::new(name).arg("--version").output().is_ok() {
            return Ok(name.to_string());
        }
    }
    Err(SourceError::FetchFailed("Python 3 not found".into()))
}

fn find_bridge_script() -> Result<String, SourceError> {
    for p in &["scripts/fetch_raw.py", "../scripts/fetch_raw.py"] {
        if std::path::Path::new(p).exists() {
            return Ok(p.to_string());
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut probe = exe; probe.pop();
        for _ in 0..4 {
            let candidate = probe.join("scripts").join("fetch_raw.py");
            if candidate.exists() { return Ok(candidate.to_string_lossy().to_string()); }
            probe.pop();
        }
    }
    Err(SourceError::FetchFailed("scripts/fetch_raw.py not found".into()))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageData {
    title: String, description: String, images: Vec<String>, has_video: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_next_data_from_html() {
        let html = r#"<script id="__NEXT_DATA__" type="application/json">{"page":"/explore/[id]"}</script>"#;
        assert_eq!(extract_next_data_from_html(html).unwrap()["page"].as_str().unwrap(), "/explore/[id]");
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
