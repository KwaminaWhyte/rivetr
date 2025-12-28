//! Volume models for persistent storage.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Volume mount for persistent storage
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Volume {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub host_path: String,
    pub container_path: String,
    pub read_only: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl Volume {
    /// Check if this volume is read-only
    pub fn is_read_only(&self) -> bool {
        self.read_only != 0
    }

    /// Get the Docker bind mount string (host_path:container_path[:ro])
    pub fn to_bind_mount(&self) -> String {
        if self.is_read_only() {
            format!("{}:{}:ro", self.host_path, self.container_path)
        } else {
            format!("{}:{}", self.host_path, self.container_path)
        }
    }
}

/// Response DTO for Volume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeResponse {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub host_path: String,
    pub container_path: String,
    pub read_only: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Volume> for VolumeResponse {
    fn from(v: Volume) -> Self {
        Self {
            id: v.id,
            app_id: v.app_id,
            name: v.name,
            host_path: v.host_path,
            container_path: v.container_path,
            read_only: v.read_only != 0,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

/// Request to create a volume
#[derive(Debug, Deserialize)]
pub struct CreateVolumeRequest {
    pub name: String,
    pub host_path: String,
    pub container_path: String,
    #[serde(default)]
    pub read_only: bool,
}

/// Request to update a volume
#[derive(Debug, Deserialize)]
pub struct UpdateVolumeRequest {
    pub name: Option<String>,
    pub host_path: Option<String>,
    pub container_path: Option<String>,
    pub read_only: Option<bool>,
}
