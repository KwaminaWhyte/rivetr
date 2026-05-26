//! API handlers for linking managed databases to apps so that DB connection
//! details (DATABASE_URL/REDIS_URL/etc, plus host/port/user/password/db) are
//! auto-injected as env vars into the app container at deploy time.
//!
//! Endpoints:
//! - POST   /api/apps/:app_id/links              — create a link
//! - GET    /api/apps/:app_id/links              — list links for an app
//! - DELETE /api/apps/:app_id/links/:link_id     — remove a link
//! - GET    /api/apps/:app_id/linked-env-vars    — preview injected env vars

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{App, ManagedDatabase};
use crate::AppState;

/// Persisted link record.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DatabaseAppLink {
    pub id: String,
    pub database_id: String,
    pub app_id: String,
    pub env_prefix: String,
    pub created_at: String,
}

/// Response DTO including the database name + type so the UI can render a
/// useful label without a second round trip.
#[derive(Debug, Serialize)]
pub struct DatabaseAppLinkResponse {
    pub id: String,
    pub database_id: String,
    pub app_id: String,
    pub env_prefix: String,
    pub created_at: String,
    pub database_name: String,
    pub database_type: String,
    pub database_status: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateLinkRequest {
    pub database_id: String,
    #[serde(default)]
    pub env_prefix: Option<String>,
}

/// Preview of one env var that would be injected.
#[derive(Debug, Serialize)]
pub struct LinkedEnvVarPreview {
    pub key: String,
    /// Whether this key would be overridden by an existing app env var.
    pub overridden: bool,
}

#[derive(Debug, Serialize)]
pub struct LinkedEnvVarsResponse {
    pub link_id: String,
    pub database_id: String,
    pub database_name: String,
    pub env_prefix: String,
    pub vars: Vec<LinkedEnvVarPreview>,
}

/// Validate a user-supplied env_prefix.  Allows empty string (no prefix),
/// otherwise must be `[A-Z][A-Z0-9_]*` and is normalized to upper-case ending
/// with `_` so the resulting var is `<PREFIX>_DATABASE_URL`.
fn normalize_prefix(raw: Option<String>) -> Result<String, String> {
    let prefix = raw.unwrap_or_default();
    if prefix.is_empty() {
        return Ok(String::new());
    }
    let upper = prefix.to_uppercase();
    if !upper.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("env_prefix may only contain letters, numbers, and underscores".to_string());
    }
    if !upper.chars().next().unwrap().is_ascii_alphabetic() {
        return Err("env_prefix must start with a letter".to_string());
    }
    let normalized = if upper.ends_with('_') {
        upper
    } else {
        format!("{}_", upper)
    };
    Ok(normalized)
}

/// Compute the env vars that the given database would inject when linked with
/// the given prefix.  Returns `(key, value)` pairs in deterministic order.
///
/// Used both at deploy time (to actually inject) and from the preview endpoint.
pub fn compute_injected_vars(database: &ManagedDatabase, prefix: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let creds = match database.get_credentials() {
        Some(c) => c,
        None => return out,
    };
    let db_type = database.get_db_type();
    let host = database.container_name();
    let port = database.internal_port;
    let user = creds.username.clone();
    let password = creds.password.clone();
    let db_name = creds.database.clone().unwrap_or_else(|| user.clone());

    // Connection-string key depends on type.
    let url_key = match db_type {
        crate::db::DatabaseType::Redis
        | crate::db::DatabaseType::Dragonfly
        | crate::db::DatabaseType::Keydb => "REDIS_URL",
        crate::db::DatabaseType::Mongodb => "MONGODB_URL",
        _ => "DATABASE_URL",
    };

    if let Some(url) = database.internal_connection_string() {
        out.push((format!("{}{}", prefix, url_key), url));
    }

    // Per-component vars — useful for clients that don't accept a URL.
    out.push((format!("{}HOST", prefix), host));
    out.push((format!("{}PORT", prefix), port.to_string()));
    out.push((format!("{}USER", prefix), user));
    out.push((format!("{}PASSWORD", prefix), password));
    out.push((format!("{}DB", prefix), db_name));

    out
}

