use xhs_recipe::models::{ContentType, RawContent, TextContent};

/// Split extracted content into items for SSE streaming.
///
/// Rules:
/// - Video notes → single item [description + ASR]
/// - Image notes (single) → single item [description + all OCR]
/// - Collection posts → per-image items [description + per-image OCR]
pub fn split(raw: &RawContent, text: &TextContent) -> Vec<String> {
    match raw.content_type {
        ContentType::Video => {
            vec![text.full_text.clone()]
        }
        ContentType::Collection => {
            split_collection(raw, text)
        }
        ContentType::Image => {
            let non_empty_count = text.image_texts.iter().filter(|t| !t.is_empty()).count();
            if non_empty_count > 1 {
                // Multiple non-empty OCR texts from a single image post suggests collection-like content
                split_collection(raw, text)
            } else {
                // Single image or empty OCR → single combined item
                vec![text.full_text.clone()]
            }
        }
    }
}

/// Build per-image items for a collection post.
fn split_collection(raw: &RawContent, text: &TextContent) -> Vec<String> {
    let image_texts = &text.image_texts;
    if image_texts.is_empty() {
        return vec![text.full_text.clone()];
    }

    if raw.image_urls.len() != image_texts.len() {
        return vec![text.full_text.clone()];
    }

    let mut items = Vec::new();
    for (i, img_text) in image_texts.iter().enumerate() {
        let mut item = String::new();
        if !raw.title.is_empty() {
            item.push_str(&format!("标题：{}", raw.title));
        }
        if !raw.text_content.is_empty() {
            if !item.is_empty() {
                item.push('\n');
            }
            item.push_str(&format!("描述：{}", raw.text_content));
        }
        if !img_text.is_empty() {
            if !item.is_empty() {
                item.push('\n');
            }
            item.push_str(&format!("图片 {} 文字：\n{}", i + 1, img_text));
        }
        items.push(item);
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_raw(title: &str, desc: &str, images: usize, has_video: bool) -> RawContent {
        RawContent {
            title: title.to_string(),
            text_content: desc.to_string(),
            image_urls: (0..images).map(|i| format!("https://example.com/{}.jpg", i)).collect(),
            has_video,
            video_url: if has_video { Some("https://example.com/v.mp4".into()) } else { None },
            source: "test".into(),
            source_url: "https://example.com".into(),
            content_type: Default::default(),
        }
    }

    fn make_text(title: &str, full_text: &str, image_texts: Vec<&str>) -> TextContent {
        TextContent {
            title: title.to_string(),
            full_text: full_text.to_string(),
            image_texts: image_texts.iter().map(|s| s.to_string()).collect(),
            source: "test".into(),
            source_url: "https://example.com".into(),
        }
    }

    #[test]
    fn test_split_video_single_item() {
        let raw = make_raw("测试视频", "描述内容", 0, true);
        let text = make_text("测试视频", "标题：测试视频\n\n描述：描述内容\n\n视频口述内容：\n步骤1", vec![]);
        let items = split(&raw, &text);
        assert_eq!(items.len(), 1);
        assert!(items[0].contains("视频口述内容"));
    }

    #[test]
    fn test_split_image_single_item() {
        let raw = make_raw("测试图文", "描述内容", 1, false);
        let text = make_text("测试图文", "标题：测试图文\n\n描述：描述内容", vec!["图片文字"]);
        let items = split(&raw, &text);
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_split_collection_detected() {
        let mut raw = make_raw("11道家常菜合集", "全是家常菜", 4, false);
        raw.content_type = ContentType::Collection;
        let text = make_text("11道家常菜合集", "full", vec!["text1", "text2", "text3", "text4"]);
        let items = split(&raw, &text);
        assert_eq!(items.len(), 4);
    }

    #[test]
    fn test_split_single_image_no_collection() {
        let raw = make_raw("番茄炒蛋", "简单美味", 1, false);
        let text = make_text("番茄炒蛋", "full", vec!["步骤文字"]);
        let items = split(&raw, &text);
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_split_video_without_url() {
        let raw = RawContent {
            title: "测试".into(),
            text_content: "描述".into(),
            image_urls: vec![],
            has_video: true,
            video_url: None,
            source: "test".into(),
            source_url: "https://example.com".into(),
            content_type: ContentType::Video,
        };
        let text = make_text("测试", "标题：测试", vec![]);
        let items = split(&raw, &text);
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_split_collection_content_type() {
        let mut raw = make_raw("合集", "5款简单早餐做法，第2款超受欢迎", 5, false);
        raw.content_type = ContentType::Collection;
        let text = make_text("合集", "full", vec!["a", "b", "c", "d", "e"]);
        let items = split(&raw, &text);
        assert_eq!(items.len(), 5);
    }

    #[test]
    fn test_split_non_collection_multi_image() {
        let raw = make_raw("成品展示", "看看我做的好吃的", 3, false);
        let text = make_text("成品展示", "full", vec!["", "", ""]);
        let items = split(&raw, &text);
        assert_eq!(items.len(), 1, "empty OCR should fall back to single item");
    }

    #[test]
    fn test_split_image_multi_nonempty_fallback_to_collection() {
        let raw = make_raw("我的厨房", "各种美食分享", 3, false);
        let text = make_text("我的厨房", "full", vec!["菜谱1内容", "菜谱2内容", "菜谱3内容"]);
        let items = split(&raw, &text);
        // Even without Collection content_type, multiple non-empty OCR texts trigger split
        assert_eq!(items.len(), 3);
    }
}
