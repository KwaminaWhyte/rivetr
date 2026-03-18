//! API handlers for Cloudflare Tunnel management.
//!
//! Tunnels are started by running a `cloudflare/cloudflared:latest` container
//! on the `rivetr` Docker network.  Routes are stored locally for display
//! purposes only — the actual ingress configuration is managed in the
//! Cloudflare dashboard.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{
    CloudflareTunnel, CloudflareTunnelResponse, CloudflareTunnelRoute, CreateTunnelRequest,
    CreateTunnelRouteRequest, User,
};
use crate::runtime::RunConfig;
use crate::AppState;

// ──────────────────────────────────────────────────────────────────────────────
// List
// ──────────────────────────────────────────────────────────────────────────────

/// `GET /api/tunnels` — list all tunnels with their routes.
pub async fn list_tunnels(
    State(state): State<Arc<AppState>>,
    _user: User,
) -> Result<Json<Vec<CloudflareTunnelResponse>>, StatusCode> {
    let tunnels = sqlx::query_as::<_, CloudflareTunnel>(
        "SELECT * FROM cloudflare_tunnels ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list cloudflare tunnels: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut responses = Vec::with_capacity(tunnels.len());
    for tunnel in tunnels {
        let routes = sqlx::query_as::<_, CloudflareTunnelRoute>(
            "SELECT * FROM cloudflare_tunnel_routes WHERE tunnel_id = ? ORDER BY created_at ASC",
        )
        .bind(&tunnel.id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list routes for tunnel {}: {}", tunnel.id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        responses.push(tunnel.to_response(routes));
    }

    Ok(Json(responses))
}

// ──────────────────────────────────────────────────────────────────────────────
// Create
// ──────────────────────────────────────────────────────────────────────────────

/// `POST /api/tunnels` — create a tunnel record and start the cloudflared container.
pub async fn create_tunnel(
    State(state): State<Arc<AppState>>,
    _user: User,
    Json(req): Json<CreateTunnelRequest>,
) -> Result<(StatusCode, Json<CloudflareTunnelResponse>), StatusCode> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query(
        "INSERT INTO cloudflare_tunnels (id, name, tunnel_token, status, created_at, updated_at)
         VALUES (?, ?, ?, 'stopped', ?, ?)",
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.tunnel_token)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create cloudflare tunnel: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Start the tunnel container asynchronously
    let state_clone = state.clone();
    let id_clone = id.clone();
    tokio::spawn(async move {
        if let Err(e) = start_tunnel_container(&state_clone, &id_clone).await {
            tracing::error!(
                "Failed to start cloudflared container for tunnel {}: {}",
                id_clone,
                e
            );
            let _ = sqlx::query(
                "UPDATE cloudflare_tunnels SET status = 'error', updated_at = datetime('now') WHERE id = ?",
            )
            .bind(&id_clone)
            .execute(&state_clone.db)
            .await;
        }
    });

    let tunnel =
        sqlx::query_as::<_, CloudflareTunnel>("SELECT * FROM cloudflare_tunnels WHERE id = ?")
            .bind(&id)
            .fetch_one(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(tunnel.to_response(vec![]))))
}

// ──────────────────────────────────────────────────────────────────────────────
// Delete
// ──────────────────────────────────────────────────────────────────────────────

