//! Clone a resource (app, database or service) into another project, environment or
//! server, optionally copying its volume data.
//!
//! The new resource row is created synchronously so the caller can open it straight
//! away. Copying volume data runs in a background job (a short-lived helper container
//! per volume) tracked in `resource_clone_jobs`; the dashboard polls
//! `GET /api/clone-jobs/:id` for progress. The source resource is never changed.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{App, ManagedDatabase, Service, User, Volume};
use crate::AppState;

use super::audit::{audit_log, ClientIp};
use super::authz;
use super::error::ApiError;

#[derive(Debug, Deserialize)]
pub struct CloneResourceRequest {
    /// Name for the clone. Defaults to "<name>-clone"; made unique when taken.
    pub name: Option<String>,
    /// Target project. `None` keeps the source project, "" clears it.
    pub project_id: Option<String>,
    /// Target environment (apps only). Must belong to the target project.
    pub environment_id: Option<String>,
    /// Target server (apps only). "" means this Rivetr host.
    pub server_id: Option<String>,
    /// Copy the data in the resource's volumes into the clone's own volumes.
    #[serde(default)]
    pub clone_volume_data: bool,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CloneJob {
    pub id: String,
    pub resource_type: String,
    pub source_id: String,
    pub target_id: String,
    pub clone_volume_data: i64,
    /// pending | running | completed | failed
    pub status: String,
    /// 0 to 100
    pub progress: i64,
    pub log: String,
    pub error: Option<String>,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct CloneResourceResponse {
    pub resource_type: String,
    pub resource_id: String,
    pub name: String,
    pub job: CloneJob,
}

/// One volume copy for the background job: (label, source, target).
type CopyStep = (String, String, String);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn validate_name(name: &str) -> Result<(), ApiError> {
    super::validation::validate_app_name(name).map_err(|e| ApiError::validation_field("name", e))
}

/// `base`, or `base-2`, `base-3`, ... until no row in `table` uses the name.
async fn unique_name(state: &AppState, table: &str, base: &str) -> Result<String, ApiError> {
    // `table` comes from a fixed set below, never from the request.
    let sql = format!("SELECT COUNT(*) FROM {} WHERE name = ?", table);
    for i in 1..1000 {
        let candidate = if i == 1 {
            base.to_string()
        } else {
            let suffix = format!("-{}", i);
            let keep = base.len().min(63 - suffix.len());
            format!("{}{}", &base[..keep], suffix)
        };
        let n: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(&candidate)
            .fetch_one(&state.db)
            .await?;
        if n == 0 {
            return Ok(candidate);
        }
    }
    Err(ApiError::conflict(
        "Could not find a free name for the clone",
    ))
}

fn default_clone_name(source: &str) -> String {
    let keep = source.len().min(63 - "-clone".len());
    format!("{}-clone", &source[..keep])
}

/// Resolve the target project: `None` keeps `current`, "" clears, otherwise the caller
/// must have access to the project. Returns (project_id, project team_id).
async fn resolve_project(
    state: &Arc<AppState>,
    user: &User,
    requested: Option<&str>,
    current: Option<&str>,
) -> Result<(Option<String>, Option<String>), ApiError> {
    let target = match requested {
        None => current.map(str::to_string),
        Some("") => None,
        Some(p) => Some(p.to_string()),
    };
    match target {
        Some(pid) => {
            let project = authz::authorize_project(state, user, &pid).await?;
            Ok((Some(pid), project.team_id))
        }
        None => Ok((None, None)),
    }
}

fn data_root(state: &AppState) -> std::path::PathBuf {
    std::fs::canonicalize(&state.config.server.data_dir).unwrap_or_else(|_| {
        std::env::current_dir()
            .unwrap_or_default()
            .join(&state.config.server.data_dir)
    })
}

fn path_string(p: std::path::PathBuf) -> String {
    p.to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('\\', "/")
}

/// Keep only characters that are safe in a directory or volume name.
fn safe_segment(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('.').to_string();
    if trimmed.is_empty() {
        "data".to_string()
    } else {
        trimmed
    }
}

/// New storage location for a cloned app volume. Named volumes get a suffixed name;
/// host paths move under `<data_dir>/volumes/<new app id>/`.
pub fn cloned_volume_location(
    data_root: &std::path::Path,
    new_app_id: &str,
    volume: &Volume,
) -> Option<String> {
    let host = volume.host_path.trim();
    if host.is_empty() {
        return None;
    }
    if host.starts_with('/') || host.starts_with('.') {
        Some(path_string(
            data_root
                .join("volumes")
                .join(new_app_id)
                .join(safe_segment(&volume.name)),
        ))
    } else {
        Some(format!(
            "{}-{}",
            safe_segment(host),
            &new_app_id[..8.min(new_app_id.len())]
        ))
    }
}

/// Top-level named volumes in a compose file that compose scopes to the project
/// (`<project>_<key>`). Volumes with an explicit `name:` or `external: true` are
/// shared by design and are not copied.
pub fn compose_project_volumes(compose: &str) -> Vec<String> {
    let Ok(doc) = serde_yaml::from_str::<serde_yaml::Value>(compose) else {
        return Vec::new();
    };
    let Some(volumes) = doc.get("volumes").and_then(|v| v.as_mapping()) else {
        return Vec::new();
    };
    volumes
        .iter()
        .filter_map(|(k, v)| {
            let key = k.as_str()?.to_string();
            let explicit_name = v.get("name").is_some();
            let external = v
                .get("external")
                .map(|e| e.as_bool().unwrap_or(true))
                .unwrap_or(false);
            (!explicit_name && !external).then_some(key)
        })
        .collect()
}

async fn create_job(
    state: &AppState,
    resource_type: &str,
    source_id: &str,
    target_id: &str,
    clone_volume_data: bool,
    user_id: &str,
    steps: usize,
) -> Result<CloneJob, ApiError> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let (status, progress, log) = if clone_volume_data && steps > 0 {
        (
            "pending",
            5,
            format!("Configuration copied. Copying {} volume(s)...\n", steps),
        )
    } else if clone_volume_data {
        (
            "completed",
            100,
            "Configuration copied. No volume data to copy.\n".to_string(),
        )
    } else {
        ("completed", 100, "Configuration copied.\n".to_string())
    };
    sqlx::query(
        "INSERT INTO resource_clone_jobs (id, resource_type, source_id, target_id, clone_volume_data, \
         status, progress, log, created_by, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(resource_type)
    .bind(source_id)
    .bind(target_id)
    .bind(clone_volume_data as i64)
    .bind(status)
    .bind(progress)
    .bind(&log)
    .bind(user_id)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;
    load_job(state, &id).await
}

async fn load_job(state: &AppState, id: &str) -> Result<CloneJob, ApiError> {
    sqlx::query_as("SELECT * FROM resource_clone_jobs WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Clone job not found"))
}

async fn update_job(
    db: &crate::DbPool,
    id: &str,
    status: &str,
    progress: i64,
    line: &str,
    error: Option<&str>,
) {
    let _ = sqlx::query(
        "UPDATE resource_clone_jobs SET status = ?, progress = ?, log = log || ?, \
         error = COALESCE(?, error), updated_at = ? WHERE id = ?",
    )
    .bind(status)
    .bind(progress)
    .bind(format!("{}\n", line))
    .bind(error)
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(id)
    .execute(db)
    .await;
}

/// Run the volume copies in the background, recording progress on the job.
fn spawn_copy_job(state: Arc<AppState>, job_id: String, steps: Vec<CopyStep>) {
    if steps.is_empty() {
        return;
    }
    tokio::spawn(async move {
        let total = steps.len() as i64;
        let mut failed: Vec<String> = Vec::new();
        update_job(
            &state.db,
            &job_id,
            "running",
            5,
            "Starting volume copy",
            None,
        )
        .await;
        for (i, (label, source, target)) in steps.into_iter().enumerate() {
            update_job(
                &state.db,
                &job_id,
                "running",
                5 + (i as i64 * 90) / total,
                &format!("Copying {} ({} to {})", label, source, target),
                None,
            )
            .await;
            match state.runtime.copy_volume_data(&source, &target).await {
                Ok(()) => {
                    update_job(
                        &state.db,
                        &job_id,
                        "running",
                        5 + ((i as i64 + 1) * 90) / total,
                        &format!("Copied {}", label),
                        None,
                    )
                    .await
                }
                Err(e) => {
                    tracing::warn!(job = %job_id, volume = %label, error = %e, "Volume copy failed");
                    failed.push(label.clone());
                    update_job(
                        &state.db,
                        &job_id,
                        "running",
                        5 + ((i as i64 + 1) * 90) / total,
                        &format!("Failed to copy {}: {:#}", label, e),
                        None,
                    )
                    .await
                }
            }
        }
        if failed.is_empty() {
            update_job(&state.db, &job_id, "completed", 100, "Done", None).await;
        } else {
            let msg = format!("Could not copy: {}", failed.join(", "));
            update_job(&state.db, &job_id, "failed", 100, &msg, Some(&msg)).await;
        }
    });
}

// ---------------------------------------------------------------------------
// Apps
// ---------------------------------------------------------------------------

/// POST /api/apps/:id/clone-resource
pub async fn clone_app_resource(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(id): Path<String>,
    Json(req): Json<CloneResourceRequest>,
) -> Result<(StatusCode, Json<CloneResourceResponse>), ApiError> {
    let source: App = authz::authorize_app_write(&state, &user, &id).await?;

    let base = req
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| default_clone_name(&source.name));
    validate_name(&base)?;
    let name = unique_name(&state, "apps", &base).await?;

