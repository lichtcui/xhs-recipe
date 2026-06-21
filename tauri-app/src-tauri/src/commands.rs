use tauri::{AppHandle, Emitter};
use xhs_recipe::{
    sources, textifier, analyzer,
    pipeline::{detect_collection_count, extract_collection},
    storage::local::LocalStorage,
    storage::{Storage, RecipeSummary},
    models::*,
};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

// ── Settings ──────────────────────────────────────────

#[derive(Deserialize)]
pub struct ExtractSettings {
    pub asr_model: String,
    pub ocr_images: bool,
    pub llm_model: String,
    pub api_key: Option<String>,
    #[allow(dead_code)]
    pub timeout_secs: u64,
}

// ── Progress ──────────────────────────────────────────

#[derive(Clone, Serialize)]
pub struct ProgressEvent {
    pub stage: String,
    pub detail: String,
}

// ── Prerequisites ─────────────────────────────────────

#[derive(Serialize)]
pub struct PrerequisiteStatus {
    pub ffmpeg: bool,
    pub tesseract: bool,
    pub qwen_asr: bool,
    pub cookies_exist: bool,
}

#[tauri::command]
pub fn check_prerequisites() -> PrerequisiteStatus {
    PrerequisiteStatus {
        ffmpeg: xhs_recipe::which("ffmpeg").is_some(),
        tesseract: xhs_recipe::which("tesseract").is_some(),
        qwen_asr: xhs_recipe::which("qwen-asr").is_some(),
        cookies_exist: sources::xiaohongshu::auth::has_cookies(),
    }
}

// ── Main Extract ──────────────────────────────────────

#[tauri::command]
pub async fn extract(
    app: AppHandle,
    url: String,
    settings: ExtractSettings,
) -> Result<Vec<Recipe>, String> {
    let emit = |stage: &str, detail: &str| {
        let _ = app.emit("extract:progress", ProgressEvent {
            stage: stage.to_string(),
            detail: detail.to_string(),
        });
    };

    // Stage 1: Fetch
    emit("fetching", &url);
    let raw = sources::fetch(&url).await.map_err(|e| e.to_string())?;

    // Stage 2-4: Textify with progress callback
    let on_progress: Arc<dyn Fn(&str) + Send + Sync> = Arc::new({
        let app = app.clone();
        move |stage: &str| {
            let mapped = match stage {
                "downloading" => "downloading",
                "ocr" => "ocr",
                "asr" => "asr",
                _ => "downloading",
            };
            let _ = app.emit("extract:progress", ProgressEvent {
                stage: mapped.to_string(),
                detail: String::new(),
            });
        }
    });
    let text = textifier::process(&raw, &settings.asr_model, settings.ocr_images, Some(on_progress))
        .await
        .map_err(|e| e.to_string())?;

    // Stage 5: Analyze
    emit("analyzing", "");
    let client = analyzer::RealHttpClient::new();
    let mut recipes = if settings.ocr_images && !raw.has_video && !text.image_texts.is_empty() {
        let count = detect_collection_count(&text.title);
        let total = count.unwrap_or(text.image_texts.len());
        if count.is_some() && total > 1 {
            extract_collection(&text, total, &settings.llm_model, settings.api_key.as_deref())
                .await
                .map_err(|e| e.to_string())
        } else {
            analyzer::extract_recipe(&client, &text.full_text, &[], &settings.llm_model, settings.api_key.as_deref())
                .await
                .map_err(|e| e.to_string())
        }
    } else {
        analyzer::extract_recipe(&client, &text.full_text, &[], &settings.llm_model, settings.api_key.as_deref())
            .await
            .map_err(|e| e.to_string())
    }?;

    // Set source_url and new metadata fields on all recipes
    let cover_img = raw.image_urls.first().cloned();
    let all_images = raw.image_urls.clone();
    let raw_text = text.full_text.clone();
    for recipe in &mut recipes {
        recipe.source_url = raw.source_url.clone();
        recipe.cover_image_url = cover_img.clone();
        recipe.image_urls = Some(all_images.clone());
        recipe.raw_text = Some(raw_text.clone());
    }

    // Auto-save only substantial food recipes
    let store = LocalStorage::default();
    let total = recipes.len();
    let food_count = recipes.iter().filter(|r| r.is_food && r.is_substantial()).count();
    for recipe in &mut recipes {
        if recipe.is_food && recipe.is_substantial() {
            if let Ok(id) = store.save(recipe).await {
                recipe.id = Some(id);
            }
        }
    }

    if total == 0 {
        emit("done", "未提取到任何菜谱");
    } else if food_count == 0 {
        emit("done", &format!("提取完成，但未识别到有效菜谱（共 {} 条）", total));
    } else {
        emit("done", &format!("已保存 {} 个菜谱", food_count));
    }
    Ok(recipes)
}

