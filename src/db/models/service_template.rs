//! Service template models for one-click service deployments.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Template categories for organizing services
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TemplateCategory {
    Monitoring,
    Database,
    Storage,
    Development,
    Analytics,
    Networking,
    Security,
}

impl std::fmt::Display for TemplateCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Monitoring => write!(f, "monitoring"),
            Self::Database => write!(f, "database"),
            Self::Storage => write!(f, "storage"),
            Self::Development => write!(f, "development"),
            Self::Analytics => write!(f, "analytics"),
            Self::Networking => write!(f, "networking"),
            Self::Security => write!(f, "security"),
        }
    }
}

impl std::str::FromStr for TemplateCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "monitoring" => Ok(Self::Monitoring),
            "database" => Ok(Self::Database),
            "storage" => Ok(Self::Storage),
            "development" => Ok(Self::Development),
            "analytics" => Ok(Self::Analytics),
            "networking" => Ok(Self::Networking),
            "security" => Ok(Self::Security),
            _ => Err(format!("Unknown template category: {}", s)),
        }
    }
}

impl From<String> for TemplateCategory {
    fn from(s: String) -> Self {
        s.parse().unwrap_or(Self::Development)
    }
}

/// Environment variable schema entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvSchemaEntry {
    pub name: String,
    pub label: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: String,
    #[serde(default)]
    pub secret: bool,
}

/// Service template entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ServiceTemplate {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub icon: Option<String>,
    pub compose_template: String,
    pub env_schema: Option<String>,
    pub is_builtin: i32,
    pub created_at: String,
}

impl ServiceTemplate {
    /// Get the category as enum
    pub fn get_category(&self) -> TemplateCategory {
        TemplateCategory::from(self.category.clone())
    }

    /// Check if this is a built-in template
    pub fn is_builtin(&self) -> bool {
        self.is_builtin != 0
    }

    /// Parse environment schema from JSON
    pub fn get_env_schema(&self) -> Vec<EnvSchemaEntry> {
        self.env_schema
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }

    /// Convert to API response
    pub fn to_response(&self) -> ServiceTemplateResponse {
        ServiceTemplateResponse {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            category: self.category.clone(),
            icon: self.icon.clone(),
            compose_template: self.compose_template.clone(),
            env_schema: self.get_env_schema(),
            is_builtin: self.is_builtin(),
            created_at: self.created_at.clone(),
        }
    }
}

/// Response DTO for ServiceTemplate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceTemplateResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub icon: Option<String>,
    pub compose_template: String,
    pub env_schema: Vec<EnvSchemaEntry>,
    pub is_builtin: bool,
    pub created_at: String,
}

/// Request to deploy a service template
#[derive(Debug, Deserialize)]
pub struct DeployTemplateRequest {
    /// Service name for this deployment
    pub name: String,
    /// Environment variable values (key-value pairs)
    #[serde(default)]
    pub env_vars: std::collections::HashMap<String, String>,
    /// Associated project ID
    pub project_id: Option<String>,
}

/// Response after deploying a template
#[derive(Debug, Serialize)]
pub struct DeployTemplateResponse {
    pub service_id: String,
    pub name: String,
    pub template_id: String,
    pub status: String,
    pub message: String,
}

/// Request to create a custom service template
#[derive(Debug, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub description: Option<String>,
    pub category: TemplateCategory,
    pub icon: Option<String>,
    pub compose_template: String,
    pub env_schema: Option<Vec<EnvSchemaEntry>>,
}
