use crate::models::Recipe;
use colored::*;

/// Render recipe to terminal with ANSI colors, matching Python `rich` output.
pub fn render_terminal(recipe: &Recipe) {
    if !recipe.is_food {
        render_not_food(recipe);
        return;
    }

    println!();
    println!("  {} {}", "🍖".to_string(), recipe.name.green().bold());
    let mut time_parts = Vec::new();
    if let Some(ref t) = recipe.total_time {
        time_parts.push(format!("⏱ {}", t.yellow()));
    }
    time_parts.push("👨‍👩‍👧‍👦 约2-3人份".to_string());
    println!("  {}", time_parts.join(" ｜"));

    if !recipe.ingredients.is_empty() {
        println!("\n  {} {}", "🥩".to_string(), "食材".bold());
        for ing in &recipe.ingredients {
            let mut parts = vec![format!("· {}", ing.name.cyan())];
            if let Some(ref amt) = ing.amount {
                parts.push(format!(" {}", amt));
            }
            if let Some(ref prep) = ing.prep {
                parts.push(format!("（{}）", prep));
            }
            println!("    {}", parts.concat());
        }
    }

    if !recipe.seasonings.is_empty() {
        println!("\n  {} {}", "🧂".to_string(), "调料".bold());
        let items: Vec<String> = recipe.seasonings.iter().map(|s| {
            let mut line = s.name.clone();
            if let Some(ref amt) = s.amount {
                line.push_str(&format!(" {}", amt));
            }
            if let Some(ref prep) = s.prep {
                line.push_str(&format!("（{}）", prep));
            }
            line
        }).collect();
        println!("    · {}", items.join("、"));
    }

    if !recipe.equipment.is_empty() {
        println!("\n  {} {}", "🔧".to_string(), "器具".bold());
        println!("    · {}", recipe.equipment.join("、"));
    }

    if !recipe.steps.is_empty() {
        println!("\n  {} {}", "📝".to_string(), "步骤".bold());
        let nums = ["①", "②", "③", "④", "⑤", "⑥", "⑦", "⑧", "⑨", "⑩"];
        for (i, step) in recipe.steps.iter().enumerate() {
            let fallback = format!("{}.", i + 1);
            let num = nums.get(i).copied().unwrap_or(&fallback);
            let time_str = step.time.as_ref().map_or(String::new(), |t| format!("（{}）", t.yellow()));
            println!("\n  {} {} {}", num.bold(), step.title.bold(), time_str);
            for line in step.content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    println!("     {}", trimmed);
                }
            }
        }
    }

    if !recipe.tips.is_empty() {
        let tips_short: Vec<String> = recipe.tips.iter()
            .map(|t| t.trim_end_matches('。').to_string())
            .collect();
        println!("\n  {} {}", "💡".to_string(), "小贴士".bold());
        println!("    {}", tips_short.join(" · "));
    }
}

fn render_not_food(recipe: &Recipe) {
    let reason = recipe.reason.as_deref().unwrap_or("");
    println!("{}", "⚠ 此内容与美食无关".yellow());
    if !reason.is_empty() {
        println!("{}", reason);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Ingredient, Step};

    fn sample_recipe() -> Recipe {
        Recipe {
            name: "蒜香椒盐烤排骨".into(),
            total_time: Some("1小时25分钟".into()),
            ingredients: vec![
                Ingredient {
                    name: "排骨".into(),
                    amount: Some("适量".into()),
                    prep: Some("清洗干净，擦干水分".into()),
                    category: Some("食材".into()),
                },
                Ingredient {
                    name: "面粉".into(),
                    amount: Some("1勺".into()),
                    prep: Some("用于清洗排骨".into()),
                    category: Some("食材".into()),
                },
            ],
            seasonings: vec![
                Ingredient {
                    name: "蒜".into(),
                    amount: Some("适量".into()),
                    prep: None,
                    category: Some("调料".into()),
                },
                Ingredient {
                    name: "椒盐".into(),
                    amount: Some("适量".into()),
                    prep: None,
                    category: Some("调料".into()),
                },
            ],
            equipment: vec!["空气炸锅".into(), "厨房纸巾".into(), "碗".into(), "喷油壶".into()],
            steps: vec![
                Step {
                    title: "清洗排骨".into(),
                    time: Some("约5分钟".into()),
                    content: "排骨加入1勺面粉，倒入适量清水，用手抓洗清洗出血水，然后用清水冲洗干净，用厨房纸巾擦干水分".into(),
                },
                Step {
                    title: "烤制".into(),
                    time: Some("25分钟".into()),
                    content: "空气炸锅底部倒入少许清水，放入腌制好的排骨，表面喷一点油，设置180度烤25分钟，中途翻面一次".into(),
                },
            ],
            tips: vec![
                "腌制时间建议至少1小时，时间越久越入味".into(),
                "适合宝宝吃，非油炸更健康，肉嫩易脱骨".into(),
            ],
            source_url: "https://example.com".into(),
            is_food: true,
            reason: None,
        }
    }

    #[test]
    fn test_render_not_food() {
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
            reason: Some("这是一篇旅游攻略".into()),
        };
        render_terminal(&recipe); // visual check
    }

    #[test]
    fn test_render_food_recipe() {
        let recipe = sample_recipe();
        render_terminal(&recipe); // visual check
    }
}
