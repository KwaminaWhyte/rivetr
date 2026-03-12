//! API handlers for PostgreSQL database extensions

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::{DatabaseStatus, DatabaseType, ManagedDatabase};
use crate::AppState;

/// Request to install a PostgreSQL extension
#[derive(Debug, Deserialize)]
pub struct InstallExtensionRequest {
    /// Extension name (e.g., "pgvector", "postgis")
    pub extension: String,
}

/// A single installed PostgreSQL extension
#[derive(Debug, Serialize)]
pub struct ExtensionEntry {
    pub name: String,
}

/// List of installed extensions response
#[derive(Debug, Serialize)]
pub struct ExtensionsResponse {
    pub extensions: Vec<ExtensionEntry>,
}

/// GET /api/databases/:id/extensions
/// Lists all installed extensions by running `SELECT extname FROM pg_extension;`
pub async fn list_extensions(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ExtensionsResponse>, (StatusCode, Json<serde_json::Value>)> {
    let database = get_running_postgres(&state, &id).await?;

    let container_name = database.container_name();
    let credentials = database.get_credentials().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Invalid credentials"})),
        )
    })?;

    let cmd = vec![
        "psql".to_string(),
        "-U".to_string(),
        credentials.username.clone(),
        "-t".to_string(),
        "-c".to_string(),
        "SELECT extname FROM pg_extension ORDER BY extname;".to_string(),
    ];

    let result = state
        .runtime
        .run_command(&container_name, cmd)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list extensions: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to list extensions: {}", e)})),
            )
        })?;

    if result.exit_code != 0 {
        tracing::error!("psql exited with code {}: {}", result.exit_code, result.stderr);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": result.stderr})),
        ));
    }

    let extensions: Vec<ExtensionEntry> = result
        .stdout
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(|l| ExtensionEntry { name: l.to_string() })
        .collect();

    Ok(Json(ExtensionsResponse { extensions }))
}

/// POST /api/databases/:id/extensions
/// Installs a PostgreSQL extension via `CREATE EXTENSION IF NOT EXISTS <name>;`
pub async fn install_extension(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<InstallExtensionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Validate the extension name: only alphanumeric, underscore, and hyphen
    if req.extension.is_empty()
        || !req
            .extension
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid extension name"})),
        ));
    }

    let database = get_running_postgres(&state, &id).await?;

    let container_name = database.container_name();
    let credentials = database.get_credentials().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Invalid credentials"})),
        )
    })?;

    let sql = format!("CREATE EXTENSION IF NOT EXISTS \"{}\";", req.extension);

    let cmd = vec![
        "psql".to_string(),
        "-U".to_string(),
        credentials.username.clone(),
        "-c".to_string(),
        sql,
    ];

    let result = state
        .runtime
        .run_command(&container_name, cmd)
        .await
        .map_err(|e| {
            tracing::error!("Failed to install extension {}: {}", req.extension, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to install extension: {}", e)})),
            )
        })?;

    if result.exit_code != 0 {
        tracing::error!(
            "Failed to install extension {}: {}",
            req.extension,
            result.stderr
        );
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({
                "error": format!("Extension install failed: {}", result.stderr.trim())
            })),
        ));
    }

    tracing::info!(
        "Installed PostgreSQL extension '{}' on database {}",
        req.extension,
        database.name
    );

    Ok(Json(serde_json::json!({
        "message": format!("Extension '{}' installed successfully", req.extension)
    })))
}

/// Helper: fetch the database, validate it's a running PostgreSQL instance.
async fn get_running_postgres(
    state: &Arc<AppState>,
    id: &str,
) -> Result<ManagedDatabase, (StatusCode, Json<serde_json::Value>)> {
    let database = sqlx::query_as::<_, ManagedDatabase>("SELECT * FROM databases WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("DB query failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Database query failed"})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Database not found"})),
            )
        })?;

    if database.get_db_type() != DatabaseType::Postgres {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Extensions are only supported for PostgreSQL databases"})),
        ));
    }

    if database.get_status() != DatabaseStatus::Running {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Database must be running to manage extensions"})),
        ));
    }

    Ok(database)
}