    let (project_id, project_team) = resolve_project(
        &state,
        &user,
        req.project_id.as_deref(),
        source.project_id.as_deref(),
    )
    .await?;

    // Environment: explicit choice must belong to the target project. Keeping the
    // project keeps the environment; moving projects without a choice clears it.
    let environment_id = match req.environment_id.as_deref() {
        Some("") => None,
        Some(env) => {
            let owner: Option<(String,)> =
                sqlx::query_as("SELECT project_id FROM environments WHERE id = ?")
                    .bind(env)
                    .fetch_optional(&state.db)
                    .await?;
            match owner {
                Some((pid,)) if Some(&pid) == project_id.as_ref() => Some(env.to_string()),
                Some(_) => {
                    return Err(ApiError::validation_field(
                        "environment_id",
                        "The environment does not belong to the target project",
                    ))
                }
                None => return Err(ApiError::not_found("Environment not found")),
            }
        }
        None if project_id == source.project_id => source.environment_id.clone(),
        None => None,
    };

    let server_id = match req.server_id.as_deref() {
        None => source.server_id.clone(),
        Some("") => None,
        Some(sid) => {
            authz::authorize_server(&state, &user, sid).await?;
            Some(sid.to_string())
        }
    };
    if req.clone_volume_data && (server_id.is_some() || source.server_id.is_some()) {
        return Err(ApiError::validation_field(
            "clone_volume_data",
            "Volume data can only be copied between resources on this Rivetr host",
        ));
    }

