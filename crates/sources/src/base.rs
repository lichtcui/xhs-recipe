use core::RawContent;
use crate::SourceError;

const SUPPORTED_DOMAINS: &[&str] = &["xiaohongshu.com", "xhslink.com"];

pub async fn fetch(url: &str) -> Result<RawContent, SourceError> {
    if !supports_url(url) {
        return Err(SourceError::Unsupported(url.to_string()));
    }
    if url.contains("xiaohongshu.com") || url.contains("xhslink.com") {
        return super::xiaohongshu::fetch(url).await;
    }
    Err(SourceError::Unsupported(url.to_string()))
}

pub fn supports_url(url: &str) -> bool {
    SUPPORTED_DOMAINS.iter().any(|d| url.contains(d))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supports_xhslink() {
        assert!(supports_url("https://xhslink.com/o/test123"));
    }

    #[test]
    fn test_supports_xiaohongshu() {
        assert!(supports_url("https://www.xiaohongshu.com/explore/abc123"));
    }

    #[test]
    fn test_unsupported_url() {
        assert!(!supports_url("https://example.com"));
    }
}
