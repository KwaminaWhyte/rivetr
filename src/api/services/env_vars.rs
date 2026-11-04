//! Environment variables of a Docker Compose service (encrypted at rest).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::api::audit::{audit_log, ClientIp};
use crate::api::error::ApiError;
use crate::db::{
    actions, resource_types, CreateServiceEnvVarRequest, ImportServiceEnvRequest, ServiceEnvVar,
    ServiceEnvVarResponse, UpdateServiceEnvVarRequest, User,
};
use crate::AppState;

use super::compose::parse_env_file;

#[derive(Debug, Deserialize, Default)]
pub struct RevealQuery {
    #[serde(default)]
    pub reveal: bool,
}

fn key_for(state: &AppState) -> Option<[u8; 32]> {
    state
        .config
        .auth
        .encryption_key
        .as_ref()
        .map(|s| crate::crypto::derive_key(s))
}

pub(crate) fn is_valid_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    key.len() <= 256 && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn encrypt(state: &AppState, value: &str) -> Result<String, ApiError> {
    crate::crypto::encrypt_if_key_available(value, key_for(state).as_ref())
        .map_err(|e| ApiError::internal(format!("Failed to encrypt value: {}", e)))
}

fn respond(state: &AppState, v: &ServiceEnvVar, reveal: bool) -> ServiceEnvVarResponse {
    let plain = crate::crypto::decrypt_if_encrypted(&v.value, key_for(state).as_ref())
        .unwrap_or_else(|_| v.value.clone());
    v.to_response(&plain, reveal)
}

async fn audit(
    state: &Arc<AppState>,
    user: &User,
    ip: &ClientIp,
    service_id: &str,
    what: serde_json::Value,
) {
    audit_log(
        state,
        actions::SERVICE_UPDATE,
        resource_types::SERVICE,
        Some(service_id),
        None,
        Some(&user.id),
        ip.as_deref(),
        Some(what),
    )
    .await;
}

async fn ensure_service(state: &AppState, id: &str) -> Result<(), ApiError> {
    let exists: Option<String> = sqlx::query_scalar("SELECT id FROM services WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?;
    exists
        .map(|_| ())
        .ok_or_else(|| ApiError::not_found("Service not found"))
}

/// GET /api/services/:id/env-vars?reveal=true
pub async fn list_service_env_vars(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(q): Query<RevealQuery>,
) -> Result<Json<Vec<ServiceEnvVarResponse>>, ApiError> {
    ensure_service(&state, &id).await?;
    let rows: Vec<ServiceEnvVar> =
        sqlx::query_as("SELECT * FROM service_env_vars WHERE service_id = ? ORDER BY key")
            .bind(&id)
            .fetch_all(&state.db)
            .await
            .map_err(|e| ApiError::database(e.to_string()))?;
    Ok(Json(
        rows.iter().map(|v| respond(&state, v, q.reveal)).collect(),
    ))
}

/// POST /api/services/:id/env-vars
pub async fn create_service_env_var(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(id): Path<String>,
    Json(req): Json<CreateServiceEnvVarRequest>,
) -> Result<(StatusCode, Json<ServiceEnvVarResponse>), ApiError> {
    ensure_service(&state, &id).await?;
    let key = req.key.trim().to_string();
    if !is_valid_env_key(&key) {
        return Err(ApiError::bad_request(
            "Names start with a letter or underscore and use only letters, digits and underscores",
        ));
    }
    let var_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO service_env_vars (id, service_id, key, value, is_secret, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&var_id)
    .bind(&id)
    .bind(&key)
    .bind(encrypt(&state, &req.value)?)
    .bind(req.is_secret as i32)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            ApiError::conflict(format!("{} already exists", key))
        } else {
            ApiError::database(e.to_string())
        }
    })?;
    audit(
        &state,
        &user,
        &client_ip,
        &id,
        serde_json::json!({ "env_var_created": key }),
    )
    .await;
    let row: ServiceEnvVar = sqlx::query_as("SELECT * FROM service_env_vars WHERE id = ?")
        .bind(&var_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?;
    Ok((StatusCode::CREATED, Json(respond(&state, &row, true))))
}

