/// Xiaohongshu URL handling: detection, short URL resolution, note ID extraction.
use regex::Regex;

/// Check if a URL is a Xiaohongshu URL.
pub fn is_xhs_url(url: &str) -> bool {
    let patterns = [
        Regex::new(r"xiaohongshu\.com/explore/[a-f0-9]+").unwrap(),
        Regex::new(r"xiaohongshu\.com/discovery/item/[a-f0-9]+").unwrap(),
        Regex::new(r"xhslink\.com/\w+").unwrap(),
    ];
    patterns.iter().any(|p| p.is_match(url))
}

/// Extract note ID from a resolved Xiaohongshu URL.
pub fn extract_note_id(url: &str) -> Option<String> {
    for pattern in &[
        Regex::new(r"/explore/([a-f0-9]+)").unwrap(),
        Regex::new(r"/discovery/item/([a-f0-9]+)").unwrap(),
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

/// Resolve an xhslink.com short URL using playwright-rs (needs JS redirect).
pub async fn resolve_short_url(url: &str) -> Result<String, String> {
    if !url.contains("xhslink.com") {
        return Ok(url.to_string());
    }

    use playwright::{BrowserContextOptions, LaunchOptions, Playwright};

    ensure_driver_path();
    let pw = Playwright::launch()
        .await
        .map_err(|e| format!("PW init: {}", e))?;

    let browser = pw
        .chromium()
        .launch_with_options(
            LaunchOptions::default()
                .headless(true)
                .args(vec!["--no-sandbox".into()])
                .executable_path(chrome_path()),
        )
        .await
        .map_err(|e| format!("PW launch: {}", e))?;

    let context = browser
        .new_context_with_options(
            BrowserContextOptions::builder()
                .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36".into())
                .build(),
        )
        .await
        .map_err(|e| format!("PW context: {}", e))?;

    let page = context
        .new_page()
        .await
        .map_err(|e| format!("PW page: {}", e))?;

    page.goto(url, None)
        .await
        .map_err(|e| format!("PW goto: {}", e))?;

    // Small delay to let redirect complete
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    let resolved = page.url();
    if resolved == url {
        return Err("短链解析失败: URL 未变化".into());
    }
    Ok(resolved)
}

/// Auto-detect the Playwright driver path and set PLAYWRIGHT_DRIVER_PATH env var if not set.
pub fn ensure_driver_path() {
    if std::env::var("PLAYWRIGHT_DRIVER_PATH").is_ok() {
        return;
    }
    if let Some(path) = find_driver_path() {
        std::env::set_var("PLAYWRIGHT_DRIVER_PATH", &path);
    }
}

fn find_driver_path() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    // Python Playwright installed via pip/uv caches the driver
    let uv_cache = std::path::Path::new(&home).join(".cache/uv/archive-v0");
    if let Ok(entries) = std::fs::read_dir(&uv_cache) {
        for entry in entries.filter_map(|e| e.ok()) {
            let driver = entry.path().join("playwright/driver/node");
            if driver.exists() {
                let parent = driver.parent()?;
                return Some(parent.to_string_lossy().to_string());
            }
        }
    }
    // playwright-rs cache location
    let pw_cache = std::path::Path::new(&home).join(".cache/ms-playwright");
    if pw_cache.exists() {
        for entry in std::fs::read_dir(&pw_cache).ok()?.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() && path.file_name().and_then(|n| n.to_str()).map_or(false, |n| n.starts_with("driver-")) {
                return Some(path.to_string_lossy().to_string());
            }
        }
    }
    None
}

/// Find Chrome/Chromium executable path for Playwright.
/// Prefers the full Chromium browser (Python Playwright installed version).
pub fn chrome_path() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates = vec![
        // Full Chromium from Python Playwright (best anti-detection)
        format!("{}/Library/Caches/ms-playwright/chromium-1223/chrome-mac-x64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing", home),
        // Headless shell
        format!("{}/Library/Caches/ms-playwright/chromium_headless_shell-1223/chrome-headless-shell-mac-x64/chrome-headless-shell", home),
    ];
    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return path.clone();
        }
    }
    String::new()
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
