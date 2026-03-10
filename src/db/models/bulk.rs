//! Bulk operations models — config snapshots.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A saved snapshot of an app's configuration and (masked) env vars.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConfigSnapshot {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub description: Option<String>,
    /// JSON-serialized app config (AppResponse shape)
    pub config_json: String,
    /// JSON-serialized env vars (secrets masked)
    pub env_vars_json: String,
    pub created_by: Option<String>,
    pub created_at: String,
}

/// Request to create a named snapshot
#[derive(Debug, Deserialize)]
pub struct CreateSnapshotRequest {
    pub name: String,
    pub description: Option<String>,
}