// ── Cookie Management ─────────────────────────────────

#[tauri::command]
pub async fn import_cookies(cookie_json: String) -> Result<String, String> {
    let cookies: Vec<sources::xiaohongshu::auth::Cookie> =
        serde_json::from_str(&cookie_json).map_err(|e| e.to_string())?;
    sources::xiaohongshu::auth::save_cookies(&cookies);
    Ok(format!("已导入 {} 个 Cookie", cookies.len()))
}

#[tauri::command]
pub fn check_cookies() -> bool {
    sources::xiaohongshu::auth::has_cookies()
}

// ── Saved Recipes ─────────────────────────────────────

#[tauri::command]
pub async fn list_recipes() -> Result<Vec<RecipeSummary>, String> {
    LocalStorage::default().list().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_recipe(id: String) -> Result<Recipe, String> {
    LocalStorage::default().get(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_recipe(id: String) -> Result<(), String> {
    LocalStorage::default().delete(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_recipe(recipe: Recipe) -> Result<String, String> {
    LocalStorage::default().save(&recipe).await.map_err(|e| e.to_string())
}

/// Extract text only (no LLM analysis). Returns raw text + cover image for the Bridge flow.
#[derive(Serialize)]
pub struct ExtractTextResult {
    pub raw_text: String,
    pub cover_image_url: Option<String>,
    pub image_urls: Vec<String>,
    pub title: String,
    pub source_url: String,
}

#[tauri::command]
pub async fn extract_text(
    app: AppHandle,
    url: String,
    settings: ExtractSettings,
) -> Result<ExtractTextResult, String> {
    let emit = |stage: &str, detail: &str| {
        let _ = app.emit("extract:progress", ProgressEvent {
            stage: stage.to_string(),
            detail: detail.to_string(),
        });
    };

    emit("fetching", &url);
    let raw = sources::fetch(&url).await.map_err(|e| e.to_string())?;

    let on_progress: std::sync::Arc<dyn Fn(&str) + Send + Sync> = std::sync::Arc::new({
        let app = app.clone();
        move |stage: &str| {
            let mapped = match stage {
                "downloading" => "downloading",
                "ocr" => "ocr",
                "asr" => "asr",
                _ => "downloading",
            };
            let _ = app.emit("extract:progress", ProgressEvent {
                stage: mapped.to_string(),
                detail: String::new(),
            });
        }
    });

    let text = textifier::process(&raw, &settings.asr_model, settings.ocr_images, Some(on_progress))
        .await
        .map_err(|e| e.to_string())?;

    emit("done", "");
    Ok(ExtractTextResult {
        raw_text: text.full_text,
        cover_image_url: raw.image_urls.first().cloned(),
        image_urls: raw.image_urls,
        title: text.title,
        source_url: text.source_url,
    })
}

/// Analyze raw text with LLM (no fetch/textify). Returns structured recipes.
#[tauri::command]
pub async fn analyze_recipe(
    text: String,
    model: String,
    api_key: Option<String>,
) -> Result<Vec<Recipe>, String> {
    let client = analyzer::RealHttpClient::new();
    let recipes = analyzer::extract_recipe(&client, &text, &[], &model, api_key.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(recipes)
}
