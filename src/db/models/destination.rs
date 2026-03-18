use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Destination {
    pub id: String,
    pub name: String,
    pub network_name: String,
    pub server_id: Option<String>,
    pub team_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateDestinationRequest {
    pub name: String,
    pub network_name: String,
    pub server_id: Option<String>,
    pub team_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDestinationRequest {
    pub name: Option<String>,
}
