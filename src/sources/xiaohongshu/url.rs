/// Xiaohongshu URL handling: detection, short URL resolution, note ID extraction.
use regex::Regex;
use std::sync::OnceLock;
use std::time::Duration;

/// Check if a URL is a Xiaohongshu URL.
pub fn is_xhs_url(url: &str) -> bool {
    static PATTERNS: OnceLock<[Regex; 3]> = OnceLock::new();
    let patterns = PATTERNS.get_or_init(|| [
        Regex::new(r"xiaohongshu\.com/explore/[a-f0-9]+").expect("regex: explore URL"),
        Regex::new(r"xiaohongshu\.com/discovery/item/[a-f0-9]+").expect("regex: discovery URL"),
        Regex::new(r"xhslink\.com/\w+").expect("regex: short URL"),
    ]);
    patterns.iter().any(|p| p.is_match(url))
}

/// Extract note ID from a resolved Xiaohongshu URL.
pub fn extract_note_id(url: &str) -> Option<String> {
    static PATTERNS: OnceLock<[Regex; 2]> = OnceLock::new();
    let patterns = PATTERNS.get_or_init(|| [
        Regex::new(r"/explore/([a-f0-9]+)").expect("regex: extract explore ID"),
        Regex::new(r"/discovery/item/([a-f0-9]+)").expect("regex: extract discovery ID"),
    ]);
    for pattern in patterns {
        if let Some(caps) = pattern.captures(url) {
            return Some(caps[1].to_string());
        }
    }
    // Also try from redirectPath (URL-encoded path) in login/error URLs
    static REDIRECT_RE: OnceLock<Regex> = OnceLock::new();
    if let Some(caps) = REDIRECT_RE
        .get_or_init(|| Regex::new(r"redirectPath=.*?%2Fexplore%2F([a-f0-9]+)").expect("regex: redirect path"))
        .captures(url)
    {
        return Some(caps[1].to_string());
    }
    None
}

/// Resolve an xhslink.com short URL using reqquest HTTP redirect.
pub async fn resolve_short_url(url: &str) -> Result<String, String> {
    if !url.contains("xhslink.com") {
        return Ok(url.to_string());
    }

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let resp = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    // Follow redirect chain manually
    let status = resp.status();
    if status.is_redirection() {
        if let Some(location) = resp.headers().get("location").and_then(|v| v.to_str().ok()) {
            let resolved = if location.starts_with("http") {
                location.to_string()
            } else {
                // Relative redirect — resolve against original URL
                let base = url.trim_end_matches('/');
                format!("{}{}", base, location)
            };
            if resolved == url {
                return Err("短链解析失败: URL 未变化".into());
            }
            return Ok(resolved);
        }
    }

    if status.is_success() {
        // Might have gotten the final page directly (no redirect)
        return Ok(resp.url().to_string());
    }

    Err(format!("短链解析失败: HTTP {}", status))
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
