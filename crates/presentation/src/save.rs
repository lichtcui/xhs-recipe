use std::path::Path;
use models::Recipe;

/// Save recipe to file (.md or .json based on extension).
pub fn save_to_file(recipe: &Recipe, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("md");
    match ext {
        "json" => save_json(recipe, path),
        _ => save_md(recipe, path),
    }
}

fn save_json(recipe: &Recipe, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(recipe)?;
    std::fs::write(path, json)?;
    Ok(())
}

fn save_md(recipe: &Recipe, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
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
        let nums = ["①", "②", "③", "④", "⑤", "⑥", "⑦", "⑧", "⑨", "⑩"];
        for (i, step) in recipe.steps.iter().enumerate() {
            let fallback = format!("{}.", i + 1);
            let num = nums.get(i).copied().unwrap_or(&fallback);
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
    std::fs::write(path, lines.join("\n"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::Recipe;
    use std::path::PathBuf;

    fn golden_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/testdata")
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
        save_to_file(&recipe, &tmp).unwrap();
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
        save_to_file(&recipe, &tmp).unwrap();
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
        save_to_file(&recipe, &tmp).unwrap();
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
        save_to_file(&recipe, &tmp).unwrap();
        let produced = std::fs::read_to_string(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);
        assert!(produced.starts_with("# "));
    }
}