/// PUT /api/services/:id/env-vars/:key
pub async fn update_service_env_var(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path((id, key)): Path<(String, String)>,
    Json(req): Json<UpdateServiceEnvVarRequest>,
) -> Result<Json<ServiceEnvVarResponse>, ApiError> {
    let row: ServiceEnvVar =
        sqlx::query_as("SELECT * FROM service_env_vars WHERE service_id = ? AND key = ?")
            .bind(&id)
            .bind(&key)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::database(e.to_string()))?
            .ok_or_else(|| ApiError::not_found("Variable not found"))?;
    let value = match &req.value {
        Some(v) => encrypt(&state, v)?,
        None => row.value.clone(),
    };
    let is_secret = req.is_secret.map(|b| b as i32).unwrap_or(row.is_secret);
    sqlx::query(
        "UPDATE service_env_vars SET value = ?, is_secret = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&value)
    .bind(is_secret)
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(&row.id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::database(e.to_string()))?;
    audit(
        &state,
        &user,
        &client_ip,
        &id,
        serde_json::json!({ "env_var_updated": key }),
    )
    .await;
    let row: ServiceEnvVar = sqlx::query_as("SELECT * FROM service_env_vars WHERE id = ?")
        .bind(&row.id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?;
    Ok(Json(respond(&state, &row, true)))
}

/// DELETE /api/services/:id/env-vars/:key
pub async fn delete_service_env_var(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path((id, key)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let res = sqlx::query("DELETE FROM service_env_vars WHERE service_id = ? AND key = ?")
        .bind(&id)
        .bind(&key)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?;
    if res.rows_affected() == 0 {
        return Err(ApiError::not_found("Variable not found"));
    }
    audit(
        &state,
        &user,
        &client_ip,
        &id,
        serde_json::json!({ "env_var_deleted": key }),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, serde::Serialize)]
pub struct ImportResult {
    pub created: usize,
    pub updated: usize,
    pub skipped: Vec<String>,
}

/// Import a pasted `.env` file.
///
/// POST /api/services/:id/env-import
pub async fn import_service_env_vars(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(id): Path<String>,
    Json(req): Json<ImportServiceEnvRequest>,
) -> Result<Json<ImportResult>, ApiError> {
    ensure_service(&state, &id).await?;
    if req.content.len() > 512 * 1024 {
        return Err(ApiError::bad_request("The file is larger than 512 KB"));
    }
    let mut result = ImportResult {
        created: 0,
        updated: 0,
        skipped: vec![],
    };
    let now = chrono::Utc::now().to_rfc3339();
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| ApiError::database(e.to_string()))?;
    for (key, value) in parse_env_file(&req.content) {
        if !is_valid_env_key(&key) {
            result.skipped.push(key);
            continue;
        }
        let existing: Option<String> =
            sqlx::query_scalar("SELECT id FROM service_env_vars WHERE service_id = ? AND key = ?")
                .bind(&id)
                .bind(&key)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| ApiError::database(e.to_string()))?;
        let stored = encrypt(&state, &value)?;
        match existing {
            Some(var_id) if req.overwrite => {
                sqlx::query(
                    "UPDATE service_env_vars SET value = ?, is_secret = ?, updated_at = ? WHERE id = ?",
                )
                .bind(&stored)
                .bind(req.is_secret as i32)
                .bind(&now)
                .bind(&var_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| ApiError::database(e.to_string()))?;
                result.updated += 1;
            }
            Some(_) => result.skipped.push(key),
            None => {
                sqlx::query(
                    "INSERT INTO service_env_vars (id, service_id, key, value, is_secret, created_at, updated_at) \
                     VALUES (?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(uuid::Uuid::new_v4().to_string())
                .bind(&id)
                .bind(&key)
                .bind(&stored)
                .bind(req.is_secret as i32)
                .bind(&now)
                .bind(&now)
                .execute(&mut *tx)
                .await
                .map_err(|e| ApiError::database(e.to_string()))?;
                result.created += 1;
            }
        }
    }
    tx.commit()
        .await
        .map_err(|e| ApiError::database(e.to_string()))?;
    audit(
        &state,
        &user,
        &client_ip,
        &id,
        serde_json::json!({ "env_vars_imported": result.created + result.updated }),
    )
    .await;
    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_keys() {
        assert!(is_valid_env_key("DATABASE_URL"));
        assert!(is_valid_env_key("_x1"));
        assert!(!is_valid_env_key("1X"));
        assert!(!is_valid_env_key("A-B"));
        assert!(!is_valid_env_key(""));
    }
}
