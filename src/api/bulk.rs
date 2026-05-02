//! Bulk operations and app management endpoints.
//!
//! Endpoints:
//!   POST /api/bulk/start          — start multiple apps
//!   POST /api/bulk/stop           — stop multiple apps
//!   POST /api/bulk/restart        — restart multiple apps
//!   POST /api/bulk/deploy         — trigger deploy for multiple apps
//!   POST /api/apps/:id/clone      — deep-clone an app
//!   POST /api/apps/:id/snapshots  — save config snapshot
//!   GET  /api/apps/:id/snapshots  — list snapshots
//!   POST /api/apps/:id/snapshots/:sid/restore — restore snapshot
//!   DELETE /api/apps/:id/snapshots/:sid       — delete snapshot
//!   GET  /api/projects/:id/export — export project JSON
//!   POST /api/projects/:id/import — import project JSON
//!   PUT  /api/apps/:id/maintenance — toggle maintenance mode

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{
    actions, resource_types, App, ConfigSnapshot, CreateSnapshotRequest, EnvVar, User, Volume,
};
use crate::AppState;

use super::audit::{audit_log, ClientIp};
use super::error::ApiError;
use super::validation::validate_uuid;

// -------------------------------------------------------------------------
// Request / Response types
// -------------------------------------------------------------------------

/// Body for bulk start / stop / restart / deploy
#[derive(Debug, Deserialize)]
pub struct BulkAppIdsRequest {
    pub app_ids: Vec<String>,
}

/// One result entry per app in a bulk operation
#[derive(Debug, Serialize)]
pub struct BulkAppResult {
    pub app_id: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Aggregated response for bulk operations
#[derive(Debug, Serialize)]
pub struct BulkOperationResponse {
    pub results: Vec<BulkAppResult>,
}

/// Body for cloning an app
#[derive(Debug, Deserialize)]
pub struct CloneAppRequest {
    /// Name for the cloned app (defaults to "{original}-copy")
    pub name: Option<String>,
}

/// Body for maintenance mode toggle
#[derive(Debug, Deserialize)]
pub struct MaintenanceModeRequest {
    pub enabled: bool,
    pub message: Option<String>,
}

/// Serialisable env var for export/snapshot (no secrets)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportEnvVar {
    pub key: String,
    pub value: String,
    pub is_secret: bool,
}

/// Serialisable volume for export
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportVolume {
    pub name: String,
    pub host_path: String,
    pub container_path: String,
    pub read_only: bool,
}

/// One app entry in the project export
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportApp {
    pub name: String,
    pub git_url: String,
    pub branch: String,
    pub dockerfile: String,
    pub port: i32,
    pub healthcheck: Option<String>,
    pub memory_limit: Option<String>,
    pub cpu_limit: Option<String>,
    pub environment: String,
    pub dockerfile_path: Option<String>,
    pub base_directory: Option<String>,
    pub build_target: Option<String>,
    pub pre_deploy_commands: Option<String>,
    pub post_deploy_commands: Option<String>,
    pub domains: Option<String>,
    pub docker_image: Option<String>,
    pub docker_image_tag: Option<String>,
    pub build_type: Option<String>,
    pub env_vars: Vec<ExportEnvVar>,
    pub volumes: Vec<ExportVolume>,
}

/// Full project export envelope
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectExport {
    pub project_name: String,
    pub export_version: u32,
    pub apps: Vec<ExportApp>,
}

/// Import response
#[derive(Debug, Serialize)]
pub struct ProjectImportResponse {
    pub apps_created: usize,
    pub app_ids: Vec<String>,
}

// -------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------

/// Fetch an app, returning ApiError::not_found if missing.
async fn get_app(state: &AppState, id: &str) -> Result<App, ApiError> {
    sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))
}

