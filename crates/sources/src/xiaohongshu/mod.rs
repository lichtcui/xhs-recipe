/// Xiaohongshu source adapter using chromiumoxide.
pub mod auth;
pub mod scraper;
pub mod url;

use models::RawContent;
use crate::SourceError;

/// Fetch a Xiaohongshu note by URL.
pub async fn fetch(url: &str) -> Result<RawContent, SourceError> {
    // Short URLs need resolution. Methods are attempted in order of reliability:
    // Python bridge (handles everything internally) > playwright-rs > HTTP
    scraper::scrape(url, "").await
}
