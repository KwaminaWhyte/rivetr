//! Multi-server placement of an app: which servers it runs on, rollout and
//! load balancing settings, and the per-target records of a deployment.

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;

use crate::db::{
    validate_placements, DeploymentTarget, MultiServerSettings, Server, UpdatePlacementRequest,
    User, LOCAL_SERVER_ID,
};
use crate::engine::multi_server;
use crate::AppState;

use super::authz;
use super::error::ApiError;

/// One server in an app's placement, with how the proxy reaches it.
#[derive(Debug, Serialize)]
pub struct PlacementView {
    pub server_id: String,
    pub server_name: String,
    pub replicas: i64,
    /// Host part of the address the proxy uses for this server's containers.
    pub address: String,
    /// Ports are published on a private (Tailscale) address only.
    pub private_bind: bool,
    pub is_local: bool,
}

#[derive(Debug, Serialize)]
pub struct PlacementResponse {
    /// True when the app deploys through the multi-server rollout.
    pub enabled: bool,
    pub placements: Vec<PlacementView>,
    pub settings: MultiServerSettings,
    /// Distinct servers in the placement.
    pub server_count: usize,
    /// Registry push is on with a registry URL, so images can be pulled.
    pub registry_configured: bool,
    /// Healthy upstreams currently in the app's proxy pool.
    pub upstreams: Vec<String>,
}

async fn build_response(
    state: &Arc<AppState>,
    app: &crate::db::App,
) -> Result<PlacementResponse, ApiError> {
    let rows = multi_server::load_placements(&state.db, &app.id).await;
    let settings = multi_server::load_settings(&state.db, &app.id).await;
    let mut placements = Vec::new();
    for p in &rows {
        if p.server_id == LOCAL_SERVER_ID {
            let a = multi_server::plan::local_addressing();
            placements.push(PlacementView {
                server_id: p.server_id.clone(),
                server_name: "Rivetr host".to_string(),
                replicas: p.replicas,
                address: a.upstream_host,
                private_bind: true,
                is_local: true,
            });
            continue;
        }
        let server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
            .bind(&p.server_id)
            .fetch_optional(&state.db)
            .await?;
        let (name, address, private_bind) = match server {
            Some(s) => {
                let a = multi_server::plan::remote_addressing(&s, settings.bind_private);
                (s.name, a.upstream_host, !a.public_exposure)
            }
            None => ("(deleted server)".to_string(), String::new(), false),
        };
        placements.push(PlacementView {
            server_id: p.server_id.clone(),
            server_name: name,
            replicas: p.replicas,
            address,
            private_bind,
            is_local: false,
        });
    }
    let registry_configured = app.is_registry_push_enabled()
        && app.registry_url.as_deref().is_some_and(|u| !u.is_empty());
    Ok(PlacementResponse {
        enabled: !rows.is_empty(),
        server_count: multi_server::plan::server_count(&rows),
        placements,
        settings,
        registry_configured,
        upstreams: multi_server::healthy_upstreams(&state.db, &app.id).await,
    })
}

/// GET /api/apps/{id}/placement
pub async fn get_placement(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(app_id): Path<String>,
) -> Result<Json<PlacementResponse>, ApiError> {
    let app = authz::authorize_app(&state, &user, &app_id).await?;
    Ok(Json(build_response(&state, &app).await?))
}

