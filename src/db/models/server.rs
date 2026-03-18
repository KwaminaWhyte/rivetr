//! Server models and DTOs for multi-server support.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub ssh_private_key: Option<String>, // encrypted with AES-256-GCM
    pub ssh_password: Option<String>,    // encrypted with AES-256-GCM
    pub status: String,
    pub last_seen_at: Option<String>,
    pub cpu_usage: Option<f64>,
    pub memory_usage: Option<f64>,
    pub disk_usage: Option<f64>,
    pub memory_total: Option<i64>,
    pub disk_total: Option<i64>,
    pub os_info: Option<String>,
    pub docker_version: Option<String>,
    pub team_id: Option<String>,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    /// Hourly cost in USD (used for cost analysis). Defaults to $0.036/hr (DO s-2vcpu-4gb).
    #[serde(default = "default_hourly_rate")]
    pub hourly_rate: f64,
    pub created_at: String,
    pub updated_at: String,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

fn default_hourly_rate() -> f64 {
    0.036
}

#[derive(Debug, Deserialize)]
pub struct CreateServerRequest {
    pub name: String,
    pub host: String,
    pub port: Option<i64>,
    pub username: Option<String>,
    pub ssh_private_key: Option<String>,
    pub ssh_password: Option<String>,
    pub team_id: Option<String>,
    /// Hourly cost in USD for cost analysis (e.g. 0.036 for DO s-2vcpu-4gb)
    pub hourly_rate: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateServerRequest {
    pub name: Option<String>,
    pub host: Option<String>,
    pub port: Option<i64>,
    pub username: Option<String>,
    pub ssh_private_key: Option<String>,
    pub ssh_password: Option<String>,
    pub timezone: Option<String>,
    /// Hourly cost in USD for cost analysis
    pub hourly_rate: Option<f64>,
}
