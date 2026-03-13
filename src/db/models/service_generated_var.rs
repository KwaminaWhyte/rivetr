//! Service generated variable model.
//!
//! Stores auto-generated magic variables (SERVICE_PASSWORD_*, SERVICE_BASE64_*, etc.)
//! for Docker Compose services so their values are stable across restarts.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A generated variable for a compose service.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ServiceGeneratedVar {
    pub id: String,
    pub service_id: String,
    pub key: String,
    pub value: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceGeneratedVarResponse {
    pub id: String,
    pub service_id: String,
    pub key: String,
    pub value: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ServiceGeneratedVar> for ServiceGeneratedVarResponse {
    fn from(v: ServiceGeneratedVar) -> Self {
        ServiceGeneratedVarResponse {
            id: v.id,
            service_id: v.service_id,
            key: v.key,
            value: v.value,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}
