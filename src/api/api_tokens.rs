//! API token management endpoints.
//!
//! Allows users to create named API tokens for programmatic access.
//! Tokens are stored as SHA-256 hashes; the plaintext is only shown once on creation.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::audit::{audit_log, extract_client_ip};
use super::error::ApiError;
use crate::{
    db::{actions, resource_types, User},
    AppState,
};

// ── Models ──────────────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct ApiToken {
    pub id: String,
    pub name: String,
    pub token_hash: String,
    pub user_id: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiTokenResponse {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    /// Only populated on creation, never returned again.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

impl From<ApiToken> for ApiTokenResponse {
    fn from(t: ApiToken) -> Self {
        Self {
            id: t.id,
            name: t.name,
            created_at: t.created_at,
            last_used_at: t.last_used_at,
            expires_at: t.expires_at,
            token: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateApiTokenRequest {
    pub name: String,
    /// Optional ISO-8601 expiry date.
    pub expires_at: Option<String>,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    hex::encode(digest)
}

fn generate_token() -> String {
    // 32 random bytes → 64 hex chars, prefixed with "rvt_" for easy identification
    let bytes: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
    format!("rvt_{}", hex::encode(bytes))
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// List all API tokens for the current user.
///
/// GET /api/tokens
pub async fn list_tokens(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<Vec<ApiTokenResponse>>, ApiError> {
    // System user (admin API token) has no DB record — return empty list
    if user.id == "system" {
        return Ok(Json(vec![]));
    }

    let tokens: Vec<ApiToken> =
        sqlx::query_as("SELECT * FROM api_tokens WHERE user_id = ? ORDER BY created_at DESC")
            .bind(&user.id)
            .fetch_all(&state.db)
            .await?;

    Ok(Json(tokens.into_iter().map(Into::into).collect()))
}

/// Create a new API token.
///
/// POST /api/tokens
pub async fn create_token(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Json(req): Json<CreateApiTokenRequest>,
) -> Result<Json<ApiTokenResponse>, ApiError> {
    if user.id == "system" {
        return Err(ApiError::forbidden(
            "Cannot create tokens for the admin API token user",
        ));
    }

    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::validation_field(
            "name",
            "Token name cannot be empty",
        ));
    }

    let plaintext = generate_token();
    let token_hash = hash_token(&plaintext);
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO api_tokens (id, name, token_hash, user_id, created_at, expires_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&token_hash)
    .bind(&user.id)
    .bind(&now)
    .bind(&req.expires_at)
    .execute(&state.db)
    .await?;

    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::TOKEN_CREATE,
        resource_types::TOKEN,
        Some(&id),
        Some(&name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    Ok(Json(ApiTokenResponse {
        id,
        name,
        created_at: now,
        last_used_at: None,
        expires_at: req.expires_at,
        token: Some(plaintext),
    }))
}

/// Delete an API token.
///
/// DELETE /api/tokens/:id
pub async fn delete_token(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    if user.id == "system" {
        return Err(ApiError::forbidden(
            "Cannot delete tokens for the admin API token user",
        ));
    }

    // Capture name before delete for the audit log entry.
    let name: Option<String> =
        sqlx::query_scalar("SELECT name FROM api_tokens WHERE id = ? AND user_id = ?")
            .bind(&id)
            .bind(&user.id)
            .fetch_optional(&state.db)
            .await?;

    let result = sqlx::query("DELETE FROM api_tokens WHERE id = ? AND user_id = ?")
        .bind(&id)
        .bind(&user.id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Token not found"));
    }

    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::TOKEN_DELETE,
        resource_types::TOKEN,
        Some(&id),
        name.as_deref(),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    Ok(Json(serde_json::json!({ "message": "Token deleted" })))
}