    let team_id = project_team.or_else(|| source.team_id.clone());
    authz::authorize_team_assignment(&state, &user, team_id.as_deref()).await?;

    let new_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    super::bulk::insert_cloned_app(&state.db, &source, &new_id, &name, &now).await?;
    // Domains are cleared so the clone never takes over the source's routes.
    sqlx::query(
        "UPDATE apps SET project_id = ?, environment_id = ?, server_id = ?, team_id = ?, \
         healthcheck_config = ?, domains = NULL, domain = NULL WHERE id = ?",
    )
    .bind(&project_id)
    .bind(&environment_id)
    .bind(&server_id)
    .bind(&team_id)
    .bind(&source.healthcheck_config)
    .bind(&new_id)
    .execute(&state.db)
    .await?;

    // Env vars are copied as stored (encrypted values stay valid with the same key).
    sqlx::query(
        "INSERT INTO env_vars (id, app_id, key, value, is_secret, created_at, updated_at) \
         SELECT lower(hex(randomblob(16))), ?, key, value, is_secret, ?, ? FROM env_vars WHERE app_id = ?",
    )
    .bind(&new_id)
    .bind(&now)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await?;

    // Volumes get their own storage so the clone never writes into the source's data.
    let volumes: Vec<Volume> =
        sqlx::query_as("SELECT * FROM volumes WHERE app_id = ? ORDER BY name")
            .bind(&id)
            .fetch_all(&state.db)
            .await?;
    let root = data_root(&state);
    let mut steps: Vec<CopyStep> = Vec::new();
    for vol in &volumes {
        let Some(target) = cloned_volume_location(&root, &new_id, vol) else {
            continue;
        };
        sqlx::query(
            "INSERT INTO volumes (id, app_id, name, host_path, container_path, read_only, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&new_id)
        .bind(&vol.name)
        .bind(&target)
        .bind(&vol.container_path)
        .bind(vol.read_only)
        .bind(&now)
        .bind(&now)
        .execute(&state.db)
        .await?;
        if req.clone_volume_data && !vol.host_path.starts_with('.') {
            steps.push((vol.name.clone(), vol.host_path.clone(), target));
        }
    }

    let job = create_job(
        &state,
        "app",
        &id,
        &new_id,
        req.clone_volume_data,
        &user.id,
        steps.len(),
    )
    .await?;
    spawn_copy_job(state.clone(), job.id.clone(), steps);

    audit_log(
        &state,
        "app.clone",
        "app",
        Some(&new_id),
        Some(&name),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({
            "cloned_from": id,
            "project_id": project_id,
            "environment_id": environment_id,
            "server_id": server_id,
            "clone_volume_data": req.clone_volume_data,
        })),
    )
    .await;

