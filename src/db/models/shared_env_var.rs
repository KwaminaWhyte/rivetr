//! Shared environment variable models and DTOs.
//!
//! Team-level and project-level shared variables that are inherited by apps.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ---------------------------------------------------------------------------
// Team Env Var
// ---------------------------------------------------------------------------

/// A team-level shared environment variable.
/// All apps within the team inherit these variables (lowest priority).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TeamEnvVar {
    pub id: String,
    pub team_id: String,
    pub key: String,
    pub value: String,
    pub is_secret: i32,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Response DTO that masks secret values for team env vars
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamEnvVarResponse {
    pub id: String,
    pub team_id: String,
    pub key: String,
    pub value: String,
    pub is_secret: bool,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl TeamEnvVar {
    pub fn to_response(&self, reveal_secret: bool) -> TeamEnvVarResponse {
        let value = if self.is_secret != 0 && !reveal_secret {
            "********".to_string()
        } else {
            self.value.clone()
        };
        TeamEnvVarResponse {
            id: self.id.clone(),
            team_id: self.team_id.clone(),
            key: self.key.clone(),
            value,
            is_secret: self.is_secret != 0,
            description: self.description.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateTeamEnvVarRequest {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub is_secret: bool,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTeamEnvVarRequest {
    pub value: Option<String>,
    pub is_secret: Option<bool>,
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// Project Env Var
// ---------------------------------------------------------------------------

/// A project-level shared environment variable.
/// All apps within the project inherit these variables (overrides team vars).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectEnvVar {
    pub id: String,
    pub project_id: String,
    pub key: String,
    pub value: String,
    pub is_secret: i32,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Response DTO that masks secret values for project env vars
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEnvVarResponse {
    pub id: String,
    pub project_id: String,
    pub key: String,
    pub value: String,
    pub is_secret: bool,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl ProjectEnvVar {
    pub fn to_response(&self, reveal_secret: bool) -> ProjectEnvVarResponse {
        let value = if self.is_secret != 0 && !reveal_secret {
            "********".to_string()
        } else {
            self.value.clone()
        };
        ProjectEnvVarResponse {
            id: self.id.clone(),
            project_id: self.project_id.clone(),
            key: self.key.clone(),
            value,
            is_secret: self.is_secret != 0,
            description: self.description.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectEnvVarRequest {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub is_secret: bool,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectEnvVarRequest {
    pub value: Option<String>,
    pub is_secret: Option<bool>,
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// Resolved Env Var (inheritance chain view)
// ---------------------------------------------------------------------------

/// The source level for a resolved environment variable
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EnvVarSource {
    /// Set at the app level (highest priority)
    App,
    /// Inherited from the project environment
    Environment,
    /// Inherited from the project
    Project,
    /// Inherited from the team (lowest priority)
    Team,
}

/// A resolved environment variable showing effective value and its source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedEnvVar {
    pub key: String,
    /// Masked as `***` for secrets
    pub value: String,
    pub is_secret: bool,
    pub source: EnvVarSource,
    pub description: Option<String>,
}