/// PUT /api/apps/{id}/placement: replace the placement list and settings.
/// An empty list turns the multi-server rollout off (single-host deploys again).
pub async fn update_placement(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(app_id): Path<String>,
    Json(req): Json<UpdatePlacementRequest>,
) -> Result<Json<PlacementResponse>, ApiError> {
    let app = authz::authorize_app_write(&state, &user, &app_id).await?;
    validate_placements(&req.placements).map_err(ApiError::bad_request)?;
    if let Some(s) = &req.settings {
        s.validate().map_err(ApiError::bad_request)?;
    }
    for p in &req.placements {
        if p.server_id != LOCAL_SERVER_ID {
            authz::authorize_server(&state, &user, &p.server_id).await?;
        }
    }

    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM app_server_placements WHERE app_id = ?")
        .bind(&app_id)
        .execute(&mut *tx)
        .await?;
    for (i, p) in req.placements.iter().enumerate() {
        sqlx::query(
            "INSERT INTO app_server_placements (id, app_id, server_id, replicas, sort_order) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&app_id)
        .bind(&p.server_id)
        .bind(p.replicas)
        .bind(i as i64)
        .execute(&mut *tx)
        .await?;
    }
    if let Some(s) = &req.settings {
        sqlx::query(
            "INSERT INTO app_multi_server_settings \
               (app_id, max_unavailable, failure_threshold_pct, distribution, lb_policy, sticky_sessions, bind_private) \
             VALUES (?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(app_id) DO UPDATE SET \
               max_unavailable = excluded.max_unavailable, \
               failure_threshold_pct = excluded.failure_threshold_pct, \
               distribution = excluded.distribution, \
               lb_policy = excluded.lb_policy, \
               sticky_sessions = excluded.sticky_sessions, \
               bind_private = excluded.bind_private, \
               updated_at = datetime('now')",
        )
        .bind(&app_id)
        .bind(s.max_unavailable)
        .bind(s.failure_threshold_pct)
        .bind(&s.distribution)
        .bind(&s.lb_policy)
        .bind(s.sticky_sessions as i64)
        .bind(s.bind_private as i64)
        .execute(&mut *tx)
        .await?;
    }
    // apps.server_id stays the primary server: the first placement entry
    // (NULL when that is the Rivetr host). Cleared placements leave it alone.
    if let Some(first) = req.placements.first() {
        let primary = (first.server_id != LOCAL_SERVER_ID).then(|| first.server_id.clone());
        sqlx::query("UPDATE apps SET server_id = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(primary)
            .bind(&app_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    // Apply load balancing changes to the live pool without a redeploy.
    let upstreams = multi_server::healthy_upstreams(&state.db, &app_id).await;
    if !req.placements.is_empty() && !upstreams.is_empty() {
        let settings = multi_server::load_settings(&state.db, &app_id).await;
        multi_server::install_routes(&state.db, &state.routes, &app, &upstreams, &settings).await;
    }

    let app = authz::authorize_app(&state, &user, &app_id).await?;
    Ok(Json(build_response(&state, &app).await?))
}

/// GET /api/apps/placement-counts: `{app_id: server_count}` for apps with a
/// placement the caller can see (for the canvas and app list badges).
pub async fn placement_counts(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<HashMap<String, i64>>, ApiError> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT app_id, COUNT(DISTINCT server_id) FROM app_server_placements GROUP BY app_id",
    )
    .fetch_all(&state.db)
    .await?;
    let visible = authz::accessible_app_ids(
        &state,
        &user,
        rows.iter().map(|(id, _)| id.clone()).collect(),
    )
    .await?;
    Ok(Json(
        rows.into_iter()
            .filter(|(id, _)| visible.contains(id))
            .collect(),
    ))
}

/// GET /api/deployments/{id}/targets: per-target status and logs.
pub async fn deployment_targets(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(deployment_id): Path<String>,
) -> Result<Json<Vec<DeploymentTarget>>, ApiError> {
    authz::authorize_deployment(&state, &user, &deployment_id).await?;
    let rows = sqlx::query_as::<_, DeploymentTarget>(
        "SELECT * FROM deployment_targets WHERE deployment_id = ? ORDER BY batch ASC, replica_index ASC, server_name ASC",
    )
    .bind(&deployment_id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

/// Encryption key for server credentials, from config.
pub(crate) fn encryption_key(state: &AppState) -> Option<[u8; 32]> {
    state
        .config
        .auth
        .encryption_key
        .as_ref()
        .map(|secret| crate::crypto::derive_key(secret))
}

/// Start or stop a multi-server app on every server. `None` when the app has no
/// placement, so the caller handles it as a single-host app.
pub(crate) async fn start_stop_if_multi_server(
    state: &Arc<AppState>,
    app: &crate::db::App,
    run: bool,
) -> Option<Result<Json<super::apps::AppStatusResponse>, ApiError>> {
    if !multi_server::is_enabled(&state.db, &app.id).await {
        return None;
    }
    let key = encryption_key(state);
    let result = multi_server::set_running(&state.db, &state.routes, app, key.as_ref(), run)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to {} the app: {:#}", if run { "start" } else { "stop" }, e)))
        .map(|n| {
            tracing::info!(app = %app.name, containers = n, running = run, "Multi-server app state changed");
            Json(super::apps::AppStatusResponse {
                app_id: app.id.clone(),
                container_id: None,
                running: run,
                status: if run { "running" } else { "stopped" }.to_string(),
                host_port: None,
                deployment_phase: "stable".to_string(),
                active_deployment_id: None,
                uptime_seconds: None,
            })
        });
    Some(result)
}