    Ok((
        StatusCode::ACCEPTED,
        Json(CloneResourceResponse {
            resource_type: "app".into(),
            resource_id: new_id,
            name,
            job,
        }),
    ))
}

// ---------------------------------------------------------------------------
// Databases
// ---------------------------------------------------------------------------

/// POST /api/databases/:id/clone-resource
pub async fn clone_database_resource(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(id): Path<String>,
    Json(req): Json<CloneResourceRequest>,
) -> Result<(StatusCode, Json<CloneResourceResponse>), ApiError> {
    let source: ManagedDatabase = authz::authorize_database(&state, &user, &id).await?;

    let base = req
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| default_clone_name(&source.name));
    validate_name(&base)?;
    let name = unique_name(&state, "databases", &base).await?;

    let (project_id, project_team) = resolve_project(
        &state,
        &user,
        req.project_id.as_deref(),
        source.project_id.as_deref(),
    )
    .await?;
    let team_id = project_team.or_else(|| source.team_id.clone());
    authz::authorize_team_assignment(&state, &user, team_id.as_deref()).await?;

    let new_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let container_slug = ManagedDatabase::build_slug(&new_id);
    let volume_name = format!("rivetr-db-{}-data", name);
    let volume_path = path_string(data_root(&state).join("databases").join(format!(
        "{}-{}",
        name,
        &new_id[..8]
    )));

    sqlx::query(
        r#"
        INSERT INTO databases (
            id, name, db_type, version, status, internal_port, external_port,
            public_access, credentials, volume_name, volume_path, memory_limit,
            cpu_limit, project_id, team_id, created_at, updated_at, container_slug,
            custom_image, init_commands
        ) VALUES (?, ?, ?, ?, 'pending', ?, 0, 0, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&new_id)
    .bind(&name)
    .bind(&source.db_type)
    .bind(&source.version)
    .bind(source.internal_port)
    .bind(&source.credentials)
    .bind(&volume_name)
    .bind(&volume_path)
    .bind(&source.memory_limit)
    .bind(&source.cpu_limit)
    .bind(&project_id)
    .bind(&team_id)
    .bind(&now)
    .bind(&now)
    .bind(&container_slug)
    .bind(&source.custom_image)
    .bind(&source.init_commands)
    .execute(&state.db)
    .await?;

    let mut steps: Vec<CopyStep> = Vec::new();
    if req.clone_volume_data {
        match (
            source.volume_path.as_deref().filter(|p| !p.is_empty()),
            source.volume_name.as_deref().filter(|n| !n.is_empty()),
        ) {
            (Some(path), _) => steps.push(("data".into(), path.to_string(), volume_path.clone())),
            (None, Some(vol)) => steps.push(("data".into(), vol.to_string(), volume_name.clone())),
            _ => {}
        }
    }

    let job = create_job(
        &state,
        "database",
        &id,
        &new_id,
        req.clone_volume_data,
        &user.id,
        steps.len(),
    )
    .await?;
    spawn_copy_job(state.clone(), job.id.clone(), steps);

    audit_log(
        &state,
        "database.clone",
        "database",
        Some(&new_id),
        Some(&name),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({
            "cloned_from": id,
            "project_id": project_id,
            "clone_volume_data": req.clone_volume_data,
        })),
    )
    .await;

    Ok((
        StatusCode::ACCEPTED,
        Json(CloneResourceResponse {
            resource_type: "database".into(),
            resource_id: new_id,
            name,
            job,
        }),
    ))
}

