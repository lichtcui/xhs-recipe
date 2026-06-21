use crate::models::Recipe;
use crate::models::TextContent;
use crate::sources::SourceError;
use crate::textifier::TextifierError;
use crate::analyzer::AnalyzerError;
use regex::Regex;
use std::sync::OnceLock;
use std::time::Duration;

pub struct ExtractOptions<'a> {
    pub url: &'a str,
    pub asr_model: &'a str,
    pub llm_model: &'a str,
    pub send_images: bool,
    pub api_key: Option<&'a str>,
    pub timeout_secs: u64,
}

/// Try to detect collection post count from title (e.g., "11道", "10款", "5种").
pub fn detect_collection_count(title: &str) -> Option<usize> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(\d+)\s*[道款式个]").expect("invalid regex"));
    let cap = re.captures(title)?;
    cap[1].parse::<usize>().ok()
}

/// Process a collection post by batching per-image OCR texts into separate LLM calls.
/// Each batch contains at most BATCH_SIZE images, ensuring the output fits within token limits.
pub async fn extract_collection(
    text: &TextContent,
    total: usize,
    model: &str,
    api_key: Option<&str>,
) -> Result<Vec<Recipe>, PipelineError> {
    const BATCH_SIZE: usize = 4;

    if text.image_texts.is_empty() {
        // Fallback: no per-image OCR data, use combined text in a single call
        return crate::analyzer::extract_recipe(crate::analyzer::shared_client(), &text.full_text, &[], model, api_key).await
            .map_err(PipelineError::from);
    }

    // Build batch texts from per-image OCR results
    let mut batches: Vec<String> = Vec::new();
    for (chunk_idx, chunk) in text.image_texts.chunks(BATCH_SIZE).enumerate() {
        let batch_start = chunk_idx * BATCH_SIZE;
        let mut batch = format!("标题：{}\n\n本批包含图片 {}-{}（共 {} 张图，每张图一个菜谱）：", text.title,
            batch_start + 1, batch_start + chunk.len(), total);
        for (local_idx, img_text) in chunk.iter().enumerate() {
            let img_num = batch_start + local_idx + 1;
            batch.push_str(&format!("\n\n---图片 {}---\n", img_num));
            if img_text.is_empty() {
                batch.push_str("（图片中未识别出文字）");
            } else {
                batch.push_str(img_text);
            }
        }
        batches.push(batch);
    }

    // Launch all batch calls in parallel
    let mut handles = Vec::new();
    for batch_text in batches {
        let model = model.to_string();
        let key = api_key.map(|s| s.to_string());
        handles.push(tokio::spawn(async move {
            crate::analyzer::extract_recipe(
                crate::analyzer::shared_client(),
                &batch_text,
                &[],
                &model,
                key.as_deref(),
            ).await
        }));
    }

    // Collect results in order (preserving batch order = image order)
    let mut all_recipes = Vec::new();
    for handle in handles {
        let result = handle.await
            .map_err(|e| PipelineError::Analyzer(AnalyzerError::ApiError(format!("task failed: {}", e))))?;
        all_recipes.extend(result?);
    }

    Ok(all_recipes)
}

/// Run the full extraction pipeline: fetch → textify → analyze.
/// Returns multiple recipes when the content is a collection (e.g. multi-recipe post).
pub async fn extract(opts: ExtractOptions<'_>) -> Result<Vec<Recipe>, PipelineError> {
    let timeout = Duration::from_secs(opts.timeout_secs);
    tokio::time::timeout(timeout, async {
        // Step 1: Fetch
        let raw = crate::sources::fetch(opts.url).await?;
        crate::vprintln!("  ✓ 标题: {}", raw.title);
        let note_type = if raw.has_video { "视频笔记" } else { "图文笔记" };
        crate::vprintln!("  ✓ 类型: {} | 图片: {} 张", note_type, raw.image_urls.len());
        if !raw.image_urls.is_empty() {
            for (i, url) in raw.image_urls.iter().enumerate() {
                let truncated = if url.len() > 60 {
                    format!("{}...", &url[..60])
                } else {
                    url.clone()
                };
                crate::vprintln!("      图片[{}]: {}", i, truncated);
            }
        }

        // Step 2: Textify (includes OCR for image posts when send_images is true)
        let text = crate::textifier::process_cli(&raw, opts.asr_model, opts.send_images).await?;

        // Step 3: Analyze (images already OCR'd into text, no need to pass separately)
        let mut recipes = if opts.send_images && !raw.has_video && !text.image_texts.is_empty() {
            // Check if this is a collection post (title has count like "11道")
            let count = detect_collection_count(&text.title);
            let total = count.unwrap_or(text.image_texts.len());
            if count.is_some() && total > 1 {
                println!("  📦 检测到合集 (共{}道菜)，分批处理...", total);
                extract_collection(
                    &text,
                    total,
                    opts.llm_model,
                    opts.api_key,
                ).await?
            } else {
                crate::analyzer::extract_recipe(
                    crate::analyzer::shared_client(),
                    &text.full_text,
                    &[],
                    opts.llm_model,
                    opts.api_key,
                ).await?
            }
        } else {
            crate::analyzer::extract_recipe(
                crate::analyzer::shared_client(),
                &text.full_text,
                &[],
                opts.llm_model,
                opts.api_key,
            ).await?
        };

        // Set source_url and new metadata fields on every recipe
        let cover_img = raw.image_urls.first().cloned();
        let all_images = raw.image_urls.clone();
        let raw_text = text.full_text.clone();
        for recipe in &mut recipes {
            recipe.source_url = raw.source_url.clone();
            recipe.cover_image_url = cover_img.clone();
            recipe.image_urls = Some(all_images.clone());
            recipe.raw_text = Some(raw_text.clone());
        }
        Ok(recipes)
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
