use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{App, Deployment, DeploymentLog};
use crate::engine::run_rollback;
use crate::proxy::Backend;
use crate::AppState;

use super::validation::validate_uuid;

/// Request body for rollback endpoint
#[derive(Debug, Deserialize)]
pub struct RollbackRequest {
    /// Optional: specify which deployment to roll back to.
    /// If not provided, rolls back to the previous successful deployment.
    pub target_deployment_id: Option<String>,
}

pub async fn trigger_deploy(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<(StatusCode, Json<Deployment>), (StatusCode, String)> {
    // Validate app_id format
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err((StatusCode::BAD_REQUEST, e));
    }

    // Check if app exists
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "App not found".to_string()))?;

    // Check if there's already a deployment in progress
    let in_progress: Option<Deployment> = sqlx::query_as(
        "SELECT * FROM deployments WHERE app_id = ? AND status IN ('pending', 'cloning', 'building', 'starting', 'checking')"
    )
    .bind(&app_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;

    if let Some(existing) = in_progress {
        return Err((
            StatusCode::CONFLICT,
            format!("A deployment is already in progress (id: {})", existing.id),
        ));
    }

    let deployment_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO deployments (id, app_id, status, started_at)
        VALUES (?, ?, 'pending', ?)
        "#,
    )
    .bind(&deployment_id)
    .bind(&app_id)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create deployment".to_string()))?;

    // Queue the deployment job
    if let Err(e) = state.deploy_tx.send((deployment_id.clone(), app)).await {
        tracing::error!("Failed to queue deployment: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to queue deployment job".to_string(),
        ));
    }

    let deployment = sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = ?")
        .bind(&deployment_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch deployment".to_string()))?;

    Ok((StatusCode::ACCEPTED, Json(deployment)))
}