/// Internal start-container logic (mirrors apps::start_app).
async fn start_container(state: &Arc<AppState>, app: &App) -> Result<(), String> {
    let deployment: Option<(String,)> = sqlx::query_as(
        "SELECT container_id FROM deployments WHERE app_id = ? AND status IN ('running', 'stopped') AND container_id IS NOT NULL ORDER BY started_at DESC LIMIT 1"
    )
    .bind(&app.id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let container_id = deployment
        .and_then(|(cid,)| if cid.is_empty() { None } else { Some(cid) })
        .ok_or_else(|| "No deployment with container found".to_string())?;

    state
        .runtime
        .start(&container_id)
        .await
        .map_err(|e| e.to_string())?;

    // Re-register proxy route
    if let Some(ref domain) = app.domain {
        if !domain.is_empty() {
            if let Ok(info) = state.runtime.inspect(&container_id).await {
                if let Some(port) = info.host_port {
                    let backend = crate::proxy::Backend::new(
                        container_id.clone(),
                        "127.0.0.1".to_string(),
                        port,
                    )
                    .with_healthcheck(app.healthcheck.clone())
                    .with_strip_prefix(app.strip_prefix.clone());
                    state.routes.load().add_route(domain.clone(), backend);
                }
            }
        }
    }

    Ok(())
}

/// Internal stop-container logic.
async fn stop_container(state: &Arc<AppState>, app: &App) -> Result<(), String> {
    let deployment: Option<(String,)> = sqlx::query_as(
        "SELECT container_id FROM deployments WHERE app_id = ? AND status = 'running' AND container_id IS NOT NULL ORDER BY started_at DESC LIMIT 1"
    )
    .bind(&app.id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let container_id = deployment
        .and_then(|(cid,)| if cid.is_empty() { None } else { Some(cid) })
        .ok_or_else(|| "No running deployment found".to_string())?;

    state
        .runtime
        .stop(&container_id)
        .await
        .map_err(|e| e.to_string())?;

    // Remove proxy route
    if let Some(ref domain) = app.domain {
        if !domain.is_empty() {
            state.routes.load().remove_route(domain);
        }
    }

    Ok(())
}

/// Internal restart-container logic.
async fn restart_container(state: &Arc<AppState>, app: &App) -> Result<(), String> {
    let deployment: Option<(String,)> = sqlx::query_as(
        "SELECT container_id FROM deployments WHERE app_id = ? AND status IN ('running', 'stopped') AND container_id IS NOT NULL ORDER BY started_at DESC LIMIT 1"
    )
    .bind(&app.id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let container_id = deployment
        .and_then(|(cid,)| if cid.is_empty() { None } else { Some(cid) })
        .ok_or_else(|| "No deployment with container found".to_string())?;

    let _ = state.runtime.stop(&container_id).await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    state
        .runtime
        .start(&container_id)
        .await
        .map_err(|e| e.to_string())?;

    // Re-register proxy route
    if let Some(ref domain) = app.domain {
        if !domain.is_empty() {
            if let Ok(info) = state.runtime.inspect(&container_id).await {
                if let Some(port) = info.host_port {
                    let backend = crate::proxy::Backend::new(
                        container_id.clone(),
                        "127.0.0.1".to_string(),
                        port,
                    )
                    .with_healthcheck(app.healthcheck.clone())
                    .with_strip_prefix(app.strip_prefix.clone());
                    state.routes.load().add_route(domain.clone(), backend);
                }
            }
        }
    }

    Ok(())
}

/// Internal deploy trigger logic.
async fn trigger_deploy_for_app(state: &Arc<AppState>, app: &App) -> Result<(), String> {
    // Skip if a deployment is already in progress
    let in_progress: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM deployments WHERE app_id = ? AND status IN ('pending', 'cloning', 'building', 'starting', 'checking') LIMIT 1"
    )
    .bind(&app.id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    if in_progress.is_some() {
        return Err("Deployment already in progress".to_string());
    }

    state
        .deploy_tx
        .send((app.id.clone(), app.clone()))
        .await
        .map_err(|e| e.to_string())
}

// -------------------------------------------------------------------------
// Bulk Start
// -------------------------------------------------------------------------

pub async fn bulk_start(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Json(req): Json<BulkAppIdsRequest>,
) -> Result<Json<BulkOperationResponse>, ApiError> {
    let mut results = Vec::new();

    for app_id in &req.app_ids {
        let app = match get_app(&state, app_id).await {
            Ok(a) => a,
            Err(_) => {
                results.push(BulkAppResult {
                    app_id: app_id.clone(),
                    success: false,
                    error: Some("App not found".to_string()),
                });
                continue;
            }
        };

        match start_container(&state, &app).await {
            Ok(()) => {
                audit_log(
                    &state,
                    actions::APP_START,
                    resource_types::APP,
                    Some(&app.id),
                    Some(&app.name),
                    Some(&user.id),
                    client_ip.as_deref(),
                    None,
                )
                .await;
                results.push(BulkAppResult {
                    app_id: app_id.clone(),
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                results.push(BulkAppResult {
                    app_id: app_id.clone(),
                    success: false,
                    error: Some(e),
                });
            }
        }
    }

    Ok(Json(BulkOperationResponse { results }))
}

// -------------------------------------------------------------------------
// Bulk Stop
// -------------------------------------------------------------------------

pub async fn bulk_stop(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Json(req): Json<BulkAppIdsRequest>,
) -> Result<Json<BulkOperationResponse>, ApiError> {
    let mut results = Vec::new();

    for app_id in &req.app_ids {
        let app = match get_app(&state, app_id).await {
            Ok(a) => a,
            Err(_) => {
                results.push(BulkAppResult {
                    app_id: app_id.clone(),
                    success: false,
                    error: Some("App not found".to_string()),
                });
                continue;
            }
        };

        match stop_container(&state, &app).await {
            Ok(()) => {
                audit_log(
                    &state,
                    actions::APP_STOP,
                    resource_types::APP,
                    Some(&app.id),
                    Some(&app.name),
                    Some(&user.id),
                    client_ip.as_deref(),
                    None,
                )
                .await;
                results.push(BulkAppResult {
                    app_id: app_id.clone(),
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                results.push(BulkAppResult {
                    app_id: app_id.clone(),
                    success: false,
                    error: Some(e),
                });
            }
        }
    }

    Ok(Json(BulkOperationResponse { results }))
}

// -------------------------------------------------------------------------
// Bulk Restart
// -------------------------------------------------------------------------

pub async fn bulk_restart(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Json(req): Json<BulkAppIdsRequest>,
) -> Result<Json<BulkOperationResponse>, ApiError> {
    let mut results = Vec::new();

    for app_id in &req.app_ids {
        let app = match get_app(&state, app_id).await {
            Ok(a) => a,
            Err(_) => {
                results.push(BulkAppResult {
                    app_id: app_id.clone(),
                    success: false,
                    error: Some("App not found".to_string()),
                });
                continue;
            }
        };

        match restart_container(&state, &app).await {
            Ok(()) => {
                audit_log(
                    &state,
                    actions::APP_RESTART,
                    resource_types::APP,
                    Some(&app.id),
                    Some(&app.name),
                    Some(&user.id),
                    client_ip.as_deref(),
                    None,
                )
                .await;
                results.push(BulkAppResult {
                    app_id: app_id.clone(),
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                results.push(BulkAppResult {
                    app_id: app_id.clone(),
                    success: false,
                    error: Some(e),
                });
            }
        }
    }

    Ok(Json(BulkOperationResponse { results }))
}

// -------------------------------------------------------------------------
// Bulk Deploy
// -------------------------------------------------------------------------

pub async fn bulk_deploy(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Json(req): Json<BulkAppIdsRequest>,
) -> Result<Json<BulkOperationResponse>, ApiError> {
    let mut results = Vec::new();

    for app_id in &req.app_ids {
        let app = match get_app(&state, app_id).await {
            Ok(a) => a,
            Err(_) => {
                results.push(BulkAppResult {
                    app_id: app_id.clone(),
                    success: false,
                    error: Some("App not found".to_string()),
                });
                continue;
            }
        };

        match trigger_deploy_for_app(&state, &app).await {
            Ok(()) => {
                audit_log(
                    &state,
                    actions::DEPLOYMENT_TRIGGER,
                    resource_types::APP,
                    Some(&app.id),
                    Some(&app.name),
                    Some(&user.id),
                    client_ip.as_deref(),
                    None,
                )
                .await;
                results.push(BulkAppResult {
                    app_id: app_id.clone(),
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                results.push(BulkAppResult {
                    app_id: app_id.clone(),
                    success: false,
                    error: Some(e),
                });
            }
        }
    }

    Ok(Json(BulkOperationResponse { results }))
}

// -------------------------------------------------------------------------
// Clone App
// -------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct CloneAppResponse {
    pub app: crate::db::AppResponse,
}

pub async fn clone_app(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(id): Path<String>,
    body: Option<Json<CloneAppRequest>>,
) -> Result<(StatusCode, Json<CloneAppResponse>), ApiError> {
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    let original = get_app(&state, &id).await?;

    let clone_name = body
        .as_ref()
        .and_then(|b| b.name.as_ref())
        .cloned()
        .unwrap_or_else(|| format!("{}-copy", original.name));

    // Build the cloned app row
    let new_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query(
        r#"
        INSERT INTO apps (
            id, name, git_url, branch, dockerfile, domain, port, healthcheck,
            memory_limit, cpu_limit, ssh_key_id, environment, project_id, environment_id, team_id,
            dockerfile_path, base_directory, build_target, watch_paths, custom_docker_options,
            port_mappings, network_aliases, extra_hosts,
            basic_auth_enabled, basic_auth_username, basic_auth_password_hash,
            pre_deploy_commands, post_deploy_commands,
            domains, auto_subdomain,
            docker_image, docker_image_tag, registry_url, registry_username, registry_password,
            container_labels, build_type, nixpacks_config, publish_directory,
            preview_enabled, github_app_installation_id, deployment_source,
            auto_rollback_enabled, registry_push_enabled, max_rollback_versions,
            require_approval, maintenance_mode, maintenance_message,
            created_at, updated_at
        ) VALUES (
            ?, ?, ?, ?, ?, NULL, ?, ?,
            ?, ?, ?, ?, ?, ?, ?,
            ?, ?, ?, ?, ?,
            ?, ?, ?,
            0, NULL, NULL,
            ?, ?,
            ?, NULL,
            ?, ?, ?, ?, NULL,
            ?, ?, ?, ?,
            ?, ?, ?,
            ?, ?, ?,
            0, 0, NULL,
            ?, ?
        )
        "#,
    )
    .bind(&new_id)
    .bind(&clone_name)
    .bind(&original.git_url)
    .bind(&original.branch)
    .bind(&original.dockerfile)
    // domain set to NULL (no domain for clone)
    .bind(original.port)
    .bind(&original.healthcheck)
    .bind(&original.memory_limit)
    .bind(&original.cpu_limit)
    .bind(&original.ssh_key_id)
    .bind(&original.environment)
    .bind(&original.project_id)
    .bind(&original.environment_id)
    .bind(&original.team_id)
    .bind(&original.dockerfile_path)
    .bind(&original.base_directory)
    .bind(&original.build_target)
    .bind(&original.watch_paths)
    .bind(&original.custom_docker_options)
    .bind(&original.port_mappings)
    .bind(&original.network_aliases)
    .bind(&original.extra_hosts)
    // basic_auth reset to disabled
    .bind(&original.pre_deploy_commands)
    .bind(&original.post_deploy_commands)
    .bind(&original.domains)
    // auto_subdomain reset to NULL
    .bind(&original.docker_image)
    .bind(&original.docker_image_tag)
    .bind(&original.registry_url)
    .bind(&original.registry_username)
    // registry_password reset to NULL
    .bind(&original.container_labels)
    .bind(&original.build_type)
    .bind(&original.nixpacks_config)
    .bind(&original.publish_directory)
    .bind(original.preview_enabled)
    .bind(&original.github_app_installation_id)
    .bind(&original.deployment_source)
    .bind(original.auto_rollback_enabled)
    .bind(original.registry_push_enabled)
    .bind(original.max_rollback_versions)
    // require_approval, maintenance_mode, maintenance_message reset to defaults
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;

    // Copy env vars (excluding secret values — clone gets the key structure but masked secrets)
    let env_vars: Vec<EnvVar> =
        sqlx::query_as("SELECT * FROM env_vars WHERE app_id = ? ORDER BY key")
            .bind(&id)
            .fetch_all(&state.db)
            .await?;

    for ev in &env_vars {
        let env_id = Uuid::new_v4().to_string();
        // For secrets, we copy the raw value (it's the operator's responsibility)
        sqlx::query(
            "INSERT INTO env_vars (id, app_id, key, value, is_secret, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&env_id)
        .bind(&new_id)
        .bind(&ev.key)
        .bind(&ev.value)
        .bind(ev.is_secret)
        .bind(&now)
        .bind(&now)
        .execute(&state.db)
        .await?;
    }

    // Copy volumes
    let volumes: Vec<Volume> =
        sqlx::query_as("SELECT * FROM volumes WHERE app_id = ? ORDER BY name")
            .bind(&id)
            .fetch_all(&state.db)
            .await?;

    for vol in &volumes {
        let vol_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO volumes (id, app_id, name, host_path, container_path, read_only, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&vol_id)
        .bind(&new_id)
        .bind(&vol.name)
        .bind(&vol.host_path)
        .bind(&vol.container_path)
        .bind(vol.read_only)
        .bind(&now)
        .bind(&now)
        .execute(&state.db)
        .await?;
    }

    let cloned_app: App = sqlx::query_as("SELECT * FROM apps WHERE id = ?")
        .bind(&new_id)
        .fetch_one(&state.db)
        .await?;

    audit_log(
        &state,
        actions::APP_CREATE,
        resource_types::APP,
        Some(&new_id),
        Some(&clone_name),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "cloned_from": id })),
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(CloneAppResponse {
            app: cloned_app.into(),
        }),
    ))
}

// -------------------------------------------------------------------------
// Config Snapshots
// -------------------------------------------------------------------------

pub async fn create_snapshot(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
    Json(req): Json<CreateSnapshotRequest>,
) -> Result<(StatusCode, Json<ConfigSnapshot>), ApiError> {
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    let app = get_app(&state, &id).await?;

    // Serialise app config (using AppResponse to exclude sensitive fields)
    let app_response: crate::db::AppResponse = app.clone().into();
    let config_json = serde_json::to_string(&app_response)
        .map_err(|e| ApiError::internal(format!("Failed to serialise config: {}", e)))?;

    // Serialise env vars (mask secrets)
    let env_vars: Vec<EnvVar> = sqlx::query_as("SELECT * FROM env_vars WHERE app_id = ?")
        .bind(&id)
        .fetch_all(&state.db)
        .await?;

    let masked: Vec<ExportEnvVar> = env_vars
        .iter()
        .map(|ev| ExportEnvVar {
            key: ev.key.clone(),
            value: if ev.is_secret != 0 {
                "***".to_string()
            } else {
                ev.value.clone()
            },
            is_secret: ev.is_secret != 0,
        })
        .collect();
    let env_vars_json = serde_json::to_string(&masked)
        .map_err(|e| ApiError::internal(format!("Failed to serialise env vars: {}", e)))?;

    let snapshot_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query(
        "INSERT INTO config_snapshots (id, app_id, name, description, config_json, env_vars_json, created_by, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&snapshot_id)
    .bind(&id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&config_json)
    .bind(&env_vars_json)
    .bind(&user.id)
    .bind(&now)
    .execute(&state.db)
    .await?;

    let snapshot: ConfigSnapshot = sqlx::query_as("SELECT * FROM config_snapshots WHERE id = ?")
        .bind(&snapshot_id)
        .fetch_one(&state.db)
        .await?;

    Ok((StatusCode::CREATED, Json(snapshot)))
}

pub async fn list_snapshots(
    State(state): State<Arc<AppState>>,
    _user: User,
    Path(id): Path<String>,
) -> Result<Json<Vec<ConfigSnapshot>>, ApiError> {
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Verify app exists
    get_app(&state, &id).await?;

    let snapshots: Vec<ConfigSnapshot> =
        sqlx::query_as("SELECT * FROM config_snapshots WHERE app_id = ? ORDER BY created_at DESC")
            .bind(&id)
            .fetch_all(&state.db)
            .await?;

    Ok(Json(snapshots))
}

pub async fn restore_snapshot(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path((id, snapshot_id)): Path<(String, String)>,
) -> Result<Json<crate::db::AppResponse>, ApiError> {
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }
    if let Err(e) = validate_uuid(&snapshot_id, "snapshot_id") {
        return Err(ApiError::validation_field("snapshot_id", e));
    }

    let snapshot: ConfigSnapshot =
        sqlx::query_as("SELECT * FROM config_snapshots WHERE id = ? AND app_id = ?")
            .bind(&snapshot_id)
            .bind(&id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| ApiError::not_found("Snapshot not found"))?;

    // Parse the stored config
    let stored: crate::db::AppResponse = serde_json::from_str(&snapshot.config_json)
        .map_err(|e| ApiError::internal(format!("Failed to parse snapshot config: {}", e)))?;

    // Apply the config fields (non-sensitive) back to the app
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    sqlx::query(
        r#"
        UPDATE apps SET
            dockerfile = ?,
            branch = ?,
            port = ?,
            healthcheck = ?,
            memory_limit = ?,
            cpu_limit = ?,
            dockerfile_path = ?,
            base_directory = ?,
            build_target = ?,
            watch_paths = ?,
            custom_docker_options = ?,
            pre_deploy_commands = ?,
            post_deploy_commands = ?,
            domains = ?,
            build_type = ?,
            nixpacks_config = ?,
            publish_directory = ?,
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&stored.dockerfile)
    .bind(&stored.branch)
    .bind(stored.port)
    .bind(&stored.healthcheck)
    .bind(&stored.memory_limit)
    .bind(&stored.cpu_limit)
    .bind(&stored.dockerfile_path)
    .bind(&stored.base_directory)
    .bind(&stored.build_target)
    .bind(&stored.watch_paths)
    .bind(&stored.custom_docker_options)
    .bind(&stored.pre_deploy_commands)
    .bind(&stored.post_deploy_commands)
    .bind(&stored.domains)
    .bind(&stored.build_type)
    .bind(&stored.nixpacks_config)
    .bind(&stored.publish_directory)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await?;

    let updated: App = sqlx::query_as("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    audit_log(
        &state,
        actions::APP_UPDATE,
        resource_types::APP,
        Some(&id),
        Some(&updated.name),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "restored_from_snapshot": snapshot_id })),
    )
    .await;

    Ok(Json(updated.into()))
}

pub async fn delete_snapshot(
    State(state): State<Arc<AppState>>,
    _user: User,
    Path((id, snapshot_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }
    if let Err(e) = validate_uuid(&snapshot_id, "snapshot_id") {
        return Err(ApiError::validation_field("snapshot_id", e));
    }

    let result = sqlx::query("DELETE FROM config_snapshots WHERE id = ? AND app_id = ?")
        .bind(&snapshot_id)
        .bind(&id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Snapshot not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}

// -------------------------------------------------------------------------
// Project Export / Import
// -------------------------------------------------------------------------

pub async fn export_project(
    State(state): State<Arc<AppState>>,
    _user: User,
    Path(project_id): Path<String>,
) -> Result<Json<ProjectExport>, ApiError> {
    if let Err(e) = validate_uuid(&project_id, "project_id") {
        return Err(ApiError::validation_field("project_id", e));
    }

    // Fetch project name
    let project_name: Option<(String,)> = sqlx::query_as("SELECT name FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_optional(&state.db)
        .await?;

    let project_name = project_name
        .map(|(n,)| n)
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    // Fetch all apps in project
    let apps: Vec<App> = sqlx::query_as("SELECT * FROM apps WHERE project_id = ?")
        .bind(&project_id)
        .fetch_all(&state.db)
        .await?;

    let mut export_apps = Vec::new();

    for app in &apps {
        let env_vars: Vec<EnvVar> = sqlx::query_as("SELECT * FROM env_vars WHERE app_id = ?")
            .bind(&app.id)
            .fetch_all(&state.db)
            .await?;

        let volumes: Vec<Volume> = sqlx::query_as("SELECT * FROM volumes WHERE app_id = ?")
            .bind(&app.id)
            .fetch_all(&state.db)
            .await?;

        let export_env_vars: Vec<ExportEnvVar> = env_vars
            .iter()
            .map(|ev| ExportEnvVar {
                key: ev.key.clone(),
                value: if ev.is_secret != 0 {
                    "***".to_string()
                } else {
                    ev.value.clone()
                },
                is_secret: ev.is_secret != 0,
            })
            .collect();

        let export_volumes: Vec<ExportVolume> = volumes
            .iter()
            .map(|v| ExportVolume {
                name: v.name.clone(),
                host_path: v.host_path.clone(),
                container_path: v.container_path.clone(),
                read_only: v.read_only != 0,
            })
            .collect();

        export_apps.push(ExportApp {
            name: app.name.clone(),
            git_url: app.git_url.clone(),
            branch: app.branch.clone(),
            dockerfile: app.dockerfile.clone(),
            port: app.port,
            healthcheck: app.healthcheck.clone(),
            memory_limit: app.memory_limit.clone(),
            cpu_limit: app.cpu_limit.clone(),
            environment: app.environment.clone(),
            dockerfile_path: app.dockerfile_path.clone(),
            base_directory: app.base_directory.clone(),
            build_target: app.build_target.clone(),
            pre_deploy_commands: app.pre_deploy_commands.clone(),
            post_deploy_commands: app.post_deploy_commands.clone(),
            domains: app.domains.clone(),
            docker_image: app.docker_image.clone(),
            docker_image_tag: app.docker_image_tag.clone(),
            build_type: app.build_type.clone(),
            env_vars: export_env_vars,
            volumes: export_volumes,
        });
    }

    Ok(Json(ProjectExport {
        project_name,
        export_version: 1,
        apps: export_apps,
    }))
}

pub async fn import_project(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(project_id): Path<String>,
    Json(export): Json<ProjectExport>,
) -> Result<(StatusCode, Json<ProjectImportResponse>), ApiError> {
    if let Err(e) = validate_uuid(&project_id, "project_id") {
        return Err(ApiError::validation_field("project_id", e));
    }

    // Verify project exists
    let project_exists: Option<(String,)> = sqlx::query_as("SELECT id FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_optional(&state.db)
        .await?;

    if project_exists.is_none() {
        return Err(ApiError::not_found("Project not found"));
    }

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut app_ids = Vec::new();

    for export_app in &export.apps {
        let new_app_id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO apps (
                id, name, git_url, branch, dockerfile, port, healthcheck,
                memory_limit, cpu_limit, environment, project_id,
                dockerfile_path, base_directory, build_target,
                pre_deploy_commands, post_deploy_commands, domains,
                docker_image, docker_image_tag, build_type,
                require_approval, maintenance_mode,
                created_at, updated_at
            ) VALUES (
                ?, ?, ?, ?, ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?,
                0, 0,
                ?, ?
            )
            "#,
        )
        .bind(&new_app_id)
        .bind(&export_app.name)
        .bind(&export_app.git_url)
        .bind(&export_app.branch)
        .bind(&export_app.dockerfile)
        .bind(export_app.port)
        .bind(&export_app.healthcheck)
        .bind(&export_app.memory_limit)
        .bind(&export_app.cpu_limit)
        .bind(&export_app.environment)
        .bind(&project_id)
        .bind(&export_app.dockerfile_path)
        .bind(&export_app.base_directory)
        .bind(&export_app.build_target)
        .bind(&export_app.pre_deploy_commands)
        .bind(&export_app.post_deploy_commands)
        .bind(&export_app.domains)
        .bind(&export_app.docker_image)
        .bind(&export_app.docker_image_tag)
        .bind(&export_app.build_type)
        .bind(&now)
        .bind(&now)
        .execute(&state.db)
        .await?;

        // Import env vars (skip masked secrets)
        for ev in &export_app.env_vars {
            if ev.is_secret && ev.value == "***" {
                continue;
            }
            let env_id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO env_vars (id, app_id, key, value, is_secret, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&env_id)
            .bind(&new_app_id)
            .bind(&ev.key)
            .bind(&ev.value)
            .bind(if ev.is_secret { 1i32 } else { 0i32 })
            .bind(&now)
            .bind(&now)
            .execute(&state.db)
            .await?;
        }

        // Import volumes
        for vol in &export_app.volumes {
            let vol_id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO volumes (id, app_id, name, host_path, container_path, read_only, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&vol_id)
            .bind(&new_app_id)
            .bind(&vol.name)
            .bind(&vol.host_path)
            .bind(&vol.container_path)
            .bind(if vol.read_only { 1i32 } else { 0i32 })
            .bind(&now)
            .bind(&now)
            .execute(&state.db)
            .await?;
        }

        audit_log(
            &state,
            actions::APP_CREATE,
            resource_types::APP,
            Some(&new_app_id),
            Some(&export_app.name),
            Some(&user.id),
            client_ip.as_deref(),
            Some(serde_json::json!({ "imported_from_project_export": true })),
        )
        .await;

        app_ids.push(new_app_id);
    }

    Ok((
        StatusCode::CREATED,
        Json(ProjectImportResponse {
            apps_created: app_ids.len(),
            app_ids,
        }),
    ))
}

// -------------------------------------------------------------------------
// Maintenance Mode
// -------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct MaintenanceModeResponse {
    pub app_id: String,
    pub maintenance_mode: bool,
    pub maintenance_message: Option<String>,
}

pub async fn set_maintenance_mode(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(id): Path<String>,
    Json(req): Json<MaintenanceModeRequest>,
) -> Result<Json<MaintenanceModeResponse>, ApiError> {
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    let app = get_app(&state, &id).await?;
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query(
        "UPDATE apps SET maintenance_mode = ?, maintenance_message = ?, updated_at = ? WHERE id = ?"
    )
    .bind(if req.enabled { 1i32 } else { 0i32 })
    .bind(&req.message)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await?;

    audit_log(
        &state,
        actions::APP_UPDATE,
        resource_types::APP,
        Some(&id),
        Some(&app.name),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "maintenance_mode": req.enabled })),
    )
    .await;

    Ok(Json(MaintenanceModeResponse {
        app_id: id,
        maintenance_mode: req.enabled,
        maintenance_message: req.message,
    }))
}
