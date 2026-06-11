use models::RawContent;

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

#[derive(Debug, Clone)]
pub enum SourceError {
    Unsupported(String),
    FetchFailed(String),
    ParseFailed(String),
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(url) => write!(f, "unsupported URL: {}", url),
            Self::FetchFailed(msg) => write!(f, "fetch failed: {}", msg),
            Self::ParseFailed(msg) => write!(f, "parse failed: {}", msg),
        }
    }
}

impl std::error::Error for SourceError {}
