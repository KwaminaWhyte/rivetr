use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{App, Deployment, TeamAuditAction, TeamAuditResourceType};
use crate::engine::run_rollback;
use crate::proxy::Backend;
use crate::AppState;

use crate::api::error::ApiError;
use crate::api::teams::log_team_audit;
use crate::api::validation::validate_uuid;

use super::shared::get_encryption_key;

/// Request body for rollback endpoint
#[derive(Debug, Deserialize)]
pub struct RollbackRequest {
    /// Optional: specify which deployment to roll back to.
    /// If not provided, rolls back to the previous successful deployment.
    pub target_deployment_id: Option<String>,
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
) -> Result<(StatusCode, Json<Deployment>), ApiError> {
    // Validate deployment_id format
    if let Err(e) = validate_uuid(&deployment_id, "deployment_id") {
        return Err(ApiError::validation_field("deployment_id", e));
    }

    // Get the current deployment
    let current_deployment =
        sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = ?")
            .bind(&deployment_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| ApiError::not_found("Deployment not found"))?;

    // Get the app
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&current_deployment.app_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Check if there's already a deployment in progress
    let in_progress: Option<Deployment> = sqlx::query_as(
        "SELECT * FROM deployments WHERE app_id = ? AND status IN ('pending', 'cloning', 'building', 'starting', 'checking')"
    )
    .bind(&current_deployment.app_id)
    .fetch_optional(&state.db)
    .await?;

    if let Some(existing) = in_progress {
        return Err(ApiError::conflict(format!(
            "A deployment is already in progress (id: {})",
            existing.id
        )));
    }

    // Determine target deployment
    let target_deployment = if let Some(ref req) = body {
        if let Some(ref target_id) = req.target_deployment_id {
            // Validate target_deployment_id format
            if let Err(e) = validate_uuid(target_id, "target_deployment_id") {
                return Err(ApiError::validation_field("target_deployment_id", e));
            }

            // Fetch the specified target deployment (allow running, stopped, or replaced statuses)
            sqlx::query_as::<_, Deployment>(
                "SELECT * FROM deployments WHERE id = ? AND app_id = ? AND status IN ('running', 'stopped', 'replaced') AND image_tag IS NOT NULL"
            )
            .bind(target_id)
            .bind(&current_deployment.app_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| ApiError::not_found("Target deployment not found or has no image tag for rollback"))?
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
        return Err(ApiError::bad_request(
            "Target deployment has no image tag - cannot rollback. This deployment may have been created before rollback support was added."
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
    .await?;

    // Run the rollback in a background task
    let db = state.db.clone();
    let runtime = state.runtime.clone();
    let routes = state.routes.clone();
    let rollback_id_clone = rollback_id.clone();
    let target_deployment_clone = target_deployment.clone();
    let app_clone = app.clone();
    let encryption_key = get_encryption_key(&state);

    tokio::spawn(async move {
        match run_rollback(
            &db,
            runtime.clone(),
            &rollback_id_clone,
            &target_deployment_clone,
            &app_clone,
            encryption_key.as_ref(),
        )
        .await
        {
            Ok(result) => {
                // Update proxy routes on successful rollback for all domains
                if let Some(port) = result.port {
                    let all_domains = app_clone.get_all_domain_names();
                    if !all_domains.is_empty() {
                        let route_table = routes.load();
                        for domain in &all_domains {
                            let backend = Backend::new(
                                result.container_id.clone(),
                                "127.0.0.1".to_string(),
                                port,
                            );
                            route_table.add_route(domain.clone(), backend);
                        }
                        tracing::info!(
                            domains = ?all_domains,
                            port = port,
                            "Proxy routes updated after rollback for app {}",
                            app_clone.name
                        );
                    }
                }

                // Zero-downtime: stop old containers AFTER proxy routes are updated.
                // The rollback container is already serving traffic; old one can be torn down.
                if !result.old_container_ids.is_empty() {
                    tracing::info!(
                        old_containers = ?result.old_container_ids,
                        "Stopping old containers after rollback proxy route swap (zero-downtime)"
                    );
                    for old_id in &result.old_container_ids {
                        if old_id == &result.container_id {
                            continue;
                        }
                        let _ = runtime.stop(old_id).await;
                        let _ = runtime.remove(old_id).await;
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
        .await?;

    // Log team audit event if app belongs to a team
    if let Some(ref team_id) = app.team_id {
        if let Err(e) = log_team_audit(
            &state.db,
            team_id,
            None, // User context not available in this function
            TeamAuditAction::DeploymentRolledBack,
            TeamAuditResourceType::Deployment,
            Some(&deployment.id),
            Some(serde_json::json!({
                "app_id": app.id,
                "app_name": app.name,
                "target_deployment_id": target_deployment.id,
            })),
        )
        .await
        {
            tracing::warn!("Failed to log team audit event: {}", e);
        }
    }

    Ok((StatusCode::ACCEPTED, Json(deployment)))
}

/// Find the previous successful deployment for an app
pub async fn find_previous_successful_deployment(
    state: &Arc<AppState>,
    current: &Deployment,
) -> Result<Deployment, ApiError> {
    // Find the most recent deployment with an image_tag that we can roll back to
    // Allow running, stopped, or replaced statuses (these all indicate a deployment that completed successfully)
    sqlx::query_as::<_, Deployment>(
        r#"
        SELECT * FROM deployments
        WHERE app_id = ?
          AND status IN ('running', 'stopped', 'replaced')
          AND id != ?
          AND image_tag IS NOT NULL
        ORDER BY started_at DESC
        LIMIT 1
        "#,
    )
    .bind(&current.app_id)
    .bind(&current.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("No previous successful deployment found to rollback to"))
}
