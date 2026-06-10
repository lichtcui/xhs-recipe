/// Cookie management for Xiaohongshu.
/// Reads/writes cookies in the same format as the Python auth system.
use std::path::PathBuf;

#[derive(Debug, Clone, serde::Deserialize)]
struct SavedCookie {
    name: String,
    value: String,
    domain: String,
    #[allow(dead_code)]
    path: String,
}

/// Cookies in CDP format, ready to set via chromiumoxide.
#[derive(Debug, Clone)]
pub struct CdpCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
}

/// Get the cookie file path (~/.cache/xhs-recipe/cookies.json).
pub fn cookie_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".cache").join("xhs-recipe").join("cookies.json")
}

/// Load saved cookies from the cookie file.
pub fn load_cookies() -> Vec<CdpCookie> {
    let path = cookie_path();
    if !path.exists() {
        return vec![];
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let saved: Vec<SavedCookie> = match serde_json::from_str(&content) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    saved
        .into_iter()
        .filter(|c| c.domain.contains("xiaohongshu"))
        .map(|c| CdpCookie {
            name: c.name,
            value: c.value,
            domain: c.domain,
            path: c.path,
        })
        .collect()
}

/// Check if any saved cookies exist.
pub fn has_cookies() -> bool {
    !load_cookies().is_empty()
}

/// Save cookies from playwright-rs Cookie objects to the cookie file.
/// Format matches Python's cookie file format.
pub fn save_cookies(cookies: &[playwright::Cookie]) {
    let data: Vec<serde_json::Value> = cookies
        .iter()
        .map(|c| {
            serde_json::json!({
                "name": c.name,
                "value": c.value,
                "domain": c.domain,
                "path": c.path,
            })
        })
        .collect();
    let path = cookie_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&data) {
        let _ = std::fs::write(&path, json);
    }
}