// ---------------------------------------------------------------------------
// Services
// ---------------------------------------------------------------------------

/// POST /api/services/:id/clone-resource
pub async fn clone_service_resource(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(id): Path<String>,
    Json(req): Json<CloneResourceRequest>,
) -> Result<(StatusCode, Json<CloneResourceResponse>), ApiError> {
    let source: Service = authz::authorize_service(&state, &user, &id).await?;

    let base = req
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| default_clone_name(&source.name));
    validate_name(&base)?;
    let name = unique_name(&state, "services", &base).await?;

    let (project_id, project_team) = resolve_project(
        &state,
        &user,
        req.project_id.as_deref(),
        source.project_id.as_deref(),
    )
    .await?;
    let team_id = project_team.or_else(|| source.team_id.clone());
    authz::authorize_team_assignment(&state, &user, team_id.as_deref()).await?;

    let new_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    // Domain and public port are cleared so the clone never collides with the source.
    sqlx::query(
        r#"
        INSERT INTO services (
            id, name, project_id, team_id, compose_content, domain, port,
            status, isolated_network, raw_compose_mode, cpu_limit, memory_limit,
            created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, NULL, ?, 'pending', ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&new_id)
    .bind(&name)
    .bind(&project_id)
    .bind(&team_id)
    .bind(&source.compose_content)
    .bind(source.port)
    .bind(source.isolated_network)
    .bind(source.raw_compose_mode)
    .bind(&source.cpu_limit)
    .bind(&source.memory_limit)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;

    sqlx::query(
        "INSERT INTO service_env_vars (id, service_id, key, value, is_secret, created_at, updated_at) \
         SELECT lower(hex(randomblob(16))), ?, key, value, is_secret, ?, ? FROM service_env_vars WHERE service_id = ?",
    )
    .bind(&new_id)
    .bind(&now)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await?;

    let mut steps: Vec<CopyStep> = Vec::new();
    if req.clone_volume_data {
        let src_project = source.compose_project_name();
        let dst_project = format!("rivetr-svc-{}", name);
        for key in compose_project_volumes(&source.compose_content) {
            steps.push((
                format!("volume {}", key),
                format!("{}_{}", src_project, key),
                format!("{}_{}", dst_project, key),
            ));
        }
        // Bind mounts relative to the compose file live in the compose directory.
        let data_dir = &state.config.server.data_dir;
        let src_dir = super::services::compose::get_compose_dir(data_dir, &source.name);
        if src_dir.exists() {
            let src_abs = std::fs::canonicalize(&src_dir).unwrap_or(src_dir);
            let dst = data_root(&state).join("services").join(&name);
            steps.push((
                "compose folder".into(),
                path_string(src_abs),
                path_string(dst),
            ));
        }
    }

    let job = create_job(
        &state,
        "service",
        &id,
        &new_id,
        req.clone_volume_data,
        &user.id,
        steps.len(),
    )
    .await?;
    spawn_copy_job(state.clone(), job.id.clone(), steps);

    audit_log(
        &state,
        "service.clone",
        "service",
        Some(&new_id),
        Some(&name),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({
            "cloned_from": id,
            "project_id": project_id,
            "clone_volume_data": req.clone_volume_data,
        })),
    )
    .await;

    Ok((
        StatusCode::ACCEPTED,
        Json(CloneResourceResponse {
            resource_type: "service".into(),
            resource_id: new_id,
            name,
            job,
        }),
    ))
}

