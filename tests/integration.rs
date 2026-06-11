use xhs_recipe::models::Recipe;
use xhs_recipe::sources;

#[test]
fn test_supports_xiaohongshu_urls() {
    assert!(sources::supports_url("https://www.xiaohongshu.com/explore/abc123"));
    assert!(sources::supports_url("https://xhslink.com/test123"));
    assert!(!sources::supports_url("https://example.com"));
    assert!(!sources::supports_url(""));
}

#[test]
fn test_recipe_default_is_food() {
    let recipe = Recipe::default();
    assert!(recipe.is_food);
    assert_eq!(recipe.name, "");
}

#[test]
fn test_recipe_serde_roundtrip() {
    use xhs_recipe::models::{Ingredient, Step};

    let recipe = Recipe {
        name: "红烧肉".into(),
        total_time: Some("2小时".into()),
        ingredients: vec![Ingredient {
            name: "五花肉".into(),
            amount: Some("500g".into()),
            prep: Some("切块".into()),
            category: Some("食材".into()),
        }],
        seasonings: vec![],
        equipment: vec!["锅".into()],
        steps: vec![Step {
            title: "炖煮".into(),
            time: Some("1.5小时".into()),
            content: "小火慢炖至收汁".into(),
        }],
        tips: vec!["冰糖上色更亮".into()],
        source_url: "https://example.com/recipe".into(),
        is_food: true,
        reason: None,
    };

    let json = serde_json::to_string_pretty(&recipe).unwrap();
    let deserialized: Recipe = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.name, "红烧肉");
    assert_eq!(deserialized.ingredients.len(), 1);
    assert_eq!(deserialized.ingredients[0].name, "五花肉");
    assert_eq!(deserialized.equipment[0], "锅");
    assert_eq!(deserialized.steps.len(), 1);
    assert_eq!(deserialized.tips.len(), 1);
    assert!(deserialized.is_food);
    assert_eq!(deserialized.source_url, "https://example.com/recipe");
}

#[test]
fn test_recipe_non_food_roundtrip() {
    let recipe = Recipe {
        name: String::new(),
        total_time: None,
        ingredients: vec![],
        seasonings: vec![],
        equipment: vec![],
        steps: vec![],
        tips: vec![],
        source_url: String::new(),
        is_food: false,
        reason: Some("旅游攻略，非美食内容".into()),
    };

    let json = serde_json::to_string_pretty(&recipe).unwrap();
    let deserialized: Recipe = serde_json::from_str(&json).unwrap();

    assert!(!deserialized.is_food);
    assert_eq!(deserialized.reason.unwrap(), "旅游攻略，非美食内容");
}
