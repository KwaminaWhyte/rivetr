//! SSH key models and DTOs.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SshKey {
    pub id: String,
    pub name: String,
    pub private_key: String,
    pub public_key: Option<String>,
    pub app_id: Option<String>,
    pub is_global: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// Response DTO that excludes the private key for security
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKeyResponse {
    pub id: String,
    pub name: String,
    pub public_key: Option<String>,
    pub app_id: Option<String>,
    pub is_global: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<SshKey> for SshKeyResponse {
    fn from(key: SshKey) -> Self {
        Self {
            id: key.id,
            name: key.name,
            public_key: key.public_key,
            app_id: key.app_id,
            is_global: key.is_global != 0,
            created_at: key.created_at,
            updated_at: key.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateSshKeyRequest {
    pub name: String,
    pub private_key: String,
    pub public_key: Option<String>,
    pub app_id: Option<String>,
    #[serde(default)]
    pub is_global: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSshKeyRequest {
    pub name: Option<String>,
    pub private_key: Option<String>,
    pub public_key: Option<String>,
    pub app_id: Option<String>,
    pub is_global: Option<bool>,
}
