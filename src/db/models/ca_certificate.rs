//! CA certificate model.
//!
//! Stores custom CA certificates (PEM format) that can be used for servers
//! that rely on private certificate authorities.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A stored CA certificate.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CaCertificate {
    pub id: String,
    pub name: String,
    /// PEM-encoded certificate content.
    pub certificate: String,
    pub team_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Request body for creating a CA certificate.
#[derive(Debug, Deserialize)]
pub struct CreateCaCertificateRequest {
    pub name: String,
    /// PEM-encoded certificate (must start with `-----BEGIN CERTIFICATE-----`).
    pub certificate: String,
    pub team_id: Option<String>,
}
