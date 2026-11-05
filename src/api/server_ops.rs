//! Server operations API: image inventory, Docker cleanup and package patching
//! for remote servers and the local runtime (server id `localhost`).
//!
//! `localhost` is the machine Rivetr runs on, so every route for it requires an
//! instance administrator. Remote server routes follow the normal server
//! authorization (team scoped, writes need the developer role).

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::db::{Server, User};
use crate::engine::server_ops::{
    self, cleanup, images, patching, HostTarget, MaintenanceSettings, LOCALHOST_ID,
};
use crate::AppState;

use super::authz;
use super::error::{ApiError, ErrorCode};

/// Resolve and authorize the target host for `id`.
async fn target_for(
    state: &Arc<AppState>,
    user: &User,
    id: &str,
) -> Result<(HostTarget, String), ApiError> {
    if id == LOCALHOST_ID {
        authz::require_privileged(user)?;
    } else {
        authz::authorize_server(state, user, id).await?;
    }
    server_ops::resolve_target(&state.db, &state.config, id)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("Server not found"))
}

fn upstream(e: anyhow::Error) -> ApiError {
    ApiError::new(ErrorCode::ExternalServiceError, e.to_string())
}

fn internal(e: impl std::fmt::Display) -> ApiError {
    ApiError::internal(e.to_string())
}

#[derive(Debug, Serialize)]
pub struct JobStarted {
    pub id: String,
}

// ---------------------------------------------------------------------------
// Images
// ---------------------------------------------------------------------------

/// GET /api/servers/{id}/images
pub async fn list_images(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
) -> Result<Json<images::ImageInventory>, ApiError> {
    let (target, _) = target_for(&state, &user, &id).await?;
    let inv = images::list_images(&state.db, &target)
        .await
        .map_err(upstream)?;
    Ok(Json(inv))
}

/// DELETE /api/servers/{id}/images/{image_id}
///
/// Only images that no container (running or stopped) uses can be removed.
pub async fn delete_image(
    State(state): State<Arc<AppState>>,
    user: User,
    Path((id, image_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    if !images::is_valid_image_id(&image_id) {
        return Err(ApiError::bad_request("Invalid image id"));
    }
    let (target, _) = target_for(&state, &user, &id).await?;
    match images::delete_unused_image(&state.db, &target, &image_id)
        .await
        .map_err(upstream)?
    {
        images::DeleteImageOutcome::Deleted => Ok(StatusCode::NO_CONTENT),
        images::DeleteImageOutcome::NotFound => Err(ApiError::not_found("Image not found")),
        images::DeleteImageOutcome::InUse(names) => Err(ApiError::conflict(format!(
            "Image is used by: {}",
            names.join(", ")
        ))),
        images::DeleteImageOutcome::Failed(msg) => Err(ApiError::new(
            ErrorCode::ExternalServiceError,
            format!("Failed to remove image: {msg}"),
        )),
    }
}

// ---------------------------------------------------------------------------
// Maintenance settings
// ---------------------------------------------------------------------------

/// GET /api/servers/{id}/maintenance
pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
) -> Result<Json<MaintenanceSettings>, ApiError> {
    target_for(&state, &user, &id).await?;
    let s = server_ops::load_settings(&state.db, &id)
        .await
        .map_err(internal)?;
    Ok(Json(s))
}

/// Partial update of maintenance settings.
#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub cleanup_enabled: Option<bool>,
    pub cleanup_cron: Option<String>,
    pub cleanup_trigger: Option<String>,
    pub cleanup_threshold_pct: Option<i64>,
    pub remove_unused_volumes: Option<bool>,
    pub remove_unused_networks: Option<bool>,
    pub remove_retained_images: Option<bool>,
    pub patch_check_enabled: Option<bool>,
    pub patch_check_cron: Option<String>,
    pub auto_security_updates: Option<bool>,
    pub maintenance_window: Option<String>,
}