/// `DELETE /api/tunnels/:id` — stop the container (if running) and remove the record.
pub async fn delete_tunnel(
    State(state): State<Arc<AppState>>,
    _user: User,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let tunnel =
        sqlx::query_as::<_, CloudflareTunnel>("SELECT * FROM cloudflare_tunnels WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

    // Stop and remove container if one exists
    if let Some(ref cid) = tunnel.container_id {
        if !cid.is_empty() {
            if let Err(e) = state.runtime.stop(cid).await {
                tracing::warn!("Failed to stop cloudflared container {}: {}", cid, e);
            }
            if let Err(e) = state.runtime.remove(cid).await {
                tracing::warn!("Failed to remove cloudflared container {}: {}", cid, e);
            }
        }
    }

    sqlx::query("DELETE FROM cloudflare_tunnels WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

// ──────────────────────────────────────────────────────────────────────────────
// Start / Stop
// ──────────────────────────────────────────────────────────────────────────────

/// `POST /api/tunnels/:id/start` — start the cloudflared container.
pub async fn start_tunnel(
    State(state): State<Arc<AppState>>,
    _user: User,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    // Verify tunnel exists
    sqlx::query_as::<_, CloudflareTunnel>("SELECT * FROM cloudflare_tunnels WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let state_clone = state.clone();
    let id_clone = id.clone();
    tokio::spawn(async move {
        if let Err(e) = start_tunnel_container(&state_clone, &id_clone).await {
            tracing::error!(
                "Failed to start cloudflared container for tunnel {}: {}",
                id_clone,
                e
            );
            let _ = sqlx::query(
                "UPDATE cloudflare_tunnels SET status = 'error', updated_at = datetime('now') WHERE id = ?",
            )
            .bind(&id_clone)
            .execute(&state_clone.db)
            .await;
        }
    });

    Ok(StatusCode::ACCEPTED)
}

/// `POST /api/tunnels/:id/stop` — stop the cloudflared container.
pub async fn stop_tunnel(
    State(state): State<Arc<AppState>>,
    _user: User,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let tunnel =
        sqlx::query_as::<_, CloudflareTunnel>("SELECT * FROM cloudflare_tunnels WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(ref cid) = tunnel.container_id {
        if !cid.is_empty() {
            if let Err(e) = state.runtime.stop(cid).await {
                tracing::warn!("Failed to stop cloudflared container {}: {}", cid, e);
            }
        }
    }

    sqlx::query(
        "UPDATE cloudflare_tunnels SET status = 'stopped', updated_at = datetime('now') WHERE id = ?",
    )
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

// ──────────────────────────────────────────────────────────────────────────────
// Routes
// ──────────────────────────────────────────────────────────────────────────────

/// `GET /api/tunnels/:id/routes` — list routes for a tunnel.
pub async fn list_tunnel_routes(
    State(state): State<Arc<AppState>>,
    _user: User,
    Path(id): Path<String>,
) -> Result<Json<Vec<CloudflareTunnelRoute>>, StatusCode> {
    // Verify tunnel exists
    sqlx::query_as::<_, CloudflareTunnel>("SELECT * FROM cloudflare_tunnels WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let routes = sqlx::query_as::<_, CloudflareTunnelRoute>(
        "SELECT * FROM cloudflare_tunnel_routes WHERE tunnel_id = ? ORDER BY created_at ASC",
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list tunnel routes: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(routes))
}

/// `POST /api/tunnels/:id/routes` — add a route to a tunnel.
pub async fn create_tunnel_route(
    State(state): State<Arc<AppState>>,
    _user: User,
    Path(id): Path<String>,
    Json(req): Json<CreateTunnelRouteRequest>,
) -> Result<(StatusCode, Json<CloudflareTunnelRoute>), StatusCode> {
    // Verify tunnel exists
    sqlx::query_as::<_, CloudflareTunnel>("SELECT * FROM cloudflare_tunnels WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let route_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query(
        "INSERT INTO cloudflare_tunnel_routes (id, tunnel_id, hostname, service_url, app_id, created_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&route_id)
    .bind(&id)
    .bind(&req.hostname)
    .bind(&req.service_url)
    .bind(&req.app_id)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create tunnel route: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let route = sqlx::query_as::<_, CloudflareTunnelRoute>(
        "SELECT * FROM cloudflare_tunnel_routes WHERE id = ?",
    )
    .bind(&route_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(route)))
}

/// `DELETE /api/tunnels/:id/routes/:route_id` — remove a route.
pub async fn delete_tunnel_route(
    State(state): State<Arc<AppState>>,
    _user: User,
    Path((tunnel_id, route_id)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM cloudflare_tunnel_routes WHERE id = ? AND tunnel_id = ?")
        .bind(&route_id)
        .bind(&tunnel_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}

// ──────────────────────────────────────────────────────────────────────────────
// Internal: start cloudflared container
// ──────────────────────────────────────────────────────────────────────────────

async fn start_tunnel_container(state: &Arc<AppState>, id: &str) -> anyhow::Result<()> {
    let tunnel =
        sqlx::query_as::<_, CloudflareTunnel>("SELECT * FROM cloudflare_tunnels WHERE id = ?")
            .bind(id)
            .fetch_one(&state.db)
            .await?;

    // If there's an existing container try to (re)start it
    if let Some(ref existing_cid) = tunnel.container_id {
        if !existing_cid.is_empty() {
            tracing::info!(
                "Restarting existing cloudflared container: {}",
                existing_cid
            );

            sqlx::query(
                "UPDATE cloudflare_tunnels SET status = 'starting', updated_at = datetime('now') WHERE id = ?",
            )
            .bind(id)
            .execute(&state.db)
            .await?;

            state.runtime.start(existing_cid).await?;

            sqlx::query(
                "UPDATE cloudflare_tunnels SET status = 'running', updated_at = datetime('now') WHERE id = ?",
            )
            .bind(id)
            .execute(&state.db)
            .await?;

            tracing::info!(
                "Cloudflared tunnel {} is running (restarted container {})",
                id,
                existing_cid
            );
            return Ok(());
        }
    }

    // No existing container — create a new one
    sqlx::query(
        "UPDATE cloudflare_tunnels SET status = 'starting', updated_at = datetime('now') WHERE id = ?",
    )
    .bind(id)
    .execute(&state.db)
    .await?;

    let container_name = format!("rivetr-tunnel-{}", id);

    let run_config = RunConfig {
        image: "cloudflare/cloudflared:latest".to_string(),
        name: container_name.clone(),
        // cloudflared connects outbound — no inbound port to expose
        port: 0,
        env: vec![],
        memory_limit: None,
        cpu_limit: None,
        port_mappings: vec![],
        network_aliases: vec![container_name.clone()],
        extra_hosts: vec![],
        labels: HashMap::new(),
        binds: vec![],
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
        // Pass `tunnel run --token <TOKEN>` as the CMD override
        cmd: Some(vec![
            "tunnel".to_string(),
            "--no-autoupdate".to_string(),
            "run".to_string(),
            "--token".to_string(),
            tunnel.tunnel_token.clone(),
        ]),
        network: None,
    };

    tracing::info!("Pulling cloudflare/cloudflared:latest");
    state
        .runtime
        .pull_image("cloudflare/cloudflared:latest", None)
        .await?;

    tracing::info!("Starting cloudflared container: {}", container_name);
    let container_id = state.runtime.run(&run_config).await?;

    sqlx::query(
        "UPDATE cloudflare_tunnels SET container_id = ?, status = 'running', updated_at = datetime('now') WHERE id = ?",
    )
    .bind(&container_id)
    .bind(id)
    .execute(&state.db)
    .await?;

    tracing::info!(
        "Cloudflared tunnel {} started (container {})",
        id,
        container_id
    );
    Ok(())
}
