use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AppReplica {
    pub id: String,
    pub app_id: String,
    pub replica_index: i64,
    pub container_id: Option<String>,
    pub status: String,
    pub started_at: Option<String>,
    pub stopped_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ReplicaStatus {
    pub replica_index: i64,
    pub container_id: Option<String>,
    pub status: String,
    pub started_at: Option<String>,
}
