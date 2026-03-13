//! App deployment patch models.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A file-level patch applied to the cloned repository before building.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AppPatch {
    pub id: String,
    pub app_id: String,
    /// Relative file path inside the repository (e.g. "config/production.env")
    pub file_path: String,
    /// File content to write (unused for 'delete' operation)
    pub content: String,
    /// Operation: "create", "append", or "delete"
    pub operation: String,
    pub is_enabled: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl AppPatch {
    pub fn is_enabled(&self) -> bool {
        self.is_enabled != 0
    }
}

/// Request to create a new patch
#[derive(Debug, Deserialize)]
pub struct CreateAppPatchRequest {
    pub file_path: String,
    pub content: Option<String>,
    /// "create", "append", or "delete"
    #[serde(default = "default_operation")]
    pub operation: String,
    #[serde(default = "default_enabled")]
    pub is_enabled: bool,
}

/// Request to update an existing patch
#[derive(Debug, Deserialize)]
pub struct UpdateAppPatchRequest {
    pub file_path: Option<String>,
    pub content: Option<String>,
    pub operation: Option<String>,
    pub is_enabled: Option<bool>,
}

fn default_operation() -> String {
    "create".to_string()
}

fn default_enabled() -> bool {
    true
}
