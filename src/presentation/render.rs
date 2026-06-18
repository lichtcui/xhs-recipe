use crate::models::Recipe;
use colored::*;

/// Returns true if the amount string is a generic qualifier with no real information.
fn is_generic_amount(s: &str) -> bool {
    matches!(s.trim(), "适量" | "少许" | "适量即可" | "少量" | "若干" | "一点")
}

/// Formats an amount string, skipping generic qualifiers.
fn fmt_amount(amt: &str) -> Option<String> {
    if is_generic_amount(amt) { None } else { Some(format!(" {}", amt)) }
}

/// Render multiple recipes to terminal with dividers between them.
pub fn render_terminal_multi(recipes: &[Recipe]) {
    if recipes.is_empty() {
        return;
    }
    if recipes.len() == 1 {
        return render_terminal(&recipes[0]);
    }

    let food_recipes: Vec<&Recipe> = recipes.iter().filter(|r| r.is_food).collect();
    if food_recipes.is_empty() {
        // None are food — render first one which explains why
        return render_terminal(&recipes[0]);
    }

    let total = recipes.len();
    let food_total = food_recipes.len();
    if food_total < total {
        println!("\n  📋 {} (共{}个，其中{}个识别为美食)", format!("合集共 {} 个菜谱", total).green().bold(), total, food_total);
    } else {
        println!("\n  📋 {}", format!("合集共 {} 个菜谱", total).green().bold());
    }
    for (i, recipe) in food_recipes.iter().enumerate() {
        println!("\n  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  📖 {} / {}", format!("第{}个", i + 1).bold(), total);
        render_terminal(recipe);
    }
    println!("  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

/// Render recipe to terminal with ANSI colors, compact layout.
pub fn render_terminal(recipe: &Recipe) {
    if !recipe.is_food {
        render_not_food(recipe);
        return;
    }

    // Name and total time on the same line
    let time_str = recipe.total_time.as_ref()
        .map(|t| format!("  ⏱ {}", t.yellow()))
        .unwrap_or_default();
    println!("  🍖 {}{}", recipe.name.green().bold(), time_str);

    if !recipe.ingredients.is_empty() {
        println!();
        println!("  🥩 {}", "食材".bold());
        let items: Vec<String> = recipe.ingredients.iter().map(|ing| {
            let mut s = ing.name.clone();
            if let Some(ref amt) = ing.amount {
                if let Some(fa) = fmt_amount(amt) {
                    s.push_str(&fa);
                }
            }
            if let Some(ref prep) = ing.prep {
                s.push_str(&format!("（{}）", prep));
            }
            s
        }).collect();
        println!("    · {}", items.join("、"));
    }

    if !recipe.seasonings.is_empty() {
        println!();
        println!("  🧂 {}", "调料".bold());
        let items: Vec<String> = recipe.seasonings.iter().map(|s| {
            let mut line = s.name.clone();
            if let Some(ref amt) = s.amount {
                if let Some(fa) = fmt_amount(amt) {
                    line.push_str(&fa);
                }
            }
            if let Some(ref prep) = s.prep {
                line.push_str(&format!("（{}）", prep));
            }
            line
        }).collect();
        println!("    · {}", items.join("、"));
    }

    if !recipe.equipment.is_empty() {
        println!();
        println!("  🔧 {}", "器具".bold());
        println!("    · {}", recipe.equipment.join("、"));
    }

    if !recipe.steps.is_empty() {
        println!();
        println!("  📝 {}", "步骤".bold());
        for (i, step) in recipe.steps.iter().enumerate() {
            let fallback = format!("{}.", i + 1);
            let num = crate::STEP_NUMS.get(i).copied().unwrap_or(&fallback);
            let time_str = step.time.as_ref().map_or(String::new(), |t| format!("（{}）", t.yellow()));
            println!("  {} {} {}", num.bold(), step.title.bold(), time_str);
            for line in step.content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    println!("    {}", trimmed);
                }
            }
        }
    }

    if !recipe.tips.is_empty() {
        println!();
        let tips_short: Vec<String> = recipe.tips.iter()
            .map(|t| t.trim_end_matches('。').to_string())
            .collect();
        println!("  💡 {}", "小贴士".bold());
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
