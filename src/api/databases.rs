//! API handlers for managed databases

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{
    actions, resource_types, CreateManagedDatabaseRequest, DatabaseCredentials, DatabaseStatus,
    DatabaseType, ManagedDatabase, ManagedDatabaseResponse, User,
};
use crate::engine::database_config::{generate_env_vars, generate_password, generate_username, get_config};
use crate::runtime::{ContainerStats, PortMapping, RunConfig};
use crate::AppState;

use super::audit::{audit_log, extract_client_ip};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub reveal: bool,
}

/// List all managed databases
pub async fn list_databases(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<ManagedDatabaseResponse>>, StatusCode> {
    let databases = sqlx::query_as::<_, ManagedDatabase>(
        "SELECT * FROM databases ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list databases: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let host = Some(state.config.server.host.as_str());
    let responses: Vec<ManagedDatabaseResponse> = databases
        .into_iter()
        .map(|db| db.to_response(query.reveal, host))
        .collect();

    Ok(Json(responses))
}

/// Get a single database by ID
pub async fn get_database(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<ListQuery>,
) -> Result<Json<ManagedDatabaseResponse>, StatusCode> {
    let database = sqlx::query_as::<_, ManagedDatabase>("SELECT * FROM databases WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get database: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let host = Some(state.config.server.host.as_str());
    Ok(Json(database.to_response(query.reveal, host)))
}

/// Create a new managed database
pub async fn create_database(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Json(req): Json<CreateManagedDatabaseRequest>,
) -> Result<(StatusCode, Json<ManagedDatabaseResponse>), StatusCode> {
    // Validate name
    if req.name.is_empty() {
        tracing::warn!("Database name is empty");
        return Err(StatusCode::BAD_REQUEST);
    }

    // Validate name format (alphanumeric and hyphens only)
    if !req
        .name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        tracing::warn!("Database name contains invalid characters: {}", req.name);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Get database type configuration
    let config = get_config(&req.db_type);

    // Generate credentials
    let username = req.username.unwrap_or_else(generate_username);
    let password = req.password.unwrap_or_else(|| generate_password(24));
    let database_name = req.database.or_else(|| Some(username.clone()));

    let credentials = DatabaseCredentials {
        username: username.clone(),
        password: password.clone(),
        database: database_name.clone(),
        root_password: if req.db_type == DatabaseType::Mysql {
            // Use custom root_password if provided, otherwise generate one
            Some(req.root_password.unwrap_or_else(|| generate_password(32)))
        } else {
            None
        },
    };

    let credentials_json =
        serde_json::to_string(&credentials).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Determine version
    let version = if req.version == "latest" {
        config.default_version.to_string()
    } else {
        req.version.clone()
    };

    // Create database record
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let volume_name = format!("rivetr-db-{}-data", req.name);

    // Build absolute volume path for Docker compatibility
    let volume_path = std::fs::canonicalize(&state.config.server.data_dir)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default().join(&state.config.server.data_dir))
        .join("databases")
        .join(&req.name)
        .to_string_lossy()
        .to_string()
        // Remove Windows UNC prefix (\\?\) and convert backslashes to forward slashes for Docker
        .trim_start_matches(r"\\?\")
        .replace('\\', "/");

    sqlx::query(
        r#"
        INSERT INTO databases (
            id, name, db_type, version, status, internal_port, external_port,
            public_access, credentials, volume_name, volume_path, memory_limit,
            cpu_limit, project_id, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.name)
    .bind(req.db_type.to_string())
    .bind(&version)
    .bind(DatabaseStatus::Pending.to_string())
    .bind(config.port as i32)
    .bind(0i32) // Will be assigned when started
    .bind(if req.public_access { 1 } else { 0 })
    .bind(&credentials_json)
    .bind(&volume_name)
    .bind(&volume_path)
    .bind(req.memory_limit.as_deref().unwrap_or("512mb"))
    .bind(req.cpu_limit.as_deref().unwrap_or("0.5"))
    .bind(&req.project_id)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create database: {}", e);
        if e.to_string().contains("UNIQUE") {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    // Start the database container asynchronously
    let state_clone = state.clone();
    let id_clone = id.clone();
    tokio::spawn(async move {
        if let Err(e) = start_database_container(&state_clone, &id_clone).await {
            tracing::error!("Failed to start database container: {}", e);
            // Update status to failed
            let _ = sqlx::query(
                "UPDATE databases SET status = ?, error_message = ?, updated_at = datetime('now') WHERE id = ?",
            )
            .bind(DatabaseStatus::Failed.to_string())
            .bind(e.to_string())
            .bind(&id_clone)
            .execute(&state_clone.db)
            .await;
        }
    });

    // Return the database record
    let database = sqlx::query_as::<_, ManagedDatabase>("SELECT * FROM databases WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::DATABASE_CREATE,
        resource_types::DATABASE,
        Some(&database.id),
        Some(&database.name),
        Some(&user.id),
        ip.as_deref(),
        Some(serde_json::json!({
            "db_type": req.db_type.to_string(),
            "version": version,
        })),
    )
    .await;

    let host = Some(state.config.server.host.as_str());
    Ok((StatusCode::CREATED, Json(database.to_response(true, host))))
}

/// Delete a managed database
pub async fn delete_database(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    // Get the database record
    let database = sqlx::query_as::<_, ManagedDatabase>("SELECT * FROM databases WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get database: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Stop and remove the container if it exists
    if let Some(ref container_id) = database.container_id {
        if let Err(e) = state.runtime.stop(container_id).await {
            tracing::warn!("Failed to stop database container: {}", e);
        }
        if let Err(e) = state.runtime.remove(container_id).await {
            tracing::warn!("Failed to remove database container: {}", e);
        }
    }

    // Delete the database record
    sqlx::query("DELETE FROM databases WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete database: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::DATABASE_DELETE,
        resource_types::DATABASE,
        Some(&database.id),
        Some(&database.name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    tracing::info!("Deleted managed database: {}", database.name);
    Ok(StatusCode::NO_CONTENT)
}

/// Start a database container
pub async fn start_database(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ManagedDatabaseResponse>, StatusCode> {
    // Check if database exists
    let database = sqlx::query_as::<_, ManagedDatabase>("SELECT * FROM databases WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check if already running
    if database.get_status() == DatabaseStatus::Running {
        let host = Some(state.config.server.host.as_str());
        return Ok(Json(database.to_response(false, host)));
    }

    start_database_container(&state, &id).await.map_err(|e| {
        tracing::error!("Failed to start database: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let database = sqlx::query_as::<_, ManagedDatabase>("SELECT * FROM databases WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::DATABASE_START,
        resource_types::DATABASE,
        Some(&database.id),
        Some(&database.name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    let host = Some(state.config.server.host.as_str());
    Ok(Json(database.to_response(false, host)))
}

/// Stop a database container
pub async fn stop_database(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ManagedDatabaseResponse>, StatusCode> {
    let database = sqlx::query_as::<_, ManagedDatabase>("SELECT * FROM databases WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(ref container_id) = database.container_id {
        state.runtime.stop(container_id).await.map_err(|e| {
            tracing::error!("Failed to stop container: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    sqlx::query("UPDATE databases SET status = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(DatabaseStatus::Stopped.to_string())
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let database = sqlx::query_as::<_, ManagedDatabase>("SELECT * FROM databases WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::DATABASE_STOP,
        resource_types::DATABASE,
        Some(&database.id),
        Some(&database.name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    let host = Some(state.config.server.host.as_str());
    Ok(Json(database.to_response(false, host)))
}

/// Query parameters for logs
#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    /// Number of lines to return (default: 100)
    #[serde(default = "default_lines")]
    pub lines: u32,
}

fn default_lines() -> u32 {
    100
}

/// Log entry for database logs
#[derive(Debug, serde::Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub message: String,
    pub stream: String,
}

/// Get database container logs
pub async fn get_database_logs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<LogsQuery>,
) -> Result<Json<Vec<LogEntry>>, (StatusCode, Json<serde_json::Value>)> {
    let database = sqlx::query_as::<_, ManagedDatabase>("SELECT * FROM databases WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get database: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to get database"})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Database not found"})),
            )
        })?;

    // Check if the database is running
    if database.get_status() != DatabaseStatus::Running {
        tracing::info!(
            "Database {} is not running (status: {})",
            database.name,
            database.status
        );
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Container is stopped",
                "message": "The database container is not running. Start the database to view logs.",
                "status": database.status
            })),
        ));
    }

    let container_id = database.container_id.ok_or_else(|| {
        tracing::warn!("Database {} has no container", database.name);
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Container is stopped",
                "message": "The database container is not running. Start the database to view logs."
            })),
        )
    })?;

    // Verify the container exists and is running
    match state.runtime.inspect(&container_id).await {
        Ok(info) => {
            if !info.running {
                tracing::info!("Database {} container exists but is not running", database.name);
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({
                        "error": "Container is stopped",
                        "message": "The database container is not running. Start the database to view logs."
                    })),
                ));
            }
        }
        Err(e) => {
            tracing::warn!("Failed to inspect container for database {}: {}", database.name, e);
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Container is stopped",
                    "message": "The database container is not running. Start the database to view logs."
                })),
            ));
        }
    }

    // Get logs from the container
    use futures::StreamExt;
    let log_stream = state.runtime.logs(&container_id).await.map_err(|e| {
        tracing::error!("Failed to get container logs: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to get container logs: {}", e)})),
        )
    })?;

    // Collect logs (limited to query.lines)
    let logs: Vec<LogEntry> = log_stream
        .take(query.lines as usize)
        .map(|log| LogEntry {
            timestamp: log.timestamp.clone(),
            message: log.message.clone(),
            stream: match log.stream {
                crate::runtime::LogStream::Stdout => "stdout".to_string(),
                crate::runtime::LogStream::Stderr => "stderr".to_string(),
            },
        })
        .collect()
        .await;

    Ok(Json(logs))
}

