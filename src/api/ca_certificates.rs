//! CA certificate management API endpoints.
//!
//! Allows storing custom CA certificates (PEM format) for servers that use
//! private certificate authorities.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use super::error::ApiError;
use crate::db::{CaCertificate, CreateCaCertificateRequest};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct ListCaCertificatesQuery {
    pub team_id: Option<String>,
}

/// List all CA certificates, optionally filtered by team_id.
///
/// GET /api/ca-certificates
pub async fn list_ca_certificates(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListCaCertificatesQuery>,
) -> Result<Json<Vec<CaCertificate>>, StatusCode> {
    let certs = if let Some(team_id) = &query.team_id {
        sqlx::query_as::<_, CaCertificate>(
            "SELECT * FROM ca_certificates WHERE team_id = ? ORDER BY created_at DESC",
        )
        .bind(team_id)
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as::<_, CaCertificate>(
            "SELECT * FROM ca_certificates ORDER BY created_at DESC",
        )
        .fetch_all(&state.db)
        .await
    }
    .map_err(|e| {
        tracing::error!("Failed to list CA certificates: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(certs))
}

/// Create a new CA certificate.
///
/// POST /api/ca-certificates
pub async fn create_ca_certificate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateCaCertificateRequest>,
) -> Result<(StatusCode, Json<CaCertificate>), ApiError> {
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::validation_field("name", "Certificate name cannot be empty"));
    }

    let certificate = req.certificate.trim().to_string();
    if !certificate.starts_with("-----BEGIN CERTIFICATE-----") {
        return Err(ApiError::validation_field(
            "certificate",
            "Certificate must be in PEM format and start with -----BEGIN CERTIFICATE-----",
        ));
    }

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO ca_certificates (id, name, certificate, team_id, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&name)
    .bind(&certificate)
    .bind(&req.team_id)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create CA certificate: {}", e);
        ApiError::internal("Failed to create CA certificate")
    })?;

    let cert = sqlx::query_as::<_, CaCertificate>("SELECT * FROM ca_certificates WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| ApiError::internal("Failed to fetch created certificate"))?;

    Ok((StatusCode::CREATED, Json(cert)))
}

/// Delete a CA certificate.
///
/// DELETE /api/ca-certificates/:id
pub async fn delete_ca_certificate(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query("DELETE FROM ca_certificates WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete CA certificate {}: {}", id, e);
            ApiError::internal("Failed to delete CA certificate")
        })?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("CA certificate not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}
