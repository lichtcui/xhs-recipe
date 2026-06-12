use std::path::Path;
use crate::models::Recipe;

/// Save recipe(s) to file (.md or .json based on extension).
/// For multiple recipes, saves all to a single file with separators.
pub fn save_to_file(recipes: &[Recipe], path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if recipes.is_empty() {
        return Ok(());
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("md");
    match ext {
        "json" => {
            if recipes.len() == 1 {
                save_json(&recipes[0], path)
            } else {
                save_json_array(recipes, path)
            }
        }
        _ => save_md_multi(recipes, path),
    }
}

fn save_json(recipe: &Recipe, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(recipe)?;
    std::fs::write(path, json)?;
    Ok(())
}

fn save_json_array(recipes: &[Recipe], path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(recipes)?;
    std::fs::write(path, json)?;
    Ok(())
}

fn save_md_multi(recipes: &[Recipe], path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut parts: Vec<String> = recipes.iter()
        .filter(|r| r.is_food)
        .map(recipe_to_md)
        .collect();
    if parts.is_empty() && !recipes.is_empty() {
        // None are food — write the first one explaining why
        parts.push(recipe_to_md(&recipes[0]));
    }
    let combined = parts.join("\n\n---\n\n");
    std::fs::write(path, combined)?;
    Ok(())
}

fn recipe_to_md(recipe: &Recipe) -> String {
    let mut lines = Vec::new();
    lines.push(format!("# {}", recipe.name));
    lines.push(String::new());
    if let Some(ref time) = recipe.total_time {
        lines.push(format!("总时间：{}", time));
        lines.push(String::new());
    }
    if !recipe.ingredients.is_empty() {
        lines.push("## 食材".into());
        for ing in &recipe.ingredients {
            let mut parts = vec![ing.name.clone()];
            if let Some(ref amt) = ing.amount {
                parts.push(amt.clone());
            }
            if let Some(ref prep) = ing.prep {
                parts.push(format!("（{}）", prep));
            }
            lines.push(format!("- {}", parts.join(" ")));
        }
        lines.push(String::new());
    }
    if !recipe.seasonings.is_empty() {
        lines.push("## 调料".into());
        for s in &recipe.seasonings {
            let mut line = s.name.clone();
            if let Some(ref amt) = s.amount {
                line.push_str(&format!(" {}", amt));
            }
            lines.push(format!("- {}", line));
        }
        lines.push(String::new());
    }
    if !recipe.equipment.is_empty() {
        lines.push(format!("器具：{}", recipe.equipment.join("、")));
        lines.push(String::new());
    }
    if !recipe.steps.is_empty() {
        lines.push("## 步骤".into());
        for (i, step) in recipe.steps.iter().enumerate() {
            let fallback = format!("{}.", i + 1);
            let num = crate::STEP_NUMS.get(i).copied().unwrap_or(&fallback);
            let time_str = step.time.as_ref().map_or(String::new(), |t| format!("（{}）", t));
            let label = if step.title.is_empty() {
                format!("步骤{}", i + 1)
            } else {
                step.title.clone()
            };
            lines.push(String::new());
            lines.push(format!("{num} {label}{time_str}"));
            for line in step.content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    lines.push(format!("  {}", trimmed));
                }
            }
        }
        lines.push(String::new());
    }
    if !recipe.tips.is_empty() {
        lines.push("## 小贴士".into());
        for tip in &recipe.tips {
            lines.push(format!("- {}", tip));
        }
        lines.push(String::new());
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Recipe;
    use std::path::PathBuf;

    fn golden_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/testdata")
    }

    fn load_golden_json() -> Recipe {
        let path = golden_dir().join("recipe_test.json");
        let data = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&data).unwrap()
    }

    fn read_golden(path: &Path) -> String {
        let full = golden_dir().join(path);
        std::fs::read_to_string(full).unwrap()
    }

    #[test]
    fn test_json_output_matches_golden() {
        let recipe = load_golden_json();
        let tmp = std::env::temp_dir().join("test_recipe_output.json");
        save_to_file(&[recipe], &tmp).unwrap();
        let produced: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&tmp).unwrap()).unwrap();
        let _ = std::fs::remove_file(&tmp);

        // Verify structure and key fields (not byte-exact due to serde formatting differences)
        assert_eq!(produced["name"], "蒜香椒盐烤排骨");
        assert_eq!(produced["ingredients"].as_array().unwrap().len(), 2);
        assert_eq!(produced["seasonings"].as_array().unwrap().len(), 2);
        assert_eq!(produced["steps"].as_array().unwrap().len(), 3);
        assert_eq!(produced["tips"].as_array().unwrap().len(), 3);
        assert_eq!(produced["is_food"], true);
        assert!(produced.get("reason").is_none());
    }

    #[test]
    fn test_markdown_output_matches_golden() {
        let recipe = load_golden_json();
        let tmp = std::env::temp_dir().join("test_recipe_output.md");
        save_to_file(&[recipe], &tmp).unwrap();
        let produced = std::fs::read_to_string(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);

        // Markdown comparison — exact byte match
        let golden = read_golden(Path::new("recipe_test.md"));
        // Normalize line endings
        let produced = produced.replace("\r\n", "\n");
        let golden = golden.replace("\r\n", "\n");
        assert_eq!(produced, golden, "Markdown output differs from golden file");
    }

    #[test]
    fn test_json_output_non_food() {
        let recipe = Recipe {
            name: "".into(),
            total_time: None,
            ingredients: vec![],
            seasonings: vec![],
            equipment: vec![],
            steps: vec![],
            tips: vec![],
            source_url: "".into(),
            is_food: false,
            reason: Some("旅游攻略".into()),
        };
        let tmp = std::env::temp_dir().join("test_non_food.json");
        save_to_file(&[recipe], &tmp).unwrap();
        let produced: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&tmp).unwrap()).unwrap();
        let _ = std::fs::remove_file(&tmp);

        assert_eq!(produced["is_food"], false);
        assert_eq!(produced["reason"], "旅游攻略");
        // None fields should be skipped
        assert!(produced.get("total_time").is_none());
    }

    #[test]
    fn test_markdown_output_empty_recipe() {
        let recipe = Recipe::default();
        let tmp = std::env::temp_dir().join("test_empty.md");
        save_to_file(&[recipe], &tmp).unwrap();
        let produced = std::fs::read_to_string(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);
        assert!(produced.starts_with("# "));
    }

    #[test]
    fn test_save_multi_markdown() {
        let r1 = Recipe {
            name: "菜1".into(),
            steps: vec![crate::models::Step { title: "步骤".into(), time: None, content: "内容1".into() }],
            ..Default::default()
        };
        let r2 = Recipe {
            name: "菜2".into(),
            steps: vec![crate::models::Step { title: "步骤".into(), time: None, content: "内容2".into() }],
            ..Default::default()
        };
        let tmp = std::env::temp_dir().join("test_multi.md");
        save_to_file(&[r1, r2], &tmp).unwrap();
        let produced = std::fs::read_to_string(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);
        assert!(produced.contains("# 菜1"));
        assert!(produced.contains("# 菜2"));
        assert!(produced.contains("---"));
    }
}