// ---------------------------------------------------------------------------
// Job status
// ---------------------------------------------------------------------------

/// GET /api/clone-jobs/:id
pub async fn get_clone_job(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
) -> Result<Json<CloneJob>, ApiError> {
    let job = load_job(&state, &id).await?;
    match job.resource_type.as_str() {
        "app" => {
            authz::authorize_app(&state, &user, &job.target_id).await?;
        }
        "database" => {
            authz::authorize_database(&state, &user, &job.target_id).await?;
        }
        "service" => {
            authz::authorize_service(&state, &user, &job.target_id).await?;
        }
        _ => return Err(ApiError::not_found("Clone job not found")),
    }
    Ok(Json(job))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vol(name: &str, host: &str) -> Volume {
        Volume {
            id: "v".into(),
            app_id: "a".into(),
            name: name.into(),
            host_path: host.into(),
            container_path: "/data".into(),
            read_only: 0,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn cloned_volumes_get_their_own_storage() {
        let root = std::path::Path::new("/var/lib/rivetr");
        let id = "12345678-aaaa-bbbb-cccc-dddddddddddd";
        assert_eq!(
            cloned_volume_location(root, id, &vol("uploads", "/srv/app/uploads")).unwrap(),
            "/var/lib/rivetr/volumes/12345678-aaaa-bbbb-cccc-dddddddddddd/uploads"
        );
        assert_eq!(
            cloned_volume_location(root, id, &vol("cache", "app_cache")).unwrap(),
            "app_cache-12345678"
        );
        // Names cannot climb out of the clone's folder.
        assert_eq!(
            cloned_volume_location(root, id, &vol("../../etc", "/x")).unwrap(),
            "/var/lib/rivetr/volumes/12345678-aaaa-bbbb-cccc-dddddddddddd/-..-etc"
        );
        assert!(cloned_volume_location(root, id, &vol("empty", " ")).is_none());
    }

    #[test]
    fn compose_volumes_skip_named_and_external() {
        let compose = r#"
services:
  db:
    image: postgres
volumes:
  pgdata:
  shared:
    name: global-shared
  ext:
    external: true
  cache: {}
"#;
        let mut v = compose_project_volumes(compose);
        v.sort();
        assert_eq!(v, vec!["cache".to_string(), "pgdata".to_string()]);
        assert!(compose_project_volumes("not: [valid").is_empty());
    }

    #[test]
    fn default_name_fits_the_length_limit() {
        assert_eq!(default_clone_name("web"), "web-clone");
        assert_eq!(default_clone_name(&"a".repeat(63)).len(), 63);
    }
}
