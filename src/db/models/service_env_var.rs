//! Environment variables of a Docker Compose service.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ServiceEnvVar {
    pub id: String,
    pub service_id: String,
    pub key: String,
    /// Encrypted at rest when an encryption key is configured.
    pub value: String,
    pub is_secret: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEnvVarResponse {
    pub id: String,
    pub service_id: String,
    pub key: String,
    /// `********` for secrets unless revealed.
    pub value: String,
    pub is_secret: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl ServiceEnvVar {
    /// Response with `plain` as the (already decrypted) value.
    pub fn to_response(&self, plain: &str, reveal: bool) -> ServiceEnvVarResponse {
        ServiceEnvVarResponse {
            id: self.id.clone(),
            service_id: self.service_id.clone(),
            key: self.key.clone(),
            value: if self.is_secret != 0 && !reveal {
                "********".to_string()
            } else {
                plain.to_string()
            },
            is_secret: self.is_secret != 0,
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateServiceEnvVarRequest {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub is_secret: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateServiceEnvVarRequest {
    pub value: Option<String>,
    pub is_secret: Option<bool>,
}

/// Paste a `.env` file: every `KEY=VALUE` line becomes a variable.
#[derive(Debug, Deserialize)]
pub struct ImportServiceEnvRequest {
    pub content: String,
    /// Replace values of keys that already exist (default: keep them).
    #[serde(default)]
    pub overwrite: bool,
    /// Mark imported variables as secret (default: true).
    #[serde(default = "default_true")]
    pub is_secret: bool,
}

fn default_true() -> bool {
    true
}