/// Apply a partial update to settings.
pub fn apply_update(mut s: MaintenanceSettings, req: UpdateSettingsRequest) -> MaintenanceSettings {
    if let Some(v) = req.cleanup_enabled {
        s.cleanup_enabled = v;
    }
    if let Some(v) = req.cleanup_cron {
        s.cleanup_cron = v.trim().to_string();
    }
    if let Some(v) = req.cleanup_trigger {
        s.cleanup_trigger = v;
    }
    if let Some(v) = req.cleanup_threshold_pct {
        s.cleanup_threshold_pct = v;
    }
    if let Some(v) = req.remove_unused_volumes {
        s.remove_unused_volumes = v;
    }
    if let Some(v) = req.remove_unused_networks {
        s.remove_unused_networks = v;
    }
    if let Some(v) = req.remove_retained_images {
        s.remove_retained_images = v;
    }
    if let Some(v) = req.patch_check_enabled {
        s.patch_check_enabled = v;
    }
    if let Some(v) = req.patch_check_cron {
        s.patch_check_cron = v.trim().to_string();
    }
    if let Some(v) = req.auto_security_updates {
        s.auto_security_updates = v;
    }
    if let Some(v) = req.maintenance_window {
        s.maintenance_window = v.trim().to_string();
    }
    s
}

/// PUT /api/servers/{id}/maintenance
///
/// Turning automatic security updates on or off (or moving the maintenance
/// window while they are on) starts a background job that configures the host.
pub async fn update_settings(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
    Json(req): Json<UpdateSettingsRequest>,
) -> Result<Json<MaintenanceSettings>, ApiError> {
    let (target, _) = target_for(&state, &user, &id).await?;
    let old = server_ops::load_settings(&state.db, &id)
        .await
        .map_err(internal)?;
    let new = apply_update(old.clone(), req);
    new.validate().map_err(ApiError::bad_request)?;

    let auto_changed = old.auto_security_updates != new.auto_security_updates
        || (new.auto_security_updates && old.maintenance_window != new.maintenance_window);
    if auto_changed {
        if let Some(st) = old.patch_state() {
            if new.auto_security_updates && !patching::supports_auto_updates(&st.package_manager) {
                return Err(ApiError::bad_request(format!(
                    "Automatic security updates are only supported with apt or dnf (this server uses {})",
                    st.package_manager
                )));
            }
        }
        if patching::running_job(&state.db, &id)
            .await
            .map_err(internal)?
            .is_some()
        {
            return Err(ApiError::conflict(
                "Another maintenance job is running on this server",
            ));
        }
    }

    server_ops::save_settings(&state.db, &new)
        .await
        .map_err(internal)?;

    if auto_changed {
        patching::start_job(
            state.db.clone(),
            id.clone(),
            target,
            patching::PatchJobKind::ConfigureAutoUpdates {
                enabled: new.auto_security_updates,
                window: new.maintenance_window.clone(),
            },
        )
        .await
        .map_err(internal)?;
    }

    let saved = server_ops::load_settings(&state.db, &id)
        .await
        .map_err(internal)?;
    Ok(Json(saved))
}

// ---------------------------------------------------------------------------
// Docker cleanup
// ---------------------------------------------------------------------------

/// GET /api/servers/{id}/cleanup/runs
pub async fn list_cleanup_runs(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
) -> Result<Json<Vec<cleanup::CleanupRun>>, ApiError> {
    target_for(&state, &user, &id).await?;
    let runs = cleanup::recent_runs(&state.db, &id, 25)
        .await
        .map_err(internal)?;
    Ok(Json(runs))
}

/// POST /api/servers/{id}/cleanup
///
/// Starts a cleanup with the saved options in the background (202).
pub async fn run_cleanup(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<JobStarted>), ApiError> {
    let (target, _) = target_for(&state, &user, &id).await?;
    if cleanup::is_running(&state.db, &id)
        .await
        .map_err(internal)?
    {
        return Err(ApiError::conflict(
            "A cleanup is already running on this server",
        ));
    }
    let settings = server_ops::load_settings(&state.db, &id)
        .await
        .map_err(internal)?;
    let run_id = cleanup::start_run(
        state.db.clone(),
        id,
        target,
        cleanup::CleanupOptions::from(&settings),
        "manual",
    )
    .await
    .map_err(internal)?;
    Ok((StatusCode::ACCEPTED, Json(JobStarted { id: run_id })))
}

