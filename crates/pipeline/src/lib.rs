use core::Recipe;

pub struct ExtractOptions<'a> {
    pub url: &'a str,
    pub whisper_model: &'a str,
    pub llm_model: &'a str,
    pub send_images: bool,
    pub api_key: Option<&'a str>,
}

/// Run the full extraction pipeline: fetch → textify → analyze.
pub async fn extract(opts: ExtractOptions<'_>) -> Result<Recipe, PipelineError> {
    // Step 1: Fetch
    println!("  ↓ 抓取页面内容...");
    let raw = sources::fetch(opts.url)
        .await
        .map_err(|e| PipelineError::Source(e.to_string()))?;
    println!("  ✓ 标题: {}", raw.title);
    println!("  ✓ 类型: {}", if raw.has_video { "视频笔记" } else { "图文笔记" });
    println!("  ✓ 图片: {} 张", raw.image_urls.len());

    // Step 2: Textify
    let text = textifier::process(&raw, opts.whisper_model)
        .await
        .map_err(|e| PipelineError::Textifier(e.to_string()))?;

    // Step 3: Analyze
    let image_urls: &[String] = if opts.send_images {
        &raw.image_urls
    } else {
        &[]
    };
    let mut recipe = analyzer::extract_recipe(
        &text.full_text,
        &text.title,
        image_urls,
        opts.llm_model,
        opts.api_key,
    )
    .await
    .map_err(|e| PipelineError::Analyzer(e.to_string()))?;

    recipe.source_url = raw.source_url.clone();
    Ok(recipe)
}

#[derive(Debug)]
pub enum PipelineError {
    Source(String),
    Textifier(String),
    Analyzer(String),
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(msg) => write!(f, "source error: {}", msg),
            Self::Textifier(msg) => write!(f, "textifier error: {}", msg),
            Self::Analyzer(msg) => write!(f, "analyzer error: {}", msg),
        }
    }
}

impl std::error::Error for PipelineError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_options_build() {
        let opts = ExtractOptions {
            url: "https://example.com",
            whisper_model: "medium",
            llm_model: "deepseek-chat",
            send_images: true,
            api_key: None,
        };
        assert_eq!(opts.url, "https://example.com");
        assert!(opts.send_images);
    }

    #[test]
    fn test_extract_unsupported_url() {
        // Manually create tokio runtime to avoid core crate shadowing in macro
        let rt = tokio::runtime::Runtime::new().unwrap();
        let opts = ExtractOptions {
            url: "https://example.com",
            whisper_model: "medium",
            llm_model: "deepseek-chat",
            send_images: true,
            api_key: None,
        };
        let result = rt.block_on(extract(opts));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, PipelineError::Source(_)));
        assert!(err.to_string().contains("unsupported URL"));
    }
}
