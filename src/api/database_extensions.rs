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

/// Returns the apt package name for extensions that aren't bundled in the
/// official postgres image and must be installed at the OS level first.
/// Built-in extensions (pg_trgm, uuid-ossp, hstore, etc.) return None.
fn apt_package_for_extension(extension: &str, pg_version: &str) -> Option<String> {
    match extension {
        "pgvector" => Some(format!("postgresql-{}-pgvector", pg_version)),
        "postgis" => Some(format!("postgresql-{}-postgis-3", pg_version)),
        "apache_age" => Some(format!("postgresql-{}-age", pg_version)),
        "timescaledb" => Some(format!("postgresql-{}-timescaledb", pg_version)),
        "pg_cron" => Some(format!("postgresql-{}-cron", pg_version)),
        "pg_partman" => Some(format!("postgresql-{}-partman", pg_version)),
        "pgrouting" => Some(format!("postgresql-{}-pgrouting", pg_version)),
        "rum" => Some(format!("postgresql-{}-rum", pg_version)),
        _ => None, // Built-in contrib extensions need no package
    }
}

/// Some packages register their extension under a different SQL name than
/// the display name shown in the UI. Maps display name → SQL extension name.
fn sql_extension_name(extension: &str) -> &str {
    match extension {
        "pgvector" => "vector",
        "apache_age" => "age",
        _ => extension,
    }
}

/// Reverse of sql_extension_name: maps SQL name → UI display name so that
/// the frontend's installed-check works against the curated extension list.
fn display_name_for_sql(sql_name: &str) -> &str {
    match sql_name {
        "vector" => "pgvector",
        "age" => "apache_age",
        _ => sql_name,
    }
}

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

    let db_name = credentials
        .database
        .clone()
        .unwrap_or_else(|| credentials.username.clone());

    let cmd = vec![
        "psql".to_string(),
        "-U".to_string(),
        credentials.username.clone(),
        "-d".to_string(),
        db_name,
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
        .map(|l| ExtensionEntry {
            name: display_name_for_sql(l).to_string(),
        })
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

    let db_name = credentials
        .database
        .clone()
        .unwrap_or_else(|| credentials.username.clone());

    // Some extensions are not bundled in the official postgres image and must be
    // installed at the OS level first via apt before CREATE EXTENSION will work.
    if let Some(pkg) = apt_package_for_extension(&req.extension, &database.version) {
        let apt_cmd = vec![
            "sh".to_string(),
            "-c".to_string(),
            format!("apt-get update -qq && apt-get install -y --no-install-recommends {}", pkg),
        ];
        let apt_result = state.runtime.run_command(&container_name, apt_cmd).await;
        match apt_result {
            Ok(r) if r.exit_code != 0 => {
                tracing::error!("apt-get install {} failed: {}", pkg, r.stderr);
                return Err((
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(serde_json::json!({
                        "error": format!("Failed to install OS package '{}': {}", pkg, r.stderr.trim())
                    })),
                ));
            }
            Err(e) => {
                tracing::error!("Failed to run apt-get for {}: {}", pkg, e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": format!("Failed to install OS package: {}", e)})),
                ));
            }
            Ok(_) => {
                tracing::info!("Installed OS package '{}' for extension '{}'", pkg, req.extension);
            }
        }
    }

    // TimescaleDB requires shared_preload_libraries in postgresql.conf and a restart
    // before CREATE EXTENSION will succeed.
    if req.extension == "timescaledb" {
        let patch_cmd = vec![
            "sh".to_string(),
            "-c".to_string(),
            // Only add if not already present
            "grep -q 'shared_preload_libraries' /var/lib/postgresql/data/postgresql.conf \
             && sed -i \"s/.*shared_preload_libraries.*/shared_preload_libraries = 'timescaledb'/\" /var/lib/postgresql/data/postgresql.conf \
             || echo \"shared_preload_libraries = 'timescaledb'\" >> /var/lib/postgresql/data/postgresql.conf".to_string(),
        ];
        if let Ok(r) = state.runtime.run_command(&container_name, patch_cmd).await {
            if r.exit_code != 0 {
                tracing::warn!("Failed to patch postgresql.conf for timescaledb: {}", r.stderr);
            }
        }

        // Restart the container so PostgreSQL picks up the new config
        let container_id = database.container_id.as_deref().unwrap_or(&container_name);
        let _ = state.runtime.stop(container_id).await;
        let _ = state.runtime.start(container_id).await;

        // Wait for PostgreSQL to be ready (poll pg_isready up to 30s)
        let username = credentials.username.clone();
        let ready = {
            let mut ok = false;
            for _ in 0..30 {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                let check = state.runtime.run_command(
                    &container_name,
                    vec!["pg_isready".to_string(), "-U".to_string(), username.clone()],
                ).await;
                if check.map(|r| r.exit_code == 0).unwrap_or(false) {
                    ok = true;
                    break;
                }
            }
            ok
        };

        if !ready {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Database did not become ready after restart"})),
            ));
        }
    }

    // TimescaleDB requires CASCADE to install its internal dependencies.
    let sql = if req.extension == "timescaledb" {
        format!("CREATE EXTENSION IF NOT EXISTS \"{}\" CASCADE;", sql_extension_name(&req.extension))
    } else {
        format!("CREATE EXTENSION IF NOT EXISTS \"{}\";", sql_extension_name(&req.extension))
    };

    let cmd = vec![
        "psql".to_string(),
        "-U".to_string(),
        credentials.username.clone(),
        "-d".to_string(),
        db_name,
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
