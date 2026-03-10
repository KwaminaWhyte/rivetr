//! Docker Swarm management API endpoints.
//!
//! Provides swarm initialization, node management, and service management
//! using the Docker CLI via `tokio::process::Command`.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{SwarmNode, SwarmService};
use crate::AppState;

use super::error::ApiError;

// ---------------------------------------------------------------------------
// Docker CLI helper
// ---------------------------------------------------------------------------

/// Run a docker CLI command and return its stdout as a String.
async fn run_docker(args: &[&str]) -> Result<String, ApiError> {
    let output = tokio::process::Command::new("docker")
        .args(args)
        .output()
        .await
        .map_err(|e| ApiError::internal(format!("Failed to run docker: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ApiError::internal(format!(
            "docker command failed: {}",
            stderr
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct SwarmInitResponse {
    pub node_id: String,
    pub manager_token: String,
    pub worker_token: String,
}

#[derive(Debug, Serialize)]
pub struct SwarmStatusResponse {
    pub node_id: Option<String>,
    pub is_manager: bool,
    pub node_count: u64,
    pub managers: u64,
    pub workers: u64,
    pub local_node_state: String,
}

#[derive(Debug, Deserialize)]
pub struct NodeAvailabilityRequest {
    /// One of: "active", "pause", "drain"
    pub availability: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateServiceRequest {
    pub app_id: Option<String>,
    pub service_name: String,
    pub image: String,
    pub replicas: Option<i64>,
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ScaleServiceRequest {
    pub replicas: i64,
}

// ---------------------------------------------------------------------------
// Swarm management endpoints
// ---------------------------------------------------------------------------

/// POST /api/swarm/init — Initialize the Docker Swarm on this node
pub async fn init_swarm(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SwarmInitResponse>, ApiError> {
    // Initialize swarm
    run_docker(&["swarm", "init"]).await?;

    // Get the manager join token
    let manager_token = run_docker(&["swarm", "join-token", "-q", "manager"])
        .await?
        .trim()
        .to_string();

    // Get the worker join token
    let worker_token = run_docker(&["swarm", "join-token", "-q", "worker"])
        .await?
        .trim()
        .to_string();

    // Get swarm node ID
    let node_id = run_docker(&["info", "--format", "{{.Swarm.NodeID}}"])
        .await?
        .trim()
        .to_string();

    let now = chrono::Utc::now().to_rfc3339();

    // Store config entries
    for (key, value) in [
        ("node_id", node_id.as_str()),
        ("manager_token", manager_token.as_str()),
        ("worker_token", worker_token.as_str()),
    ] {
        sqlx::query(
            r#"
            INSERT INTO swarm_config (key, value, updated_at)
            VALUES (?, ?, ?)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
            "#,
        )
        .bind(key)
        .bind(value)
        .bind(&now)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to store swarm config: {}", e);
            ApiError::database("Failed to store swarm config")
        })?;
    }

    Ok(Json(SwarmInitResponse {
        node_id,
        manager_token,
        worker_token,
    }))
}

/// GET /api/swarm/status — Get swarm status from docker info
pub async fn get_swarm_status(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<SwarmStatusResponse>, ApiError> {
    let raw = run_docker(&[
        "info",
        "--format",
        "{{json .Swarm}}",
    ])
    .await?;

    let info: serde_json::Value = serde_json::from_str(raw.trim()).map_err(|e| {
        ApiError::internal(format!("Failed to parse docker swarm info: {}", e))
    })?;

    let local_node_state = info["LocalNodeState"]
        .as_str()
        .unwrap_or("inactive")
        .to_string();
    let is_manager = info["ControlAvailable"].as_bool().unwrap_or(false);
    let node_id = info["NodeID"].as_str().map(|s| s.to_string());
    let node_count = info["Nodes"].as_u64().unwrap_or(0);
    let managers = info["Managers"].as_u64().unwrap_or(0);
    let workers = node_count.saturating_sub(managers);

    Ok(Json(SwarmStatusResponse {
        node_id,
        is_manager,
        node_count,
        managers,
        workers,
        local_node_state,
    }))
}

/// POST /api/swarm/leave — Leave the swarm (force)
pub async fn leave_swarm(
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    run_docker(&["swarm", "leave", "--force"]).await?;

    // Clear stored swarm config
    sqlx::query("DELETE FROM swarm_config")
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to clear swarm config: {}", e);
            ApiError::database("Failed to clear swarm config")
        })?;

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Node management endpoints
// ---------------------------------------------------------------------------

/// GET /api/swarm/nodes — List nodes from DB
pub async fn list_nodes(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<SwarmNode>>, ApiError> {
    let nodes = sqlx::query_as::<_, SwarmNode>(
        "SELECT * FROM swarm_nodes ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list swarm nodes: {}", e);
        ApiError::database("Failed to list swarm nodes")
    })?;

    Ok(Json(nodes))
}

/// POST /api/swarm/sync-nodes — Sync nodes from `docker node ls`
pub async fn sync_nodes(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<SwarmNode>>, ApiError> {
    // Each line from `docker node ls --format json` is a separate JSON object
    let raw = run_docker(&["node", "ls", "--format", "{{json .}}"]).await?;

    let now = chrono::Utc::now().to_rfc3339();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let node_info: serde_json::Value =
            serde_json::from_str(line).map_err(|e| {
                ApiError::internal(format!("Failed to parse node info: {}", e))
            })?;

        let node_id = node_info["ID"].as_str().unwrap_or("").to_string();
        let hostname = node_info["Hostname"].as_str().unwrap_or("").to_string();
        let role = node_info["Role"]
            .as_str()
            .unwrap_or("worker")
            .to_lowercase();
        let status = node_info["Status"]
            .as_str()
            .unwrap_or("unknown")
            .to_lowercase();
        let availability = node_info["Availability"]
            .as_str()
            .unwrap_or("active")
            .to_lowercase();

        if node_id.is_empty() {
            continue;
        }

        // Upsert node — match by node_id
        let existing: Option<SwarmNode> =
            sqlx::query_as("SELECT * FROM swarm_nodes WHERE node_id = ?")
                .bind(&node_id)
                .fetch_optional(&state.db)
                .await
                .map_err(ApiError::from)?;

        if let Some(existing) = existing {
            sqlx::query(
                r#"
                UPDATE swarm_nodes SET
                    hostname = ?,
                    role = ?,
                    status = ?,
                    availability = ?,
                    last_seen_at = ?
                WHERE id = ?
                "#,
            )
            .bind(&hostname)
            .bind(&role)
            .bind(&status)
            .bind(&availability)
            .bind(&now)
            .bind(&existing.id)
            .execute(&state.db)
            .await
            .map_err(ApiError::from)?;
        } else {
            let id = Uuid::new_v4().to_string();
            sqlx::query(
                r#"
                INSERT INTO swarm_nodes
                    (id, node_id, hostname, role, status, availability, last_seen_at, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&id)
            .bind(&node_id)
            .bind(&hostname)
            .bind(&role)
            .bind(&status)
            .bind(&availability)
            .bind(&now)
            .bind(&now)
            .execute(&state.db)
            .await
            .map_err(ApiError::from)?;
        }
    }

    // Return updated list
    let nodes = sqlx::query_as::<_, SwarmNode>(
        "SELECT * FROM swarm_nodes ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::from)?;

    Ok(Json(nodes))
}

/// PUT /api/swarm/nodes/:id/availability — Drain or activate a node
pub async fn update_node_availability(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<NodeAvailabilityRequest>,
) -> Result<Json<SwarmNode>, ApiError> {
    let valid_values = ["active", "pause", "drain"];
    if !valid_values.contains(&req.availability.as_str()) {
        return Err(ApiError::bad_request(
            "availability must be one of: active, pause, drain",
        ));
    }

    // Fetch node from DB
    let node = sqlx::query_as::<_, SwarmNode>("SELECT * FROM swarm_nodes WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found("Swarm node not found"))?;

    // Run docker node update
    run_docker(&[
        "node",
        "update",
        "--availability",
        &req.availability,
        &node.node_id,
    ])
    .await?;

    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "UPDATE swarm_nodes SET availability = ?, last_seen_at = ? WHERE id = ?",
    )
    .bind(&req.availability)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(ApiError::from)?;

    let updated = sqlx::query_as::<_, SwarmNode>("SELECT * FROM swarm_nodes WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(updated))
}

// ---------------------------------------------------------------------------
// Service management endpoints
// ---------------------------------------------------------------------------

/// GET /api/swarm/services — List swarm services from DB
pub async fn list_services(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<SwarmService>>, ApiError> {
    let services = sqlx::query_as::<_, SwarmService>(
        "SELECT * FROM swarm_services ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list swarm services: {}", e);
        ApiError::database("Failed to list swarm services")
    })?;

    Ok(Json(services))
}

/// POST /api/swarm/services — Create a swarm service
pub async fn create_service(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateServiceRequest>,
) -> Result<(StatusCode, Json<SwarmService>), ApiError> {
    if req.service_name.trim().is_empty() {
        return Err(ApiError::bad_request("service_name is required"));
    }
    if req.image.trim().is_empty() {
        return Err(ApiError::bad_request("image is required"));
    }

    let replicas = req.replicas.unwrap_or(1);
    let mode = req.mode.as_deref().unwrap_or("replicated").to_string();

    // Build docker service create command
    let replicas_str = replicas.to_string();
    let mut args = vec![
        "service",
        "create",
        "--name",
        &req.service_name,
        "--replicas",
        &replicas_str,
    ];

    if mode == "global" {
        args = vec!["service", "create", "--name", &req.service_name, "--mode", "global"];
    }

    args.push(&req.image);

    let raw_id = run_docker(&args).await?.trim().to_string();

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO swarm_services
            (id, app_id, service_name, service_id, replicas, mode, image, status, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, 'running', ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.app_id)
    .bind(&req.service_name)
    .bind(if raw_id.is_empty() { None } else { Some(raw_id) })
    .bind(replicas)
    .bind(&mode)
    .bind(&req.image)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(ApiError::from)?;

    let service = sqlx::query_as::<_, SwarmService>("SELECT * FROM swarm_services WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(ApiError::from)?;

    Ok((StatusCode::CREATED, Json(service)))
}

/// DELETE /api/swarm/services/:id — Remove a swarm service
pub async fn delete_service(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let service =
        sqlx::query_as::<_, SwarmService>("SELECT * FROM swarm_services WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(ApiError::from)?
            .ok_or_else(|| ApiError::not_found("Swarm service not found"))?;

    // Remove from Docker if we have the service name
    if let Err(e) = run_docker(&["service", "rm", &service.service_name]).await {
        tracing::warn!(
            "Failed to remove Docker service '{}': {}",
            service.service_name,
            e
        );
    }

    sqlx::query("DELETE FROM swarm_services WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(ApiError::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/swarm/services/:id/scale — Scale a swarm service
pub async fn scale_service(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ScaleServiceRequest>,
) -> Result<Json<SwarmService>, ApiError> {
    if req.replicas < 0 {
        return Err(ApiError::bad_request("replicas must be >= 0"));
    }

    let service =
        sqlx::query_as::<_, SwarmService>("SELECT * FROM swarm_services WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(ApiError::from)?
            .ok_or_else(|| ApiError::not_found("Swarm service not found"))?;

    let scale_arg = format!("{}={}", service.service_name, req.replicas);
    run_docker(&["service", "scale", &scale_arg]).await?;

    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "UPDATE swarm_services SET replicas = ?, updated_at = ? WHERE id = ?",
    )
    .bind(req.replicas)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(ApiError::from)?;

    let updated =
        sqlx::query_as::<_, SwarmService>("SELECT * FROM swarm_services WHERE id = ?")
            .bind(&id)
            .fetch_one(&state.db)
            .await
            .map_err(ApiError::from)?;

    Ok(Json(updated))
}

/// GET /api/swarm/services/:id/logs — Get logs for a swarm service
pub async fn get_service_logs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let service =
        sqlx::query_as::<_, SwarmService>("SELECT * FROM swarm_services WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(ApiError::from)?
            .ok_or_else(|| ApiError::not_found("Swarm service not found"))?;

    let output = run_docker(&[
        "service",
        "logs",
        "--no-task-ids",
        "--tail",
        "200",
        &service.service_name,
    ])
    .await
    .unwrap_or_default();

    let lines: Vec<&str> = output.lines().collect();

    Ok(Json(serde_json::json!({ "logs": lines })))
}
