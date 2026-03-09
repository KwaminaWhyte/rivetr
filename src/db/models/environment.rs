//! Project environment models and DTOs.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A project environment (e.g., production, staging, development)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectEnvironment {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_default: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// Response DTO for an environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentResponse {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl ProjectEnvironment {
    pub fn to_response(&self) -> EnvironmentResponse {
        EnvironmentResponse {
            id: self.id.clone(),
            project_id: self.project_id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            is_default: self.is_default != 0,
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateEnvironmentRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEnvironmentRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// An environment-scoped environment variable
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EnvironmentEnvVar {
    pub id: String,
    pub environment_id: String,
    pub key: String,
    pub value: String,
    pub is_secret: i32,
    pub created_at: String,
}

/// Response DTO that masks secret values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentEnvVarResponse {
    pub id: String,
    pub environment_id: String,
    pub key: String,
    pub value: String,
    pub is_secret: bool,
    pub created_at: String,
}

impl EnvironmentEnvVar {
    pub fn to_response(&self, reveal_secret: bool) -> EnvironmentEnvVarResponse {
        let value = if self.is_secret != 0 && !reveal_secret {
            "********".to_string()
        } else {
            self.value.clone()
        };
        EnvironmentEnvVarResponse {
            id: self.id.clone(),
            environment_id: self.environment_id.clone(),
            key: self.key.clone(),
            value,
            is_secret: self.is_secret != 0,
            created_at: self.created_at.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateEnvironmentEnvVarRequest {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub is_secret: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEnvironmentEnvVarRequest {
    pub value: Option<String>,
    pub is_secret: Option<bool>,
}
