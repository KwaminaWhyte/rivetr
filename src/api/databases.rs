//! API handlers for managed databases

use axum::{
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{
    actions, resource_types, CreateManagedDatabaseRequest, DatabaseCredentials, DatabaseStatus,
    DatabaseType, ManagedDatabase, ManagedDatabaseResponse, TeamAuditAction, TeamAuditResourceType,
    User,
};
use crate::engine::database_config::{
    generate_env_vars, generate_password, generate_username, get_config,
};
use crate::runtime::{ContainerStats, PortMapping, RunConfig};
use crate::AppState;

use super::audit::{audit_log, extract_client_ip};
use super::teams::log_team_audit;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub reveal: bool,
    /// Filter by team ID
    pub team_id: Option<String>,
}

/// List all managed databases
pub async fn list_databases(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<ManagedDatabaseResponse>>, StatusCode> {
    let databases = if let Some(team_id) = &query.team_id {
        // Filter by team_id
        // Also include databases with NULL team_id if team_id is empty (for backward compatibility)
        if team_id.is_empty() {
            sqlx::query_as::<_, ManagedDatabase>(
                "SELECT * FROM databases WHERE team_id IS NULL ORDER BY created_at DESC",
            )
            .fetch_all(&state.db)
            .await
        } else {
            sqlx::query_as::<_, ManagedDatabase>(
                "SELECT * FROM databases WHERE team_id = ? OR team_id IS NULL ORDER BY created_at DESC",
            )
            .bind(team_id)
            .fetch_all(&state.db)
            .await
        }
    } else {
        // No filter, return all databases
        sqlx::query_as::<_, ManagedDatabase>("SELECT * FROM databases ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await
    }
    .map_err(|e| {
        tracing::error!("Failed to list databases: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let hostname = state.config.public_hostname();
    let host = Some(hostname.as_str());
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

    let hostname = state.config.public_hostname();
    let host = Some(hostname.as_str());
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
        root_password: if req.db_type == DatabaseType::Mysql || req.db_type == DatabaseType::Mariadb
        {
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
    // ID-based slug ensures globally unique container hostname across teams.
    let container_slug = ManagedDatabase::build_slug(&id);

    // Build absolute volume path for Docker compatibility.
    // Use "{name}-{id[:8]}" as the directory name so it is both human-readable
    // and globally unique — two databases with the same name (e.g. created at
    // different times) will never share a data directory.
    let volume_dir = format!("{}-{}", req.name, &id[..8.min(id.len())]);
    let volume_path = std::fs::canonicalize(&state.config.server.data_dir)
        .unwrap_or_else(|_| {
            std::env::current_dir()
                .unwrap_or_default()
                .join(&state.config.server.data_dir)
        })
        .join("databases")
        .join(&volume_dir)
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
            cpu_limit, project_id, team_id, created_at, updated_at, container_slug,
            custom_image, init_commands
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
    .bind(&req.team_id)
    .bind(&now)
    .bind(&now)
    .bind(&container_slug)
    .bind(&req.custom_image)
    .bind(&req.init_commands)
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

    // Log team audit event if database belongs to a team
    if let Some(ref team_id) = database.team_id {
        if let Err(e) = log_team_audit(
            &state.db,
            team_id,
            Some(&user.id),
            TeamAuditAction::DatabaseCreated,
            TeamAuditResourceType::Database,
            Some(&database.id),
            Some(serde_json::json!({
                "database_name": database.name,
                "db_type": req.db_type.to_string(),
                "version": version,
            })),
        )
        .await
        {
            tracing::warn!("Failed to log team audit event: {}", e);
        }
    }

    let hostname = state.config.public_hostname();
    let host = Some(hostname.as_str());
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

    // Log team audit event if database belonged to a team
    if let Some(ref team_id) = database.team_id {
        if let Err(e) = log_team_audit(
            &state.db,
            team_id,
            Some(&user.id),
            TeamAuditAction::DatabaseDeleted,
            TeamAuditResourceType::Database,
            Some(&database.id),
            Some(serde_json::json!({
                "database_name": database.name,
            })),
        )
        .await
        {
            tracing::warn!("Failed to log team audit event: {}", e);
        }
    }

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
        let hostname = state.config.public_hostname();
        let host = Some(hostname.as_str());
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

    let hostname = state.config.public_hostname();
    let host = Some(hostname.as_str());
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

    let hostname = state.config.public_hostname();
    let host = Some(hostname.as_str());
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
                tracing::info!(
                    "Database {} container exists but is not running",
                    database.name
                );
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
            tracing::warn!(
                "Failed to inspect container for database {}: {}",
                database.name,
                e
            );
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

/// Internal function to start a database container.
///
/// Emits live "start log" events through `state.start_log_streams` so the
/// dashboard side panel can show image-pull / container-create / running
/// progress without persisting a synthetic deployment row.
async fn start_database_container(state: &Arc<AppState>, id: &str) -> anyhow::Result<()> {
    let resource_key = format!("database:{}", id);
    state.start_log_streams.clear(&resource_key);
    state
        .start_log_streams
        .info(&resource_key, "info", "Starting database…");

    let result = start_database_container_inner(state, id, &resource_key).await;
    match &result {
        Ok(_) => {
            state
                .start_log_streams
                .end(&resource_key, "running", "Database is running");
        }
        Err(e) => {
            state
                .start_log_streams
                .error(&resource_key, "failed", format!("Failed: {}", e));
            state
                .start_log_streams
                .end(&resource_key, "failed", "Start aborted");
        }
    }
    result
}

async fn start_database_container_inner(
    state: &Arc<AppState>,
    id: &str,
    resource_key: &str,
) -> anyhow::Result<()> {
    let database = sqlx::query_as::<_, ManagedDatabase>("SELECT * FROM databases WHERE id = ?")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    let db_type = database.get_db_type();
    let config = get_config(&db_type);
    let credentials = database
        .get_credentials()
        .ok_or_else(|| anyhow::anyhow!("Invalid credentials"))?;

    // If there's an existing container (previously stopped), try to start it directly
    if let Some(ref existing_container_id) = database.container_id {
        if !existing_container_id.is_empty() {
            tracing::info!(
                "Starting existing database container: {}",
                existing_container_id
            );
            let short = &existing_container_id[..12.min(existing_container_id.len())];
            state.start_log_streams.info(
                resource_key,
                "starting",
                format!("Starting existing container {}", short),
            );

            // Update status to starting
            sqlx::query(
                "UPDATE databases SET status = ?, updated_at = datetime('now') WHERE id = ?",
            )
            .bind(DatabaseStatus::Starting.to_string())
            .bind(id)
            .execute(&state.db)
            .await?;

            if let Err(e) = state.runtime.start(existing_container_id).await {
                tracing::warn!(
                    "Failed to start existing container {}, will create a new one: {}",
                    existing_container_id,
                    e
                );
                // Clear the stale container_id so we fall through to create a new container
                sqlx::query(
                    "UPDATE databases SET container_id = NULL, status = ?, updated_at = datetime('now') WHERE id = ?",
                )
                .bind(DatabaseStatus::Stopped.to_string())
                .bind(id)
                .execute(&state.db)
                .await?;
                // Fall through to create a new container below
            } else {
                // For MySQL/MariaDB: ensure the app user exists even if the data directory
                // was pre-initialized (Docker only creates MYSQL_USER on first boot).
                if matches!(db_type, DatabaseType::Mysql | DatabaseType::Mariadb) {
                    if let Some(ref db_name) = credentials.database {
                        let runtime = state.runtime.clone();
                        let cid = existing_container_id.clone();
                        let creds = credentials.clone();
                        let db_nm = db_name.clone();
                        let dt = db_type.clone();
                        tokio::spawn(async move {
                            ensure_mysql_user(&runtime, &cid, &creds, &db_nm, &dt).await;
                        });
                    }
                }

                // Get the assigned host port if public access is enabled
                let external_port = if database.is_public() {
                    if database.external_port > 0 {
                        database.external_port
                    } else {
                        let info = state.runtime.inspect(existing_container_id).await?;
                        info.host_port.unwrap_or(0) as i32
                    }
                } else {
                    0
                };

                sqlx::query(
                "UPDATE databases SET external_port = ?, status = ?, error_message = NULL, updated_at = datetime('now') WHERE id = ?",
            )
            .bind(external_port)
            .bind(DatabaseStatus::Running.to_string())
            .bind(id)
            .execute(&state.db)
            .await?;

                tracing::info!(
                    "Database {} started successfully (existing container: {})",
                    database.name,
                    existing_container_id
                );
                state.start_log_streams.info(
                    resource_key,
                    "running",
                    format!(
                        "Container {} started",
                        &existing_container_id[..12.min(existing_container_id.len())]
                    ),
                );

                return Ok(());
            } // end else (start succeeded)
        }
    }

    // No existing container — create a new one
    // Update status to pulling
    sqlx::query("UPDATE databases SET status = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(DatabaseStatus::Pulling.to_string())
        .bind(id)
        .execute(&state.db)
        .await?;

    // Pull the image — use custom_image if set, otherwise build from default config
    let image = if let Some(ref custom) = database.custom_image {
        custom.clone()
    } else {
        format!("{}:{}", config.image, database.version)
    };
    tracing::info!("Pulling database image: {}", image);
    state
        .start_log_streams
        .info(resource_key, "pulling", format!("Pulling image {}", image));
    state.runtime.pull_image(&image, None).await?;
    state
        .start_log_streams
        .info(resource_key, "pulling", format!("Image {} ready", image));

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
        // Use custom external port if specified (non-zero), otherwise auto-assign
        let host_port = if database.external_port > 0 {
            database.external_port as u16
        } else {
            0 // auto-assign
        };
        port_mappings.push(PortMapping::new(host_port, config.port));
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
        restart_policy: "unless-stopped".to_string(),
        privileged: false,
        cap_add: vec![],
        cap_drop: vec![],
        devices: vec![],
        shm_size: None,
        init: false,
        app_id: None,
        gpus: None,
        ulimits: vec![],
        security_opt: vec![],
        cmd: None,
        network: None,
        custom_labels: vec![],
    };

    // Start the container
    tracing::info!("Starting database container: {}", container_name);
    state.start_log_streams.info(
        resource_key,
        "starting",
        format!("Creating container {}", container_name),
    );
    let container_id = state.runtime.run(&run_config).await?;
    state.start_log_streams.info(
        resource_key,
        "starting",
        format!(
            "Container {} created",
            &container_id[..12.min(container_id.len())]
        ),
    );

    // Get the assigned host port if public access is enabled
    let external_port = if database.is_public() {
        if database.external_port > 0 {
            // Custom port was specified, use it
            database.external_port
        } else {
            // Auto-assigned port, get from container info
            let info = state.runtime.inspect(&container_id).await?;
            info.host_port.unwrap_or(0) as i32
        }
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
    state.start_log_streams.info(
        resource_key,
        "running",
        format!("Database {} started successfully", database.name),
    );

    // For MySQL/MariaDB: ensure the app user exists. Docker only runs its init scripts
    // (which create MYSQL_USER) when the data directory is empty on first boot. If the
    // bind-mount directory already has data from a previous container, the user is never
    // created and the app cannot connect. We provision the user idempotently via socket.
    if matches!(db_type, DatabaseType::Mysql | DatabaseType::Mariadb) {
        if let Some(ref db_name) = credentials.database {
            let runtime = state.runtime.clone();
            let cid = container_id.clone();
            let creds = credentials.clone();
            let db_nm = db_name.clone();
            let dt = db_type.clone();
            tokio::spawn(async move {
                ensure_mysql_user(&runtime, &cid, &creds, &db_nm, &dt).await;
            });
        }
    }

    // For ClickHouse, create the database after the server is ready (CLICKHOUSE_DB env var
    // sets the default but does not always auto-create it; this ensures it exists).
    if db_type == DatabaseType::ClickHouse {
        if let Some(ref db_name) = credentials.database {
            tracing::info!(
                "Creating ClickHouse database '{}' for {}",
                db_name,
                database.name
            );
            // Wait for ClickHouse HTTP server to be ready
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let create_cmd = vec![
                "clickhouse-client".to_string(),
                "--password".to_string(),
                credentials.password.clone(),
                "--query".to_string(),
                format!("CREATE DATABASE IF NOT EXISTS `{}`", db_name),
            ];
            match state.runtime.run_command(&container_id, create_cmd).await {
                Ok(result) if result.exit_code != 0 => {
                    tracing::warn!(
                        "ClickHouse CREATE DATABASE for {} exited with code {}: {}",
                        database.name,
                        result.exit_code,
                        result.stderr
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to create ClickHouse database for {}: {}",
                        database.name,
                        e
                    );
                }
                _ => {
                    tracing::info!(
                        "ClickHouse database '{}' created for {}",
                        db_name,
                        database.name
                    );
                }
            }
        }
    }

    // Execute init commands if present (only on first start — container_id was NULL before)
    if let Some(ref init_json) = database.init_commands {
        if let Ok(commands) = serde_json::from_str::<Vec<String>>(init_json) {
            if !commands.is_empty() {
                tracing::info!(
                    "Executing {} init command(s) for database {}",
                    commands.len(),
                    database.name
                );
                // Wait briefly for the database process to become ready inside the container
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                for (i, cmd) in commands.iter().enumerate() {
                    let exec_cmd = build_init_exec_cmd(&db_type, &credentials, cmd);
                    match state.runtime.run_command(&container_id, exec_cmd).await {
                        Ok(result) => {
                            if result.exit_code != 0 {
                                tracing::warn!(
                                    "Init command {} for database {} exited with code {}: {}",
                                    i + 1,
                                    database.name,
                                    result.exit_code,
                                    result.stderr
                                );
                            } else {
                                tracing::info!(
                                    "Init command {} for database {} succeeded",
                                    i + 1,
                                    database.name
                                );
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to run init command {} for database {}: {}",
                                i + 1,
                                database.name,
                                e
                            );
                        }
                    }
                }
            }
        } else {
            tracing::warn!(
                "Database {} has invalid init_commands JSON, skipping",
                database.name
            );
        }
    }

    Ok(())
}

/// Ensure the MySQL/MariaDB app user and database exist inside the container.
///
/// The official Docker MySQL/MariaDB image only runs its init scripts (which create
/// `MYSQL_USER`) when the data directory is **empty** on first boot. If a bind-mount
/// directory already contains data from a previous container, the user is never created
/// and any app that relies on it gets `SQLSTATE[HY000] [1130] Host … not allowed`.
///
/// This function connects via the Unix socket as root (no password required) and issues
/// `CREATE USER IF NOT EXISTS … GRANT …` — a no-op when the user already exists.
async fn ensure_mysql_user(
    runtime: &std::sync::Arc<dyn crate::runtime::ContainerRuntime>,
    container_id: &str,
    credentials: &crate::db::DatabaseCredentials,
    db_name: &str,
    db_type: &DatabaseType,
) {
    let client = match db_type {
        DatabaseType::Mysql => "mysql",
        DatabaseType::Mariadb => "mariadb",
        _ => return,
    };

    // Escape single quotes for SQL string literals (passwords are auto-generated
    // alphanumerics so this is purely defensive)
    let safe_pass = credentials.password.replace('\'', "''");
    let safe_user = credentials.username.replace('\'', "''");
    let safe_db = db_name.replace('`', "");

    let sql = format!(
        "CREATE USER IF NOT EXISTS '{safe_user}'@'%' IDENTIFIED BY '{safe_pass}'; \
         CREATE DATABASE IF NOT EXISTS `{safe_db}`; \
         GRANT ALL PRIVILEGES ON `{safe_db}`.* TO '{safe_user}'@'%'; \
         FLUSH PRIVILEGES;"
    );

    // Retry up to 6 × 5 s = 30 s for MySQL to finish initialising
    for attempt in 1u8..=6 {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        let cmd = vec![
            client.to_string(),
            "--socket=/var/run/mysqld/mysqld.sock".to_string(),
            "-uroot".to_string(),
            "-e".to_string(),
            sql.clone(),
        ];

        match runtime.run_command(container_id, cmd).await {
            Ok(result) if result.exit_code == 0 => {
                tracing::info!(
                    user = %credentials.username,
                    database = %db_name,
                    "MySQL/MariaDB user provisioned (or already exists)"
                );
                return;
            }
            Ok(result) => {
                tracing::debug!(
                    attempt,
                    exit_code = result.exit_code,
                    stderr = %result.stderr,
                    "MySQL/MariaDB user provisioning attempt {} failed, retrying",
                    attempt
                );
            }
            Err(e) => {
                tracing::debug!(
                    attempt,
                    error = %e,
                    "MySQL/MariaDB user provisioning exec error on attempt {}, retrying",
                    attempt
                );
            }
        }
    }

    tracing::warn!(
        user = %credentials.username,
        database = %db_name,
        "Could not provision MySQL/MariaDB user after 30 s — \
         the app may fail to connect until the database container is restarted"
    );
}

/// Build the exec command vector for running a SQL init command inside a database container.
fn build_init_exec_cmd(
    db_type: &DatabaseType,
    credentials: &crate::db::DatabaseCredentials,
    sql: &str,
) -> Vec<String> {
    match db_type {
        DatabaseType::Postgres => vec![
            "psql".to_string(),
            "-U".to_string(),
            credentials.username.clone(),
            "-d".to_string(),
            credentials
                .database
                .clone()
                .unwrap_or_else(|| credentials.username.clone()),
            "-c".to_string(),
            sql.to_string(),
        ],
        DatabaseType::Mysql | DatabaseType::Mariadb => {
            let db_name = credentials.database.clone().unwrap_or_default();
            vec![
                "mysql".to_string(),
                "-u".to_string(),
                credentials.username.clone(),
                format!("-p{}", credentials.password),
                db_name,
                "-e".to_string(),
                sql.to_string(),
            ]
        }
        DatabaseType::Mongodb => {
            let db_name = credentials
                .database
                .clone()
                .unwrap_or_else(|| "admin".to_string());
            vec![
                "mongosh".to_string(),
                db_name,
                "--eval".to_string(),
                sql.to_string(),
            ]
        }
        DatabaseType::Redis | DatabaseType::Dragonfly | DatabaseType::Keydb => {
            // Redis-compatible stores do not support SQL init commands; skip gracefully
            vec!["redis-cli".to_string(), "ping".to_string()]
        }
        DatabaseType::ClickHouse => {
            vec![
                "clickhouse-client".to_string(),
                "--password".to_string(),
                credentials.password.clone(),
                "--query".to_string(),
                sql.to_string(),
            ]
        }
    }
}

/// Update a managed database configuration
pub async fn update_database(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<crate::db::UpdateManagedDatabaseRequest>,
) -> Result<Json<ManagedDatabaseResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Get the database record
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

    // Check if public_access is changing
    let public_access_changed =
        req.public_access.is_some() && req.public_access.unwrap() != database.is_public();

    // Validate external port if provided
    if let Some(port) = req.external_port {
        if port != 0 && !(1024..=65535).contains(&port) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid external port",
                    "message": "External port must be 0 (auto-assign) or between 1024 and 65535"
                })),
            ));
        }

        // Check for port conflicts when a specific port is requested
        if port != 0 {
            let existing_service: Option<(String,)> =
                sqlx::query_as("SELECT name FROM services WHERE port = ?")
                    .bind(port)
                    .fetch_optional(&state.db)
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to check port conflict in services: {}", e);
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({"error": "Failed to check port availability"})),
                        )
                    })?;

            if let Some((name,)) = existing_service {
                return Err((
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({
                        "error": "Port conflict",
                        "message": format!("Port {} is already used by service '{}'", port, name)
                    })),
                ));
            }

            // Check other databases (excluding this one)
            let existing_db: Option<(String,)> = sqlx::query_as(
                "SELECT name FROM databases WHERE external_port = ? AND public_access = 1 AND id != ?",
            )
            .bind(port)
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to check port conflict in databases: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Failed to check port availability"})),
                )
            })?;

            if let Some((name,)) = existing_db {
                return Err((
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({
                        "error": "Port conflict",
                        "message": format!("Port {} is already used by database '{}'", port, name)
                    })),
                ));
            }
        }
    }

    // Build the update query dynamically
    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn std::any::Any + Send + Sync>> = Vec::new();

    if let Some(public_access) = req.public_access {
        updates.push("public_access = ?");
        values.push(Box::new(if public_access { 1i32 } else { 0i32 }));
    }

    if let Some(ref memory_limit) = req.memory_limit {
        updates.push("memory_limit = ?");
        values.push(Box::new(memory_limit.clone()));
    }

    if let Some(ref cpu_limit) = req.cpu_limit {
        updates.push("cpu_limit = ?");
        values.push(Box::new(cpu_limit.clone()));
    }

    if let Some(external_port) = req.external_port {
        updates.push("external_port = ?");
        values.push(Box::new(external_port));
    }

    if let Some(ssl_enabled) = req.ssl_enabled {
        updates.push("ssl_enabled = ?");
        values.push(Box::new(if ssl_enabled { 1i32 } else { 0i32 }));
    }

    if let Some(ref ssl_mode) = req.ssl_mode {
        updates.push("ssl_mode = ?");
        values.push(Box::new(ssl_mode.clone()));
    }

    if req.custom_image.is_some() {
        updates.push("custom_image = ?");
        // value tracked separately via req.custom_image below
    }

    if req.init_commands.is_some() {
        updates.push("init_commands = ?");
        // value tracked separately via req.init_commands below
    }

    if updates.is_empty() {
        // No changes
        let hostname = state.config.public_hostname();
        let host = Some(hostname.as_str());
        return Ok(Json(database.to_response(false, host)));
    }

    updates.push("updated_at = datetime('now')");

    // Execute the update using direct bindings
    let update_sql = format!("UPDATE databases SET {} WHERE id = ?", updates.join(", "));

    // Build and execute the query manually since we have dynamic bindings
    let mut query = sqlx::query(&update_sql);

    if let Some(public_access) = req.public_access {
        query = query.bind(if public_access { 1i32 } else { 0i32 });
    }
    if let Some(ref memory_limit) = req.memory_limit {
        query = query.bind(memory_limit);
    }
    if let Some(ref cpu_limit) = req.cpu_limit {
        query = query.bind(cpu_limit);
    }
    if let Some(external_port) = req.external_port {
        query = query.bind(external_port);
    }
    if let Some(ssl_enabled) = req.ssl_enabled {
        query = query.bind(if ssl_enabled { 1i32 } else { 0i32 });
    }
    if let Some(ref ssl_mode) = req.ssl_mode {
        query = query.bind(ssl_mode.as_str());
    }
    if let Some(ref custom_image) = req.custom_image {
        query = query.bind(custom_image.as_str());
    }
    if let Some(ref init_commands) = req.init_commands {
        query = query.bind(init_commands.as_str());
    }
    query = query.bind(&id);

    query.execute(&state.db).await.map_err(|e| {
        tracing::error!("Failed to update database: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to update database"})),
        )
    })?;

    // If public_access changed and database is running, we need to restart it
    let needs_restart = public_access_changed && database.get_status() == DatabaseStatus::Running;

    if needs_restart {
        tracing::info!(
            "Public access changed for database {}, restarting container",
            database.name
        );

        // Stop the existing container
        if let Some(ref container_id) = database.container_id {
            if let Err(e) = state.runtime.stop(container_id).await {
                tracing::warn!("Failed to stop container during restart: {}", e);
            }
            if let Err(e) = state.runtime.remove(container_id).await {
                tracing::warn!("Failed to remove container during restart: {}", e);
            }
        }

        // Start with new configuration
        let state_clone = state.clone();
        let id_clone = id.clone();
        tokio::spawn(async move {
            if let Err(e) = start_database_container(&state_clone, &id_clone).await {
                tracing::error!("Failed to restart database container: {}", e);
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
    }

    // Fetch the updated database record
    let database = sqlx::query_as::<_, ManagedDatabase>("SELECT * FROM databases WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get updated database: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to get updated database"})),
            )
        })?;

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::DATABASE_UPDATE,
        resource_types::DATABASE,
        Some(&database.id),
        Some(&database.name),
        Some(&user.id),
        ip.as_deref(),
        Some(serde_json::json!({
            "public_access_changed": public_access_changed,
            "needs_restart": needs_restart,
        })),
    )
    .await;

    let hostname = state.config.public_hostname();
    let host = Some(hostname.as_str());
    Ok(Json(database.to_response(false, host)))
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
        tracing::warn!(
            "Database {} is not running (status: {})",
            id,
            database.status
        );
        return Err(StatusCode::NOT_FOUND);
    }

    // Get container ID
    let container_id = database.container_id.ok_or_else(|| {
        tracing::warn!("Database {} has no container ID", id);
        StatusCode::NOT_FOUND
    })?;

    // Get stats from the container runtime
    let stats = state.runtime.stats(&container_id).await.map_err(|e| {
        tracing::warn!("Failed to get container stats for {}: {}", container_id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(stats))
}

/// Import a database dump into a running database container
pub async fn import_database_dump(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    use base64::Engine as _;

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

    if database.get_status() != DatabaseStatus::Running {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Database must be running to import a dump"})),
        ));
    }

    let container_id = database.container_id.as_deref().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Database has no container"})),
        )
    })?;

    // Read the uploaded file from multipart
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("Failed to read multipart field: {}", e)})),
        )
    })? {
        if field.name() == Some("file") {
            file_name = field.file_name().map(|s| s.to_string());
            file_bytes = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| {
                        (
                            StatusCode::BAD_REQUEST,
                            Json(
                                serde_json::json!({"error": format!("Failed to read file: {}", e)}),
                            ),
                        )
                    })?
                    .to_vec(),
            );
            break;
        }
    }

    let bytes = file_bytes.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "No file field found in request"})),
        )
    })?;

    if bytes.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Uploaded file is empty"})),
        ));
    }

    // Encode the dump as base64 and write it into the container
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let dest_path = "/tmp/rivetr_import_dump";
    let write_arg = format!("echo '{}' | base64 -d > {}", encoded, dest_path);

    let write_result = state
        .runtime
        .run_command(container_id, vec!["sh".to_string(), "-c".to_string(), write_arg])
        .await
        .map_err(|e| {
            tracing::error!("Failed to write dump into container: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to write dump to container: {}", e)})),
            )
        })?;

    if write_result.exit_code != 0 {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to write dump file to container",
                "stderr": write_result.stderr
            })),
        ));
    }

    let creds = database.get_credentials().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to read database credentials"})),
        )
    })?;

    let db_type_str = database.db_type.as_str();
    let is_gz = file_name
        .as_deref()
        .map(|n| n.ends_with(".gz"))
        .unwrap_or(false);

    // Build the restore command based on DB type
    let restore_cmd: Vec<String> = match db_type_str {
        "postgres" => {
            let db_name = creds
                .database
                .clone()
                .unwrap_or_else(|| creds.username.clone());
            if is_gz {
                // Custom format via pg_restore
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!(
                        "PGPASSWORD='{}' pg_restore -U {} -d {} {}",
                        creds.password, creds.username, db_name, dest_path
                    ),
                ]
            } else {
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!(
                        "PGPASSWORD='{}' psql -U {} -d {} -f {}",
                        creds.password, creds.username, db_name, dest_path
                    ),
                ]
            }
        }
        "mysql" => {
            let db_name = creds
                .database
                .clone()
                .unwrap_or_else(|| creds.username.clone());
            vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "mysql -u {} -p'{}' {} < {}",
                    creds.username, creds.password, db_name, dest_path
                ),
            ]
        }
        "mariadb" => {
            let db_name = creds
                .database
                .clone()
                .unwrap_or_else(|| creds.username.clone());
            vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "mariadb -u {} -p'{}' {} < {}",
                    creds.username, creds.password, db_name, dest_path
                ),
            ]
        }
        "mongodb" => {
            vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "mongorestore --username {} --password '{}' --authenticationDatabase admin --archive={} --gzip",
                    creds.username, creds.password, dest_path
                ),
            ]
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Import not supported for this database type"})),
            ));
        }
    };

    let restore_result = state
        .runtime
        .run_command(container_id, restore_cmd)
        .await
        .map_err(|e| {
            tracing::error!("Failed to run restore command: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to run restore command: {}", e)})),
            )
        })?;

    if restore_result.exit_code != 0 {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Restore command failed",
                "stderr": restore_result.stderr,
                "stdout": restore_result.stdout
            })),
        ));
    }

    // Clean up the temp file
    let _ = state
        .runtime
        .run_command(
            container_id,
            vec!["rm".to_string(), "-f".to_string(), dest_path.to_string()],
        )
        .await;

    tracing::info!(
        "Successfully imported dump into database {} ({})",
        database.name,
        database.db_type
    );

    Ok(Json(serde_json::json!({
        "message": "Database dump imported successfully",
        "database_id": id
    })))
}