/// Helper: load all links for an app and return `(database, prefix)` pairs.
/// User-defined env vars take precedence — caller is responsible for the merge.
pub async fn load_links_for_app(
    db: &crate::DbPool,
    app_id: &str,
) -> Vec<(ManagedDatabase, String)> {
    let links: Vec<DatabaseAppLink> = sqlx::query_as::<_, DatabaseAppLink>(
        "SELECT id, database_id, app_id, env_prefix, created_at \
         FROM database_app_links WHERE app_id = ?",
    )
    .bind(app_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    let mut out = Vec::with_capacity(links.len());
    for link in links {
        if let Ok(database) =
            sqlx::query_as::<_, ManagedDatabase>("SELECT * FROM databases WHERE id = ?")
                .bind(&link.database_id)
                .fetch_one(db)
                .await
        {
            out.push((database, link.env_prefix));
        }
    }
    out
}

async fn fetch_app(state: &Arc<AppState>, app_id: &str) -> Result<App, StatusCode> {
    sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(app_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)
}

async fn fetch_database(
    state: &Arc<AppState>,
    database_id: &str,
) -> Result<ManagedDatabase, StatusCode> {
    sqlx::query_as::<_, ManagedDatabase>("SELECT * FROM databases WHERE id = ?")
        .bind(database_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)
}

/// POST /api/apps/:app_id/links
pub async fn create_link(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(req): Json<CreateLinkRequest>,
) -> Result<(StatusCode, Json<DatabaseAppLinkResponse>), (StatusCode, Json<serde_json::Value>)> {
    let app = fetch_app(&state, &app_id)
        .await
        .map_err(|s| (s, Json(serde_json::json!({"error": "App not found"}))))?;
    let database = fetch_database(&state, &req.database_id)
        .await
        .map_err(|s| (s, Json(serde_json::json!({"error": "Database not found"}))))?;

    // Best-effort scoping: if both have a project_id, they must match.  This
    // keeps the picker honest without preventing power users from linking DBs
    // outside a project (project_id is optional on apps).
    if let (Some(app_proj), Some(db_proj)) = (app.project_id.as_ref(), database.project_id.as_ref())
    {
        if app_proj != db_proj {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "App and database belong to different projects"})),
            ));
        }
    }

    let prefix = normalize_prefix(req.env_prefix).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        )
    })?;

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO database_app_links (id, database_id, app_id, env_prefix, created_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&database.id)
    .bind(&app.id)
    .bind(&prefix)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("UNIQUE") {
            (
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": "Database is already linked to this app"})),
            )
        } else {
            tracing::error!("Failed to create database link: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to create link"})),
            )
        }
    })?;

    Ok((
        StatusCode::CREATED,
        Json(DatabaseAppLinkResponse {
            id,
            database_id: database.id.clone(),
            app_id: app.id.clone(),
            env_prefix: prefix,
            created_at: now,
            database_name: database.name.clone(),
            database_type: database.db_type.clone(),
            database_status: database.status.clone(),
        }),
    ))
}

/// GET /api/apps/:app_id/links
pub async fn list_links(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<DatabaseAppLinkResponse>>, StatusCode> {
    fetch_app(&state, &app_id).await?;

    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
        ),
    >(
        "SELECT l.id, l.database_id, l.app_id, l.env_prefix, l.created_at, \
                d.name, d.db_type, d.status \
         FROM database_app_links l \
         JOIN databases d ON d.id = l.database_id \
         WHERE l.app_id = ? \
         ORDER BY l.created_at ASC",
    )
    .bind(&app_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list database links: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let out: Vec<DatabaseAppLinkResponse> = rows
        .into_iter()
        .map(
            |(id, database_id, app_id, env_prefix, created_at, name, db_type, status)| {
                DatabaseAppLinkResponse {
                    id,
                    database_id,
                    app_id,
                    env_prefix,
                    created_at,
                    database_name: name,
                    database_type: db_type,
                    database_status: status,
                }
            },
        )
        .collect();

    Ok(Json(out))
}

/// DELETE /api/apps/:app_id/links/:link_id
pub async fn delete_link(
    State(state): State<Arc<AppState>>,
    Path((app_id, link_id)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM database_app_links WHERE id = ? AND app_id = ?")
        .bind(&link_id)
        .bind(&app_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete database link: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/apps/:app_id/linked-env-vars
///
/// Returns, for each link, the list of env var keys that would be injected at
/// deploy time, with `overridden` flagged for keys that already exist on the
/// app (and therefore won't actually be injected).
pub async fn preview_linked_env_vars(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<LinkedEnvVarsResponse>>, StatusCode> {
    fetch_app(&state, &app_id).await?;

    let existing_keys: Vec<String> =
        sqlx::query_scalar::<_, String>("SELECT key FROM env_vars WHERE app_id = ?")
            .bind(&app_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

    let links: Vec<DatabaseAppLink> = sqlx::query_as::<_, DatabaseAppLink>(
        "SELECT id, database_id, app_id, env_prefix, created_at \
         FROM database_app_links WHERE app_id = ? ORDER BY created_at ASC",
    )
    .bind(&app_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut out = Vec::with_capacity(links.len());
    for link in links {
        let database = match fetch_database(&state, &link.database_id).await {
            Ok(d) => d,
            Err(_) => continue,
        };
        let pairs = compute_injected_vars(&database, &link.env_prefix);
        let vars = pairs
            .into_iter()
            .map(|(key, _)| LinkedEnvVarPreview {
                overridden: existing_keys.contains(&key),
                key,
            })
            .collect();
        out.push(LinkedEnvVarsResponse {
            link_id: link.id,
            database_id: database.id,
            database_name: database.name,
            env_prefix: link.env_prefix,
            vars,
        });
    }

    Ok(Json(out))
}
