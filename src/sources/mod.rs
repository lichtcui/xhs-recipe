use crate::models::RawContent;

pub mod base;
pub mod xiaohongshu;

/// Fetch content from a supported source URL.
pub async fn fetch(url: &str) -> Result<RawContent, SourceError> {
    base::fetch(url).await
}

/// Check if a URL is supported by any source adapter.
pub fn supports_url(url: &str) -> bool {
    base::supports_url(url)
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum SourceError {
    #[error("unsupported URL: {0}")]
    Unsupported(String),
    #[error("fetch failed: {0}")]
    FetchFailed(String),
    #[error("parse failed: {0}")]
    ParseFailed(String),
}
