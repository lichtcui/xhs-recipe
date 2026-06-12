pub mod local;

use crate::models::Recipe;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Summary of a stored recipe, used for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeSummary {
    pub id: String,
    pub name: String,
    pub source_url: String,
    pub saved_at: u64,
    pub is_food: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialize error: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("Recipe not found: {id}")]
    NotFound { id: String },
}

/// Abstract recipe storage. Implementations can be local filesystem or
/// a database in the future.
#[async_trait]
pub trait Storage: Send + Sync {
    /// Save a recipe, returning the generated ID.
    async fn save(&self, recipe: &Recipe) -> Result<String, StorageError>;
    /// List all saved recipes, newest first.
    async fn list(&self) -> Result<Vec<RecipeSummary>, StorageError>;
    /// Get a single recipe by ID.
    async fn get(&self, id: &str) -> Result<Recipe, StorageError>;
    /// Delete a recipe by ID.
    async fn delete(&self, id: &str) -> Result<(), StorageError>;
}
