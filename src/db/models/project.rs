//! Project models and DTOs.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::app::App;
use super::database::ManagedDatabaseResponse;
use super::service::ServiceResponse;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Project with app count for list view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectWithAppCount {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub app_count: i64,
}

/// Project with its apps, databases, and services for detail view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectWithApps {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub apps: Vec<App>,
    #[serde(default)]
    pub databases: Vec<ManagedDatabaseResponse>,
    #[serde(default)]
    pub services: Vec<ServiceResponse>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Request to assign/unassign an app to a project
#[derive(Debug, Deserialize)]
pub struct AssignAppProjectRequest {
    pub project_id: Option<String>,
}
