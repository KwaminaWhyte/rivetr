//! Docker Compose service models.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Service status for Docker Compose services
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceStatus {
    Pending,
    Running,
    Stopped,
    Failed,
}

impl std::fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Stopped => write!(f, "stopped"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for ServiceStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "stopped" => Ok(Self::Stopped),
            "failed" => Ok(Self::Failed),
            _ => Err(format!("Unknown service status: {}", s)),
        }
    }
}

impl From<String> for ServiceStatus {
    fn from(s: String) -> Self {
        s.parse().unwrap_or(Self::Pending)
    }
}

/// Docker Compose service entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Service {
    pub id: String,
    pub name: String,
    pub project_id: Option<String>,
    pub compose_content: String,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Service {
    /// Get the status as enum
    pub fn get_status(&self) -> ServiceStatus {
        ServiceStatus::from(self.status.clone())
    }

    /// Get the compose project name (used for docker compose commands)
    pub fn compose_project_name(&self) -> String {
        format!("rivetr-svc-{}", self.name)
    }

    /// Convert to response DTO
    pub fn to_response(self) -> ServiceResponse {
        ServiceResponse::from(self)
    }
}

/// Response DTO for Service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceResponse {
    pub id: String,
    pub name: String,
    pub project_id: Option<String>,
    pub compose_content: String,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Service> for ServiceResponse {
    fn from(service: Service) -> Self {
        ServiceResponse {
            id: service.id,
            name: service.name,
            project_id: service.project_id,
            compose_content: service.compose_content,
            status: service.status,
            error_message: service.error_message,
            created_at: service.created_at,
            updated_at: service.updated_at,
        }
    }
}

/// Request to create a Docker Compose service
#[derive(Debug, Deserialize)]
pub struct CreateServiceRequest {
    pub name: String,
    /// The docker-compose.yml content as a string
    pub compose_content: String,
    /// Associated project ID (optional)
    pub project_id: Option<String>,
}

/// Request to update a Docker Compose service
#[derive(Debug, Deserialize)]
pub struct UpdateServiceRequest {
    /// Update the compose content
    pub compose_content: Option<String>,
    /// Update the project association
    pub project_id: Option<String>,
}