/// Internal function to start a database container
async fn start_database_container(state: &Arc<AppState>, id: &str) -> anyhow::Result<()> {
    let database = sqlx::query_as::<_, ManagedDatabase>("SELECT * FROM databases WHERE id = ?")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    let db_type = database.get_db_type();
    let config = get_config(&db_type);
    let credentials = database
        .get_credentials()
        .ok_or_else(|| anyhow::anyhow!("Invalid credentials"))?;

    // Update status to pulling
    sqlx::query("UPDATE databases SET status = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(DatabaseStatus::Pulling.to_string())
        .bind(id)
        .execute(&state.db)
        .await?;

    // Pull the image
    let image = format!("{}:{}", config.image, database.version);
    tracing::info!("Pulling database image: {}", image);
    state.runtime.pull_image(&image, None).await?;

    // Update status to starting
    sqlx::query("UPDATE databases SET status = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(DatabaseStatus::Starting.to_string())
        .bind(id)
        .execute(&state.db)
        .await?;

    // Create volume directory
    if let Some(ref volume_path) = database.volume_path {
        std::fs::create_dir_all(volume_path)?;
    }

    // Generate environment variables
    let env_vars = generate_env_vars(&db_type, &credentials);

    // Build run config
    let container_name = database.container_name();
    let mut port_mappings = Vec::new();

    if database.is_public() {
        port_mappings.push(PortMapping::new(0, config.port)); // 0 = auto-assign host port
    }

    // Volume bind mount
    let binds = if let Some(ref volume_path) = database.volume_path {
        vec![format!("{}:{}", volume_path, config.data_path)]
    } else {
        vec![]
    };

    let run_config = RunConfig {
        image: image.clone(),
        name: container_name.clone(),
        port: config.port,
        env: env_vars,
        memory_limit: database.memory_limit.clone(),
        cpu_limit: database.cpu_limit.clone(),
        port_mappings,
        network_aliases: vec![container_name.clone()],
        extra_hosts: vec![],
        labels: HashMap::new(),
        binds,
    };

    // Start the container
    tracing::info!("Starting database container: {}", container_name);
    let container_id = state.runtime.run(&run_config).await?;

    // Get the assigned host port if public access is enabled
    let external_port = if database.is_public() {
        let info = state.runtime.inspect(&container_id).await?;
        info.host_port.unwrap_or(0) as i32
    } else {
        0
    };

    // Update database record
    sqlx::query(
        "UPDATE databases SET container_id = ?, external_port = ?, status = ?, error_message = NULL, updated_at = datetime('now') WHERE id = ?",
    )
    .bind(&container_id)
    .bind(external_port)
    .bind(DatabaseStatus::Running.to_string())
    .bind(id)
    .execute(&state.db)
    .await?;

    tracing::info!(
        "Database {} started successfully (container: {})",
        database.name,
        container_id
    );

    Ok(())
}

/// Get container stats for a database
pub async fn get_database_stats(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ContainerStats>, StatusCode> {
    // Get the database
    let database = sqlx::query_as::<_, ManagedDatabase>("SELECT * FROM databases WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get database: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check if database is running
    if database.status != "running" {
        tracing::warn!("Database {} is not running (status: {})", id, database.status);
        return Err(StatusCode::NOT_FOUND);
    }

    // Get container ID
    let container_id = database.container_id.ok_or_else(|| {
        tracing::warn!("Database {} has no container ID", id);
        StatusCode::NOT_FOUND
    })?;

    // Get stats from the container runtime
    let stats = state
        .runtime
        .stats(&container_id)
        .await
        .map_err(|e| {
            tracing::warn!("Failed to get container stats for {}: {}", container_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(stats))
}
