//! Docker Destinations API — named Docker networks that apps can be assigned to.
//!
//! Mirrors Coolify's "destinations" concept: each destination is a named Docker
//! bridge network. Apps can be assigned to a destination so their containers
//! join that network instead of the default `rivetr` bridge.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use super::error::ApiError;
use crate::db::{CreateDestinationRequest, Destination};
use crate::AppState;

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListDestinationsQuery {
    pub team_id: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// List all destinations, optionally filtered by team_id.
///
/// GET /api/destinations
pub async fn list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListDestinationsQuery>,
) -> Result<Json<Vec<Destination>>, ApiError> {
    let destinations = if let Some(ref team_id) = query.team_id {
        sqlx::query_as::<_, Destination>(
            "SELECT * FROM destinations WHERE team_id = ? ORDER BY created_at DESC",
        )
        .bind(team_id)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, Destination>("SELECT * FROM destinations ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await?
    };

    Ok(Json(destinations))
}

/// Get a single destination by ID.
///
/// GET /api/destinations/:id
pub async fn get_one(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Destination>, ApiError> {
    let destination = sqlx::query_as::<_, Destination>("SELECT * FROM destinations WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Destination not found"))?;

    Ok(Json(destination))
}

/// Create a new destination and ensure the Docker network exists.
///
/// POST /api/destinations
pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateDestinationRequest>,
) -> Result<(StatusCode, Json<Destination>), ApiError> {
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::validation_field(
            "name",
            "Destination name cannot be empty",
        ));
    }

    let network_name = req.network_name.trim().to_string();
    if network_name.is_empty() {
        return Err(ApiError::validation_field(
            "network_name",
            "Network name cannot be empty",
        ));
    }

    // Ensure the Docker network exists (ignore error if it already exists)
    let output = tokio::process::Command::new("docker")
        .args(["network", "create", "--driver", "bridge", &network_name])
        .output()
        .await;

    match output {
        Ok(out) if out.status.success() => {
            tracing::info!("Created Docker network '{}'", network_name);
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            if stderr.contains("already exists") {
                tracing::debug!("Docker network '{}' already exists — reusing", network_name);
            } else {
                tracing::warn!(
                    "docker network create '{}' exited non-zero: {}",
                    network_name,
                    stderr.trim()
                );
            }
        }
        Err(e) => {
            tracing::warn!("Failed to run docker network create: {}", e);
        }
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    sqlx::query(
        r#"
        INSERT INTO destinations (id, name, network_name, server_id, team_id, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&name)
    .bind(&network_name)
    .bind(&req.server_id)
    .bind(&req.team_id)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create destination: {}", e);
        ApiError::database("Failed to create destination")
    })?;

    let destination = sqlx::query_as::<_, Destination>("SELECT * FROM destinations WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    Ok((StatusCode::CREATED, Json(destination)))
}

/// Delete a destination by ID.
///
/// DELETE /api/destinations/:id
pub async fn delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let destination = sqlx::query_as::<_, Destination>("SELECT * FROM destinations WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Destination not found"))?;

    // Clear destination_id on any apps using this destination
    sqlx::query("UPDATE apps SET destination_id = NULL WHERE destination_id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    sqlx::query("DELETE FROM destinations WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    // Optionally prune the Docker network (best-effort, ignore errors)
    let _ = tokio::process::Command::new("docker")
        .args(["network", "rm", &destination.network_name])
        .output()
        .await;

    Ok(Json(
        serde_json::json!({ "message": "Destination deleted" }),
    ))
}
