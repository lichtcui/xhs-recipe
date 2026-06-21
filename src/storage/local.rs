use crate::models::Recipe;
use crate::storage::{RecipeSummary, Storage, StorageError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Internal wrapper stored on disk alongside each recipe.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredRecipe {
    id: String,
    saved_at: u64,
    recipe: Recipe,
}

/// File-system backed recipe storage.
///
/// Each recipe is saved as `{id}.json` in a directory
/// (default: `~/.xhs-recipe/recipes/`).
pub struct LocalStorage {
    dir: PathBuf,
}

impl LocalStorage {
    /// Create storage rooted at `dir`.
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

}

impl Default for LocalStorage {
    /// Default storage location (`~/.xhs-recipe/recipes/`).
    fn default() -> Self {
        Self {
            dir: crate::home_dir().join(".xhs-recipe").join("recipes"),
        }
    }
}

// ── ID generation ──────────────────────────────────────────────────

fn generate_id(source_url: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let hash = source_url
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    format!("{ts:016x}{hash:016x}{seq:04x}")
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ── Private helpers ──────────────────────────────────────────────

impl LocalStorage {
    fn recipe_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    /// Scan all stored recipes and return those matching `source_url`.
    async fn find_all_by_source_url(&self, source_url: &str) -> Result<Vec<StoredRecipe>, StorageError> {
        let mut dir = match tokio::fs::read_dir(&self.dir).await {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(e.into()),
        };

        let mut results = Vec::new();
        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                if let Ok(stored) = Self::read_stored(&path).await {
                    if stored.recipe.source_url == source_url {
                        results.push(stored);
                    }
                }
            }
        }
        Ok(results)
    }

    /// Read a StoredRecipe from a JSON file path.
    async fn read_stored(path: &std::path::Path) -> Result<StoredRecipe, StorageError> {
        let data = tokio::fs::read_to_string(path).await?;
        Ok(serde_json::from_str(&data)?)
    }
}

// ── Storage trait impl ────────────────────────────────────────────

#[async_trait]
impl Storage for LocalStorage {
    async fn save(&self, recipe: &Recipe) -> Result<String, StorageError> {
        // Dedup by (source_url, name) pair: skip if a recipe with the same URL and name exists.
        let existing = self.find_all_by_source_url(&recipe.source_url).await?;
        for stored in &existing {
            if stored.recipe.name == recipe.name {
                return Ok(stored.id.clone());
            }
        }

        let id = generate_id(&recipe.source_url);
        let stored = StoredRecipe {
            id: id.clone(),
            saved_at: now_secs(),
            recipe: recipe.clone(),
        };
        let json = serde_json::to_string_pretty(&stored)?;

        // Create directory tree if needed, then write.
        tokio::fs::create_dir_all(&self.dir).await?;
        let path = self.recipe_path(&id);
        tokio::fs::write(&path, json).await?;
        Ok(id)
    }

    async fn list(&self) -> Result<Vec<RecipeSummary>, StorageError> {
        let dir = match std::fs::read_dir(&self.dir) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(e.into()),
        };

        let mut summaries: Vec<RecipeSummary> = dir
            .filter_map(|entry| match entry {
                Ok(e) => Some(e),
                Err(e) => {
                    eprintln!("  ⚠ 读取存储目录失败: {}", e);
                    None
                }
            })
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            .filter_map(|e| {
                let data = std::fs::read_to_string(e.path()).ok()?;
                let stored: StoredRecipe = serde_json::from_str(&data).ok()?;
                Some(RecipeSummary {
                    id: stored.id,
                    name: stored.recipe.name,
                    source_url: stored.recipe.source_url,
                    saved_at: stored.saved_at,
                    is_food: stored.recipe.is_food,
                    cover_image_url: stored.recipe.cover_image_url.clone(),
                    total_time: stored.recipe.total_time.clone(),
                    tags: stored.recipe.tags.clone().unwrap_or_default(),
                })
            })
            .collect();

        summaries.sort_by(|a, b| b.saved_at.cmp(&a.saved_at).then_with(|| b.id.cmp(&a.id)));
        Ok(summaries)
    }

    async fn get(&self, id: &str) -> Result<Recipe, StorageError> {
        let path = self.recipe_path(id);
        let data = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => StorageError::NotFound { id: id.to_string() },
                _ => e.into(),
            })?;
        let stored: StoredRecipe = serde_json::from_str(&data)?;
        let mut recipe = stored.recipe;
        recipe.id = Some(stored.id);
        Ok(recipe)
    }

    async fn get_by_source_url(&self, source_url: &str) -> Result<Vec<Recipe>, StorageError> {
        let existing = self.find_all_by_source_url(source_url).await?;
        Ok(existing.into_iter().map(|s| s.recipe).collect())
    }

    async fn delete(&self, id: &str) -> Result<(), StorageError> {
        let path = self.recipe_path(id);
        tokio::fs::remove_file(&path)
            .await
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => StorageError::NotFound { id: id.to_string() },
                _ => e.into(),
            })
    }
}