// ---------------------------------------------------------------------------
// Patching
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct PatchingResponse {
    pub settings: MaintenanceSettings,
    pub state: Option<patching::PatchState>,
    pub running_job: Option<patching::PatchJob>,
    pub recent_jobs: Vec<patching::PatchJob>,
    pub supports_auto_updates: bool,
}

/// GET /api/servers/{id}/patching
pub async fn get_patching(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
) -> Result<Json<PatchingResponse>, ApiError> {
    target_for(&state, &user, &id).await?;
    let settings = server_ops::load_settings(&state.db, &id)
        .await
        .map_err(internal)?;
    let patch_state = settings.patch_state();
    let running_job = patching::running_job(&state.db, &id)
        .await
        .map_err(internal)?;
    let recent_jobs = patching::recent_jobs(&state.db, &id, 10)
        .await
        .map_err(internal)?;
    let supports_auto_updates = patch_state
        .as_ref()
        .map(|s| patching::supports_auto_updates(&s.package_manager))
        .unwrap_or(false);
    Ok(Json(PatchingResponse {
        settings,
        state: patch_state,
        running_job,
        recent_jobs,
        supports_auto_updates,
    }))
}

async fn start_patch_job(
    state: &Arc<AppState>,
    user: &User,
    id: &str,
    kind: patching::PatchJobKind,
) -> Result<(StatusCode, Json<JobStarted>), ApiError> {
    let (target, _) = target_for(state, user, id).await?;
    if patching::running_job(&state.db, id)
        .await
        .map_err(internal)?
        .is_some()
    {
        return Err(ApiError::conflict(
            "Another maintenance job is running on this server",
        ));
    }
    let job_id = patching::start_job(state.db.clone(), id.to_string(), target, kind)
        .await
        .map_err(internal)?;
    Ok((StatusCode::ACCEPTED, Json(JobStarted { id: job_id })))
}

/// POST /api/servers/{id}/patching/check
pub async fn check_updates(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<JobStarted>), ApiError> {
    start_patch_job(&state, &user, &id, patching::PatchJobKind::Check).await
}

#[derive(Debug, Deserialize)]
pub struct UpdatePackagesRequest {
    /// Package to update; omit to update every package.
    pub package: Option<String>,
}

/// POST /api/servers/{id}/patching/update
pub async fn update_packages(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
    Json(req): Json<UpdatePackagesRequest>,
) -> Result<(StatusCode, Json<JobStarted>), ApiError> {
    let kind = match req.package {
        Some(p) => {
            if !patching::is_valid_package_name(&p) {
                return Err(ApiError::bad_request("Invalid package name"));
            }
            patching::PatchJobKind::UpdatePackage(p)
        }
        None => patching::PatchJobKind::UpdateAll,
    };
    let has_pm = server_ops::load_settings(&state.db, &id)
        .await
        .map_err(internal)?
        .patch_state()
        .is_some_and(|s| patching::update_command(&s.package_manager, None).is_some());
    if !has_pm {
        return Err(ApiError::bad_request(
            "Check for updates first so Rivetr knows the package manager",
        ));
    }
    start_patch_job(&state, &user, &id, kind).await
}

/// POST /api/servers/{id}/reboot
pub async fn reboot_server(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<JobStarted>), ApiError> {
    start_patch_job(&state, &user, &id, patching::PatchJobKind::Reboot).await
}

// ---------------------------------------------------------------------------
// Servers list summary
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ServerSummary {
    pub server_id: String,
    pub name: String,
    pub host: String,
    pub resource_count: i64,
    pub update_count: usize,
    pub security_update_count: usize,
    pub reboot_required: bool,
    pub last_patch_check_at: Option<String>,
    /// reachable | attention | unreachable | unknown
    pub status: String,
}

