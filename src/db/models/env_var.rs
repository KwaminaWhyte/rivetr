//! Environment variable models and DTOs.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EnvVar {
    pub id: String,
    pub app_id: String,
    pub key: String,
    pub value: String,
    pub is_secret: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// Response DTO that masks secret values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVarResponse {
    pub id: String,
    pub app_id: String,
    pub key: String,
    pub value: String,
    pub is_secret: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl EnvVar {
    pub fn to_response(&self, reveal_secret: bool) -> EnvVarResponse {
        let value = if self.is_secret != 0 && !reveal_secret {
            "********".to_string()
        } else {
            self.value.clone()
        };
        EnvVarResponse {
            id: self.id.clone(),
            app_id: self.app_id.clone(),
            key: self.key.clone(),
            value,
            is_secret: self.is_secret != 0,
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateEnvVarRequest {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub is_secret: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEnvVarRequest {
    pub value: Option<String>,
    pub is_secret: Option<bool>,
}
