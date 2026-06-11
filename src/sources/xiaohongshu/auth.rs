/// Cookie management for Xiaohongshu — save/load/login/logout via zendriver-rs.
use std::path::PathBuf;

use zendriver::Cookie;

// ── Paths ────────────────────────────────────────────────────────────

/// Get the cookie file path (~/.cache/xhs-recipe/cookies.json).
pub fn cookie_path() -> PathBuf {
    crate::home_dir()
        .join(".cache")
        .join("xhs-recipe")
        .join("cookies.json")
}

// ── Load / Save ──────────────────────────────────────────────────────

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

/// Save cookies (zendriver::Cookie) to the cookie file.
pub fn save_cookies(cookies: &[Cookie]) {
    let path = cookie_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&cookies) {
        let _ = std::fs::write(&path, json);
    }
}

// ── Login ────────────────────────────────────────────────────────────

/// Scan QR code to log into Xiaohongshu, save cookies on success.
/// Returns true on successful login, false on timeout.
pub async fn login(headless: bool, timeout_secs: u32) -> Result<bool, String> {
    let cookie_dir = cookie_path().parent().map(|p| p.to_path_buf()).unwrap_or_default();

    let browser = zendriver::Browser::builder()
        .headless(headless)
        .lang(String::from("zh-CN"))
        .launch()
        .await
        .map_err(|e| format!("启动浏览器失败: {}", e))?;

    let tab = browser.main_tab();
    tab.goto("https://www.xiaohongshu.com/login")
        .await
        .map_err(|e| format!("页面加载失败: {}", e))?;
    tab.wait_for_load().await.ok();

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Try to capture QR code image
    let qr_selectors = [
        "img[class*='qrcode']",
        "img[alt*='QR']",
        ".login-qrcode img",
        "[class*='qrcode'] img",
        "canvas",
    ];

    let mut qr_saved = false;
    for sel in &qr_selectors {
        match tab.find().css(*sel).one().await {
            Ok(el) => {
                if let Ok(bytes) = el.screenshot().await {
                    let qr_path = cookie_dir.join("login_qr.png");
                    if tokio::fs::write(&qr_path, &bytes).await.is_ok() {
                        println!("\n二维码已保存到: {}", qr_path.display());
                        qr_saved = true;
                    }
                    break;
                }
            }
            Err(_) => continue,
        }
    }

    if !qr_saved {
        // Fallback: full page screenshot
        if let Ok(bytes) = tab.screenshot().await {
            let qr_path = cookie_dir.join("login_qr.png");
            if tokio::fs::write(&qr_path, &bytes).await.is_ok() {
                println!("\n页面截图已保存到: {}", qr_path.display());
                qr_saved = true;
            }
        }
    }

    print_login_instructions(qr_saved, headless);

    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs as u64);

    loop {
        if start.elapsed() >= timeout {
            println!("\n❌ 登录超时（{} 秒），请重试", timeout_secs);
            browser.close().await.ok();
            return Ok(false);
        }

        match tab.url().await {
            Ok(url) => {
                let url_str = url.to_string();
                if !url_str.contains("/login") && url_str.contains("/explore") {
                    // Login successful — redirect happened
                    break;
                }
            }
            Err(_) => {}
        }

        // Also check for avatar element as login indicator
        if tab
            .find()
            .css("[class*='avatar'], [class*='user'], [class*='User']")
            .one()
            .await
            .is_ok()
        {
            break;
        }

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let elapsed = start.elapsed().as_secs();
        let remaining = timeout_secs - elapsed as u32;
        if remaining % 10 == 0 {
            println!("  ⏳ 等待扫码... 还剩 {} 秒", remaining);
        }
    }

    // Save cookies
    let jar = browser.cookies();
    let cookies = jar
        .all()
        .await
        .map_err(|e| format!("获取 Cookie 失败: {}", e))?;

    save_cookies(&cookies);

    println!("\n✅ 登录成功！Cookie 已保存到 {}", cookie_path().display());
    println!("   Cookie 数: {}", cookies.len());

    browser.close().await.ok();
    Ok(true)
}

/// Clear saved cookies.
pub async fn logout() {
    let path = cookie_path();
    if path.exists() {
        std::fs::remove_file(&path).ok();
        println!("✅ Cookie 已清除");
    } else {
        println!("ℹ️  没有已保存的 Cookie");
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn print_login_instructions(qr_saved: bool, headless: bool) {
    println!();
    println!("==================================================");
    println!("📱 小红书扫码登录");
    println!("==================================================");

    if qr_saved {
        println!("\n请用「小红书 App」扫描二维码登录");
    } else {
        println!("\n请在浏览器窗口中完成登录");
    }

    if headless {
        println!("\n提示: 如果看不到二维码，可以尝试不加 --headless 参数");
        println!("   xhs-recipe login");
    }

    println!("\n等待扫码中...");
}
