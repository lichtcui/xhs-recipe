pub mod auth;
pub mod scraper;
pub mod url;

use crate::models::RawContent;
use super::SourceError;

/// Fetch a Xiaohongshu note by URL.
pub async fn fetch(url: &str) -> Result<RawContent, SourceError> {
    scraper::scrape(url).await
}
