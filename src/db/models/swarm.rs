use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SwarmNode {
    pub id: String,
    pub node_id: String,
    pub hostname: String,
    pub role: String,
    pub status: String,
    pub availability: String,
    pub cpu_count: Option<i64>,
    pub memory_bytes: Option<i64>,
    pub docker_version: Option<String>,
    pub ip_address: Option<String>,
    pub last_seen_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SwarmService {
    pub id: String,
    pub app_id: Option<String>,
    pub service_name: String,
    pub service_id: Option<String>,
    pub replicas: i64,
    pub mode: String,
    pub image: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}
