/// Xiaohongshu URL handling: detection, short URL resolution, note ID extraction.
use regex::Regex;

/// Check if a URL is a Xiaohongshu URL.
pub fn is_xhs_url(url: &str) -> bool {
    let patterns = [
        Regex::new(r"xiaohongshu\.com/explore/[a-f0-9]+").expect("static regex"),
        Regex::new(r"xiaohongshu\.com/discovery/item/[a-f0-9]+").expect("static regex"),
        Regex::new(r"xhslink\.com/\w+").expect("static regex"),
    ];
    patterns.iter().any(|p| p.is_match(url))
}

/// Extract note ID from a resolved Xiaohongshu URL.
pub fn extract_note_id(url: &str) -> Option<String> {
    for pattern in &[
        Regex::new(r"/explore/([a-f0-9]+)").expect("static regex"),
        Regex::new(r"/discovery/item/([a-f0-9]+)").expect("static regex"),
    ] {
        if let Some(caps) = pattern.captures(url) {
            return Some(caps[1].to_string());
        }
    }
    // Also try from redirectPath (URL-encoded path) in login/error URLs
    if let Some(caps) = Regex::new(r"redirectPath=.*?%2Fexplore%2F([a-f0-9]+)")
        .ok()
        .and_then(|r| r.captures(url))
    {
        return Some(caps[1].to_string());
    }
    None
}

/// Resolve an xhslink.com short URL using zendriver-rs (needs JS redirect).
pub async fn resolve_short_url(url: &str) -> Result<String, String> {
    if !url.contains("xhslink.com") {
        return Ok(url.to_string());
    }

    let browser = zendriver::Browser::builder()
        .headless(true)
        .lang(String::from("zh-CN"))
        .launch()
        .await
        .map_err(|e| format!("启动浏览器失败: {}", e))?;

    let tab = browser.main_tab();
    tab.goto(url)
        .await
        .map_err(|e| format!("页面加载失败: {}", e))?;

    // Small delay to let redirect complete
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    let resolved = tab
        .url()
        .await
        .map_err(|e| format!("获取 URL 失败: {}", e))?
        .to_string();

    browser.close().await.ok();

    if resolved == url {
        return Err("短链解析失败: URL 未变化".into());
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_xhs_url_explore() {
        assert!(is_xhs_url("https://www.xiaohongshu.com/explore/abc123"));
    }

    #[test]
    fn test_is_xhs_url_discovery() {
        assert!(is_xhs_url("https://www.xiaohongshu.com/discovery/item/abc123"));
    }

    #[test]
    fn test_is_xhs_url_short() {
        assert!(is_xhs_url("https://xhslink.com/o/test123"));
    }

    #[test]
    fn test_is_not_xhs_url() {
        assert!(!is_xhs_url("https://example.com"));
    }

    #[test]
    fn test_extract_note_id_explore() {
        assert_eq!(
            extract_note_id("https://www.xiaohongshu.com/explore/abc123"),
            Some("abc123".into())
        );
    }

    #[test]
    fn test_extract_note_id_from_redirect_path() {
        let url = "https://www.xiaohongshu.com/website-login/error?redirectPath=/explore/abc123def456";
        assert_eq!(
            extract_note_id(url),
            Some("abc123def456".into())
        );
    }

    #[test]
    fn test_extract_note_id_no_match() {
        assert_eq!(extract_note_id("https://example.com"), None);
    }
}
