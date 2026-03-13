//! Cloudflare Tunnel models.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Status of a cloudflared tunnel container.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TunnelStatus {
    Stopped,
    Starting,
    Running,
    Error,
}

impl std::fmt::Display for TunnelStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stopped => write!(f, "stopped"),
            Self::Starting => write!(f, "starting"),
            Self::Running => write!(f, "running"),
            Self::Error => write!(f, "error"),
        }
    }
}

impl std::str::FromStr for TunnelStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "stopped" => Ok(Self::Stopped),
            "starting" => Ok(Self::Starting),
            "running" => Ok(Self::Running),
            "error" => Ok(Self::Error),
            _ => Err(format!("Unknown tunnel status: {}", s)),
        }
    }
}

impl From<String> for TunnelStatus {
    fn from(s: String) -> Self {
        s.parse().unwrap_or(Self::Stopped)
    }
}

/// A Cloudflare Tunnel record stored in the DB.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CloudflareTunnel {
    pub id: String,
    pub name: String,
    /// Encrypted/raw tunnel token — masked in API responses.
    pub tunnel_token: String,
    pub container_id: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// API-safe response that masks the tunnel token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudflareTunnelResponse {
    pub id: String,
    pub name: String,
    /// Always "***" — never expose the raw token over the API.
    pub tunnel_token: String,
    pub container_id: Option<String>,
    pub status: String,
    pub routes: Vec<CloudflareTunnelRoute>,
    pub created_at: String,
    pub updated_at: String,
}

impl CloudflareTunnel {
    pub fn to_response(&self, routes: Vec<CloudflareTunnelRoute>) -> CloudflareTunnelResponse {
        CloudflareTunnelResponse {
            id: self.id.clone(),
            name: self.name.clone(),
            tunnel_token: "***".to_string(),
            container_id: self.container_id.clone(),
            status: self.status.clone(),
            routes,
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
        }
    }
}

/// A route entry for a Cloudflare Tunnel.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CloudflareTunnelRoute {
    pub id: String,
    pub tunnel_id: String,
    pub hostname: String,
    pub service_url: String,
    pub app_id: Option<String>,
    pub created_at: String,
}

/// Request body for creating a new tunnel.
#[derive(Debug, Deserialize)]
pub struct CreateTunnelRequest {
    pub name: String,
    pub tunnel_token: String,
}

/// Request body for adding a route to a tunnel.
#[derive(Debug, Deserialize)]
pub struct CreateTunnelRouteRequest {
    pub hostname: String,
    pub service_url: String,
    pub app_id: Option<String>,
}
