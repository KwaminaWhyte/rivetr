//! Build server models and DTOs.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BuildServer {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub ssh_private_key: Option<String>,
    pub status: String,
    pub last_seen_at: Option<String>,
    pub docker_version: Option<String>,
    pub cpu_count: Option<i64>,
    pub memory_bytes: Option<i64>,
    pub concurrent_builds: i64,
    pub active_builds: i64,
    pub team_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateBuildServerRequest {
    pub name: String,
    pub host: String,
    pub port: Option<i64>,
    pub username: Option<String>,
    pub ssh_private_key: Option<String>,
    pub concurrent_builds: Option<i64>,
    pub team_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBuildServerRequest {
    pub name: Option<String>,
    pub host: Option<String>,
    pub port: Option<i64>,
    pub username: Option<String>,
    pub ssh_private_key: Option<String>,
    pub concurrent_builds: Option<i64>,
}