// ── Relative time helper ──────────────────────────────────────────

/// Format a Unix timestamp as a human-readable relative time (Chinese).
pub fn relative_time(secs: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let diff = now.saturating_sub(secs);
    if diff < 60 {
        format!("{diff}秒前")
    } else if diff < 3600 {
        format!("{}分钟前", diff / 60)
    } else if diff < 86400 {
        format!("{}小时前", diff / 3600)
    } else {
        format!("{}天前", diff / 86400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Recipe;
    use tempfile::TempDir;

    fn sample_recipe() -> Recipe {
        Recipe {
            name: "番茄炒蛋".into(),
            total_time: Some("10分钟".into()),
            ingredients: vec![],
            seasonings: vec![],
            equipment: vec![],
            steps: vec![],
            tips: vec![],
            source_url: "https://example.com/test".into(),
            is_food: true,
            reason: None,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_save_and_get() {
        let tmp = TempDir::new().unwrap();
        let store = LocalStorage::new(tmp.path().join("recipes"));

        let recipe = sample_recipe();
        let id = store.save(&recipe).await.unwrap();
        assert!(!id.is_empty());

        let loaded = store.get(&id).await.unwrap();
        assert_eq!(loaded.name, "番茄炒蛋");
        assert_eq!(loaded.source_url, "https://example.com/test");
    }

    #[tokio::test]
    async fn test_get_not_found() {
        let tmp = TempDir::new().unwrap();
        let store = LocalStorage::new(tmp.path().join("recipes"));

        let err = store.get("nonexistent").await.unwrap_err();
        assert!(matches!(err, StorageError::NotFound { .. }));
    }

    #[tokio::test]
    async fn test_list_empty() {
        let tmp = TempDir::new().unwrap();
        let store = LocalStorage::new(tmp.path().join("recipes"));

        let list = store.list().await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_list_multiple() {
        let tmp = TempDir::new().unwrap();
        let store = LocalStorage::new(tmp.path().join("recipes"));

        let r1 = sample_recipe();
        let r2 = Recipe {
            name: "红烧肉".into(),
            source_url: "https://example.com/other".into(),
            ..sample_recipe()
        };

        store.save(&r1).await.unwrap();
        // Small delay so timestamps differ
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        store.save(&r2).await.unwrap();

        let list = store.list().await.unwrap();
        assert_eq!(list.len(), 2);
        // Newest first
        assert_eq!(list[0].name, "红烧肉");
        assert_eq!(list[1].name, "番茄炒蛋");
    }

    #[tokio::test]
    async fn test_delete() {
        let tmp = TempDir::new().unwrap();
        let store = LocalStorage::new(tmp.path().join("recipes"));

        let id = store.save(&sample_recipe()).await.unwrap();
        assert!(store.get(&id).await.is_ok());

        store.delete(&id).await.unwrap();
        assert!(matches!(
            store.get(&id).await.unwrap_err(),
            StorageError::NotFound { .. }
        ));
    }

    #[tokio::test]
    async fn test_delete_not_found() {
        let tmp = TempDir::new().unwrap();
        let store = LocalStorage::new(tmp.path().join("recipes"));

        let err = store.delete("nonexistent").await.unwrap_err();
        assert!(matches!(err, StorageError::NotFound { .. }));
    }

    #[test]
    fn test_generate_id_unique() {
        let id1 = generate_id("https://example.com/1");
        let id2 = generate_id("https://example.com/2");
        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn test_save_dedup_same_url_and_name() {
        let tmp = TempDir::new().unwrap();
        let store = LocalStorage::new(tmp.path().join("recipes"));

        let id1 = store.save(&sample_recipe()).await.unwrap();
        let id2 = store.save(&sample_recipe()).await.unwrap();
        assert_eq!(id1, id2, "same source_url + name should return same ID");

        let list = store.list().await.unwrap();
        assert_eq!(list.len(), 1, "duplicate saves should not create extra entries");
    }

    #[tokio::test]
    async fn test_save_multi_recipes_same_url() {
        let tmp = TempDir::new().unwrap();
        let store = LocalStorage::new(tmp.path().join("recipes"));

        let r1 = sample_recipe(); // name: "番茄炒蛋", source_url: "https://example.com/test"
        let r2 = Recipe {
            name: "红烧肉".into(),
            source_url: "https://example.com/test".into(), // same URL, different name
            ..sample_recipe()
        };

        let id1 = store.save(&r1).await.unwrap();
        let id2 = store.save(&r2).await.unwrap();
        assert_ne!(id1, id2, "different names from same URL should get different IDs");

        let recipes = store.get_by_source_url("https://example.com/test").await.unwrap();
        assert_eq!(recipes.len(), 2, "both recipes should be returned");
    }

    #[test]
    fn test_relative_time() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        assert_eq!(relative_time(now), "0秒前");
        assert_eq!(relative_time(now - 120), "2分钟前");
        assert_eq!(relative_time(now - 7200), "2小时前");
        assert_eq!(relative_time(now - 172800), "2天前");
    }
}
