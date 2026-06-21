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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_urls: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_text: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for Recipe {
    fn default() -> Self {
        Self {
            id: None,
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
            cover_image_url: None,
            image_urls: None,
            tags: None,
            raw_text: None,
        }
    }
}

impl Recipe {
    /// Returns false if the recipe has obviously placeholder / auto-generated
    /// content with no real cooking information.
    pub fn is_substantial(&self) -> bool {
        let patterns = [
            "待补充", "信息补充", "等待补充", "信息有限",
            "暂未提供", "暂无", "无具体", "未提及", "未提供",
        ];

        // Name must be non-empty and not placeholder
        if self.name.is_empty() || patterns.iter().any(|p| self.name.contains(p)) {
            return false;
        }

        // Check if an ingredient/seasoning has real content (including amount/prep)
        let has_field = |field: &str| !field.is_empty() && !patterns.iter().any(|p| field.contains(p));
        let ing_ok = |i: &Ingredient| -> bool {
            has_field(&i.name)
                && i.amount.as_deref().map_or(true, |a| has_field(a))
                && i.prep.as_deref().map_or(true, |p| has_field(p))
        };

        let has_ingredients = self.ingredients.iter().any(ing_ok);
        let has_seasonings = self.seasonings.iter().any(ing_ok);
        let has_steps = self.steps.iter().any(|s| {
            has_field(&s.content)
                && has_field(&s.title)
                && s.time.as_deref().map_or(true, |t| has_field(t))
        });

        // Fallback: check total meaningful text length
        let total_text: String = [
            self.ingredients.iter().map(|i| i.name.as_str()).collect::<Vec<_>>().join(" "),
            self.steps.iter().map(|s| s.content.as_str()).collect::<Vec<_>>().join(" "),
        ].concat();
        // Need at least 15 characters of concrete ingredient + step text
        let has_min_content = total_text.chars().filter(|c| !c.is_whitespace()).count() >= 15;

        (has_ingredients || has_seasonings || has_steps) && has_min_content
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
            ..Default::default()
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
            ..Default::default()
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

    #[test]
    fn test_is_substantial_rejects_placeholder() {
        // Case 1: obvious placeholder text
        let p1 = Recipe {
            name: "食材信息待补充".into(),
            ingredients: vec![Ingredient {
                name: "食材信息待补充".into(),
                amount: None, prep: None, category: None,
            }],
            steps: vec![Step {
                title: "等待信息补充".into(),
                time: None,
                content: "该菜谱来源于小红书笔记".into(),
            }],
            ..Default::default()
        };
        assert!(!p1.is_substantial());

        // Case 2: vague recipe (ingredient is just "食材", step mentions "未提及")
        let p2 = Recipe {
            name: "春日菜饭".into(),
            ingredients: vec![Ingredient {
                name: "食材".into(),
                amount: Some("具体种类未提及".into()),
                prep: None, category: None,
            }],
            steps: vec![Step {
                title: "准备".into(),
                time: Some("未提及".into()),
                content: "准备春日时蔬和米饭等食材".into(),
            }],
            ..Default::default()
        };
        assert!(!p2.is_substantial(), "vague recipe with placeholder amount/time should be rejected");

        // Case 3: real recipe
        let real = Recipe {
            name: "排骨汤".into(),
            ingredients: vec![Ingredient {
                name: "排骨".into(),
                amount: Some("500g".into()),
                prep: None, category: None,
            }],
            steps: vec![Step {
                title: "焯水".into(),
                time: Some("5分钟".into()),
                content: "排骨冷水下锅，煮开后撇去浮沫".into(),
            }],
            ..Default::default()
        };
        assert!(real.is_substantial());
    }
}