pub async fn list_deployments(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<Deployment>>, (StatusCode, String)> {
    // Validate app_id format
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err((StatusCode::BAD_REQUEST, e));
    }

    // Verify the app exists
    let app_exists: Option<(String,)> = sqlx::query_as("SELECT id FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;

    if app_exists.is_none() {
        return Err((StatusCode::NOT_FOUND, "App not found".to_string()));
    }

    let deployments = sqlx::query_as::<_, Deployment>(
        "SELECT * FROM deployments WHERE app_id = ? ORDER BY started_at DESC LIMIT 50",
    )
    .bind(&app_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch deployments".to_string()))?;

    Ok(Json(deployments))
}

pub async fn get_deployment(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Deployment>, (StatusCode, String)> {
    // Validate deployment_id format
    if let Err(e) = validate_uuid(&id, "deployment_id") {
        return Err((StatusCode::BAD_REQUEST, e));
    }

    let deployment = sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Deployment not found".to_string()))?;

    Ok(Json(deployment))
}

pub async fn get_logs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<DeploymentLog>>, (StatusCode, String)> {
    // Validate deployment_id format
    if let Err(e) = validate_uuid(&id, "deployment_id") {
        return Err((StatusCode::BAD_REQUEST, e));
    }

    // Verify the deployment exists
    let deployment_exists: Option<(String,)> =
        sqlx::query_as("SELECT id FROM deployments WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;

    if deployment_exists.is_none() {
        return Err((StatusCode::NOT_FOUND, "Deployment not found".to_string()));
    }

    let logs = sqlx::query_as::<_, DeploymentLog>(
        "SELECT * FROM deployment_logs WHERE deployment_id = ? ORDER BY id ASC",
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch logs".to_string()))?;

    Ok(Json(logs))
}

/// Rollback a deployment to a previous version
/// POST /api/deployments/:id/rollback
///
/// This endpoint allows rolling back to a previous successful deployment.
/// If no target_deployment_id is provided in the request body, it will
/// automatically roll back to the most recent successful deployment before
/// the current one.
pub async fn rollback_deployment(
    State(state): State<Arc<AppState>>,
    Path(deployment_id): Path<String>,
    Json(body): Json<Option<RollbackRequest>>,
) -> Result<(StatusCode, Json<Deployment>), (StatusCode, String)> {
    // Validate deployment_id format
    if let Err(e) = validate_uuid(&deployment_id, "deployment_id") {
        return Err((StatusCode::BAD_REQUEST, e));
    }

    // Get the current deployment
    let current_deployment = sqlx::query_as::<_, Deployment>(
        "SELECT * FROM deployments WHERE id = ?"
    )
    .bind(&deployment_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Deployment not found".to_string()))?;

    // Get the app
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&current_deployment.app_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "App not found".to_string()))?;

    // Check if there's already a deployment in progress
    let in_progress: Option<Deployment> = sqlx::query_as(
        "SELECT * FROM deployments WHERE app_id = ? AND status IN ('pending', 'cloning', 'building', 'starting', 'checking')"
    )
    .bind(&current_deployment.app_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;

    if let Some(existing) = in_progress {
        return Err((
            StatusCode::CONFLICT,
            format!("A deployment is already in progress (id: {})", existing.id),
        ));
    }

    // Determine target deployment
    let target_deployment = if let Some(ref req) = body {
        if let Some(ref target_id) = req.target_deployment_id {
            // Validate target_deployment_id format
            if let Err(e) = validate_uuid(target_id, "target_deployment_id") {
                return Err((StatusCode::BAD_REQUEST, e));
            }

            // Fetch the specified target deployment
            sqlx::query_as::<_, Deployment>(
                "SELECT * FROM deployments WHERE id = ? AND app_id = ? AND status = 'running'"
            )
            .bind(target_id)
            .bind(&current_deployment.app_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?
            .ok_or((StatusCode::NOT_FOUND, "Target deployment not found or was not successful".to_string()))?
        } else {
            // Find the previous successful deployment
            find_previous_successful_deployment(&state, &current_deployment).await?
        }
    } else {
        // Find the previous successful deployment
        find_previous_successful_deployment(&state, &current_deployment).await?
    };

    // Verify target has an image_tag (required for rollback)
    if target_deployment.image_tag.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Target deployment has no image tag - cannot rollback. This deployment may have been created before rollback support was added.".to_string(),
        ));
    }

    // Create a new deployment record for the rollback
    let rollback_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO deployments (id, app_id, commit_sha, commit_message, status, started_at)
        VALUES (?, ?, ?, ?, 'pending', ?)
        "#,
    )
    .bind(&rollback_id)
    .bind(&current_deployment.app_id)
    .bind(&target_deployment.commit_sha)
    .bind(format!("Rollback to deployment {}", target_deployment.id))
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create rollback deployment".to_string()))?;

    // Run the rollback in a background task
    let db = state.db.clone();
    let runtime = state.runtime.clone();
    let routes = state.routes.clone();
    let rollback_id_clone = rollback_id.clone();
    let target_deployment_clone = target_deployment.clone();
    let app_clone = app.clone();

    tokio::spawn(async move {
        match run_rollback(&db, runtime, &rollback_id_clone, &target_deployment_clone, &app_clone).await {
            Ok(result) => {
                // Update proxy routes on successful rollback
                if let Some(domain) = &app_clone.domain {
                    if let Some(port) = result.port {
                        let backend = Backend::new(
                            result.container_id.clone(),
                            "127.0.0.1".to_string(),
                            port,
                        );
                        routes.load().add_route(domain.clone(), backend);
                        tracing::info!(
                            domain = %domain,
                            port = port,
                            "Proxy route updated after rollback for app {}",
                            app_clone.name
                        );
                    }
                }
            }
            Err(e) => {
                tracing::error!("Rollback {} failed: {}", rollback_id_clone, e);
                let _ = sqlx::query(
                    "UPDATE deployments SET status = 'failed', error_message = ?, finished_at = ? WHERE id = ?"
                )
                .bind(e.to_string())
                .bind(chrono::Utc::now().to_rfc3339())
                .bind(&rollback_id_clone)
                .execute(&db)
                .await;
            }
        }
    });

    // Return the new rollback deployment record
    let deployment = sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = ?")
        .bind(&rollback_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch deployment".to_string()))?;

    Ok((StatusCode::ACCEPTED, Json(deployment)))
}

/// Find the previous successful deployment for an app
async fn find_previous_successful_deployment(
    state: &Arc<AppState>,
    current: &Deployment,
) -> Result<Deployment, (StatusCode, String)> {
    // Find the most recent successful deployment before the current one
    sqlx::query_as::<_, Deployment>(
        r#"
        SELECT * FROM deployments
        WHERE app_id = ?
          AND status = 'running'
          AND id != ?
          AND image_tag IS NOT NULL
        ORDER BY started_at DESC
        LIMIT 1
        "#
    )
    .bind(&current.app_id)
    .bind(&current.id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "No previous successful deployment found to rollback to".to_string()))
}
