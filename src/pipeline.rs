use crate::analyzer::AnalyzerError;
use crate::models::Recipe;
use crate::sources::SourceError;
use crate::textifier::TextifierError;
use std::time::Duration;

pub struct ExtractOptions<'a> {
    pub url: &'a str,
    pub asr_model: &'a str,
    pub llm_model: &'a str,
    pub send_images: bool,
    pub api_key: Option<&'a str>,
    pub timeout_secs: u64,
}

/// Run the full extraction pipeline: fetch → textify → analyze.
pub async fn extract(opts: ExtractOptions<'_>) -> Result<Recipe, PipelineError> {
    let timeout = Duration::from_secs(opts.timeout_secs);
    tokio::time::timeout(timeout, async {
        // Step 1: Fetch
        let raw = crate::sources::fetch(opts.url).await?;
        crate::vprintln!("  ✓ 标题: {}", raw.title);
        let note_type = if raw.has_video { "视频笔记" } else { "图文笔记" };
        crate::vprintln!("  ✓ 类型: {} | 图片: {} 张", note_type, raw.image_urls.len());

        // Step 2: Textify
        let text = crate::textifier::process(&raw, opts.asr_model).await?;

        // Step 3: Analyze
        let image_urls: &[String] = if opts.send_images {
            &raw.image_urls
        } else {
            &[]
        };
        let mut recipe = crate::analyzer::extract_recipe(
            crate::analyzer::shared_client(),
            &text.full_text,
            image_urls,
            opts.llm_model,
            opts.api_key,
        )
        .await?;

        recipe.source_url = raw.source_url.clone();
        Ok(recipe)
    })
    .await
    .map_err(|_| PipelineError::Timeout)?
}

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("{0}")]
    Source(#[from] SourceError),
    #[error("{0}")]
    Textifier(#[from] TextifierError),
    #[error("{0}")]
    Analyzer(#[from] AnalyzerError),
    #[error("提取超时，请增大 --timeout 值")]
    Timeout,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_options_build() {
        let opts = ExtractOptions {
            url: "https://example.com",
            asr_model: "qwen3-asr-0.6b",
            llm_model: "deepseek-chat",
            send_images: true,
            api_key: None,
            timeout_secs: 300,
        };
        assert_eq!(opts.url, "https://example.com");
        assert!(opts.send_images);
    }

    #[test]
    fn test_extract_unsupported_url() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let opts = ExtractOptions {
            url: "https://example.com",
            asr_model: "qwen3-asr-0.6b",
            llm_model: "deepseek-chat",
            send_images: true,
            api_key: None,
            timeout_secs: 300,
        };
        let result = rt.block_on(extract(opts));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, PipelineError::Source(SourceError::Unsupported(_))));
        assert!(err.to_string().contains("unsupported URL"));
    }
}