/// Overall status for the servers list.
pub fn summary_status(server_status: &str, updates: usize, reboot_required: bool) -> &'static str {
    match server_status {
        "online" if reboot_required || updates > 0 => "attention",
        "online" => "reachable",
        "offline" => "unreachable",
        _ => "unknown",
    }
}

/// GET /api/server-ops/summary
///
/// Resource counts, cached patch info and status for every server the caller
/// can see, plus `localhost` for instance administrators.
pub async fn servers_summary(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<Vec<ServerSummary>>, ApiError> {
    let privileged = authz::is_privileged_user(&user);
    let all = sqlx::query_as::<_, Server>("SELECT * FROM servers ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await
        .map_err(internal)?;
    let servers: Vec<Server> = if privileged {
        all
    } else {
        let teams = authz::user_team_ids(&state, &user.id).await?;
        all.into_iter()
            .filter(|s| s.team_id.as_ref().is_none_or(|t| teams.contains(t)))
            .collect()
    };

    let mut out = Vec::new();
    if privileged {
        let settings = server_ops::load_settings(&state.db, LOCALHOST_ID)
            .await
            .map_err(internal)?;
        let ps = settings.patch_state().unwrap_or_default();
        let count: i64 = sqlx::query_scalar(
            "SELECT (SELECT COUNT(*) FROM apps WHERE server_id IS NULL) \
             + (SELECT COUNT(*) FROM databases) + (SELECT COUNT(*) FROM services)",
        )
        .fetch_one(&state.db)
        .await
        .map_err(internal)?;
        out.push(ServerSummary {
            server_id: LOCALHOST_ID.to_string(),
            name: "localhost".to_string(),
            host: "This Rivetr instance".to_string(),
            resource_count: count,
            update_count: ps.actionable_count(),
            security_update_count: ps.security_count(),
            reboot_required: ps.reboot_required,
            last_patch_check_at: settings.last_patch_check_at.clone(),
            status: summary_status("online", ps.actionable_count(), ps.reboot_required).to_string(),
        });
    }

    for s in servers {
        let settings = server_ops::load_settings(&state.db, &s.id)
            .await
            .map_err(internal)?;
        let ps = settings.patch_state().unwrap_or_default();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM apps WHERE server_id = ? \
             OR id IN (SELECT app_id FROM app_server_assignments WHERE server_id = ?)",
        )
        .bind(&s.id)
        .bind(&s.id)
        .fetch_one(&state.db)
        .await
        .map_err(internal)?;
        out.push(ServerSummary {
            server_id: s.id.clone(),
            name: s.name.clone(),
            host: s.host.clone(),
            resource_count: count,
            update_count: ps.actionable_count(),
            security_update_count: ps.security_count(),
            reboot_required: ps.reboot_required,
            last_patch_check_at: settings.last_patch_check_at.clone(),
            status: summary_status(&s.status, ps.actionable_count(), ps.reboot_required)
                .to_string(),
        });
    }
    Ok(Json(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_mapping() {
        assert_eq!(summary_status("online", 0, false), "reachable");
        assert_eq!(summary_status("online", 3, false), "attention");
        assert_eq!(summary_status("online", 0, true), "attention");
        assert_eq!(summary_status("offline", 3, true), "unreachable");
        assert_eq!(summary_status("unknown", 0, false), "unknown");
    }

    #[test]
    fn partial_settings_update() {
        let s = MaintenanceSettings::defaults("x");
        let updated = apply_update(
            s,
            UpdateSettingsRequest {
                cleanup_enabled: Some(true),
                cleanup_cron: Some(" 0 4 * * * ".into()),
                cleanup_trigger: None,
                cleanup_threshold_pct: None,
                remove_unused_volumes: Some(true),
                remove_unused_networks: None,
                remove_retained_images: None,
                patch_check_enabled: None,
                patch_check_cron: None,
                auto_security_updates: None,
                maintenance_window: None,
            },
        );
        assert!(updated.cleanup_enabled);
        assert_eq!(updated.cleanup_cron, "0 4 * * *");
        assert!(updated.remove_unused_volumes);
        assert!(!updated.remove_unused_networks);
        assert!(updated.validate().is_ok());
    }
}
