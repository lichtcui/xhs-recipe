/// Cookie management for Xiaohongshu — save/load.
use std::path::PathBuf;
use std::sync::Arc;

use reqwest::cookie::Jar;
use reqwest::Url;
use serde::{Deserialize, Serialize};

/// A simple serializable cookie for file-based storage.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub path: String,
}

// ── Paths ────────────────────────────────────────────────────

/// Get the cookie file path (~/.cache/xhs-recipe/cookies.json).
pub fn cookie_path() -> PathBuf {
    crate::home_dir()
        .join(".cache")
        .join("xhs-recipe")
        .join("cookies.json")
}

// ── Load / Save ──────────────────────────────────────────────

/// Load saved cookies from the cookie file.
pub fn load_cookies() -> Vec<Cookie> {
    let path = cookie_path();
    if !path.exists() {
        return vec![];
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    serde_json::from_str(&content).unwrap_or_default()
}

/// Check if any saved cookies exist.
pub fn has_cookies() -> bool {
    cookie_path().exists()
}

/// Build a reqwest cookie jar pre-populated with saved Xiaohongshu cookies.
/// This jar can be passed to `reqwest::ClientBuilder::cookie_provider()` to
/// automatically include saved cookies (e.g. `a1`, `web_session`) in all requests.
pub fn build_cookie_jar() -> Arc<Jar> {
    let jar = Arc::new(Jar::default());
    let xhs_url: Url = "https://www.xiaohongshu.com".parse().unwrap();
    for cookie in load_cookies() {
        // Build a minimal Set-Cookie header value and add to jar
        let mut value = format!("{}={}", cookie.name, cookie.value);
        if !cookie.domain.is_empty() {
            value.push_str(&format!("; Domain={}", cookie.domain));
        }
        if !cookie.path.is_empty() {
            value.push_str(&format!("; Path={}", cookie.path));
        } else {
            value.push_str("; Path=/");
        }
        jar.add_cookie_str(&value, &xhs_url);
    }
    jar
}

/// Save cookies to the cookie file.
pub fn save_cookies(cookies: &[Cookie]) {
    let path = cookie_path();
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("  ⚠ 创建 Cookie 目录失败: {}", e);
        }
    }
    if let Ok(json) = serde_json::to_string_pretty(&cookies) {
        if let Err(e) = std::fs::write(&path, json) {
            eprintln!("  ⚠ 写入 Cookie 文件失败: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cookie_path_ends_with_cookies_json() {
        let path = cookie_path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        assert_eq!(name, "cookies.json");
    }

    #[test]
    fn test_cookie_path_contains_xhs_recipe_dir() {
        let path = cookie_path();
        let path_str = path.to_string_lossy();
        assert!(path_str.contains(".cache"));
        assert!(path_str.contains("xhs-recipe"));
    }

    #[test]
    fn test_cookie_serialize_deserialize_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cookies.json");

        let cookies = vec![Cookie {
            name: "session".into(),
            value: "abc123".into(),
            domain: ".xiaohongshu.com".into(),
            path: "/".into(),
        }];
        let json = serde_json::to_string_pretty(&cookies).unwrap();
        std::fs::write(&path, &json).unwrap();
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).unwrap();
        let loaded: Vec<Cookie> = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "session");
        assert_eq!(loaded[0].value, "abc123");
    }

    #[test]
    fn test_cookie_serde_empty_vec() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cookies.json");

        let json = serde_json::to_string_pretty(&Vec::<Cookie>::new()).unwrap();
        std::fs::write(&path, &json).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let loaded: Vec<Cookie> = serde_json::from_str(&content).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_cookie_file_creation_and_deletion() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cookies.json");

        assert!(!path.exists());

        std::fs::write(&path, "[]").unwrap();
        assert!(path.exists());

        std::fs::remove_file(&path).ok();
        assert!(!path.exists());
    }
}
