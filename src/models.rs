use serde::{Deserialize, Serialize};

/// Content type classification for splitter rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentType {
    /// Single video post
    Video,
    /// Single image post
    Image,
    /// Multi-image collection post (e.g. "11道菜")
    Collection,
}

impl Default for ContentType {
    fn default() -> Self {
        Self::Image
    }
}

/// Platform-agnostic raw content returned by source adapters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawContent {
    pub title: String,
    pub text_content: String,
    #[serde(default)]
    pub image_urls: Vec<String>,
    #[serde(default)]
    pub has_video: bool,
    pub video_url: Option<String>,
    /// Platform source identifier (e.g. "xiaohongshu")
    pub source: String,
    pub source_url: String,
    /// Content type hint for splitter (auto-detected by scraper)
    #[serde(default)]
    pub content_type: ContentType,
}

/// All media converted to text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    pub full_text: String,
    /// Per-image OCR texts (empty for video posts). Used for batching collection extracts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub image_texts: Vec<String>,
    pub title: String,
    pub source: String,
    pub source_url: String,
}

/// A recipe ingredient or seasoning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ingredient {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

/// A cooking step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    pub content: String,
}

/// The final structured recipe output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_time: Option<String>,
    #[serde(default)]
    pub ingredients: Vec<Ingredient>,
    #[serde(default)]
    pub seasonings: Vec<Ingredient>,
    #[serde(default)]
    pub equipment: Vec<String>,
    #[serde(default)]
    pub steps: Vec<Step>,
    #[serde(default)]
    pub tips: Vec<String>,
    pub source_url: String,
    #[serde(default = "default_true")]
    pub is_food: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for Recipe {
    fn default() -> Self {
        Self {
            name: String::new(),
            total_time: None,
            ingredients: vec![],
            seasonings: vec![],
            equipment: vec![],
            steps: vec![],
            tips: vec![],
            source_url: String::new(),
            is_food: true,
            reason: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recipe_json_roundtrip() {
        let recipe = Recipe {
            name: "蒜香椒盐烤排骨".into(),
            total_time: Some("1小时25分钟".into()),
            ingredients: vec![
                Ingredient {
                    name: "排骨".into(),
                    amount: Some("适量".into()),
                    prep: Some("清洗干净，擦干水分".into()),
                    category: Some("食材".into()),
                },
            ],
            seasonings: vec![],
            equipment: vec!["空气炸锅".into()],
            steps: vec![
                Step {
                    title: "清洗".into(),
                    time: Some("约5分钟".into()),
                    content: "排骨加入1勺面粉，抓洗出血水".into(),
                },
            ],
            tips: vec!["腌制时间建议至少1小时".into()],
            source_url: "https://xhslink.com/test".into(),
            is_food: true,
            reason: None,
        };

        let json = serde_json::to_string_pretty(&recipe).unwrap();
        let deserialized: Recipe = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "蒜香椒盐烤排骨");
        assert_eq!(deserialized.ingredients.len(), 1);
        assert_eq!(deserialized.seasonings.len(), 0);
        assert!(deserialized.is_food);
    }

    #[test]
    fn test_default_values() {
        let recipe = Recipe {
            name: "test".into(),
            total_time: None,
            ingredients: vec![],
            seasonings: vec![],
            equipment: vec![],
            steps: vec![],
            tips: vec![],
            source_url: "".into(),
            is_food: true,
            reason: None,
        };
        // is_food should serialize as true (default), not null
        let json = serde_json::to_string(&recipe).unwrap();
        assert!(json.contains("\"is_food\":true"));
        // None fields should be skipped
        assert!(!json.contains("total_time"));
        assert!(!json.contains("reason"));
    }

    #[test]
    fn test_raw_content_roundtrip() {
        let raw = RawContent {
            title: "测试标题".into(),
            text_content: "测试描述".into(),
            image_urls: vec!["https://example.com/img.jpg".into()],
            has_video: true,
            video_url: Some("https://example.com/video.mp4".into()),
            source: "xiaohongshu".into(),
            source_url: "https://xhslink.com/test".into(),
            content_type: Default::default(),
        };
        let json = serde_json::to_string_pretty(&raw).unwrap();
        let deserialized: RawContent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title, "测试标题");
        assert!(deserialized.has_video);
        assert_eq!(deserialized.image_urls.len(), 1);
    }
}
