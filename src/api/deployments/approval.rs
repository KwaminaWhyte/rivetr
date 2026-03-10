use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::db::{App, Deployment, User};
use crate::AppState;

use crate::api::error::ApiError;
use crate::api::validation::validate_uuid;

/// Request body for rejecting a deployment
#[derive(Debug, Deserialize, Default)]
pub struct RejectDeployRequest {
    pub reason: Option<String>,
}

/// Approve a pending deployment
/// POST /api/deployments/:id/approve
pub async fn approve_deployment(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(deployment_id): Path<String>,
) -> Result<Json<Deployment>, ApiError> {
    if let Err(e) = validate_uuid(&deployment_id, "deployment_id") {
        return Err(ApiError::validation_field("deployment_id", e));
    }

    // Only admins can approve deployments
    if user.role != "admin" {
        return Err(ApiError::forbidden("Only admins can approve deployments"));
    }

    // Get the deployment
    let deployment = sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = ?")
        .bind(&deployment_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Deployment not found"))?;

    // Must be in pending approval state
    if deployment.approval_status.as_deref() != Some("pending") {
        return Err(ApiError::bad_request("Deployment is not pending approval"));
    }

    // Must still have status 'pending'
    if deployment.status != "pending" {
        return Err(ApiError::bad_request(
            "Deployment is no longer in pending state",
        ));
    }

    let now = chrono::Utc::now().to_rfc3339();

    // Use NULL for approved_by when using the admin API token (synthetic "system" user
    // has no row in the users table and would violate the FK constraint).
    let approver_id: Option<&str> = if user.id == "system" {
        None
    } else {
        Some(&user.id)
    };

    // Update approval status
    sqlx::query(
        "UPDATE deployments SET approval_status = 'approved', approved_by = ?, approved_at = ? WHERE id = ?",
    )
    .bind(approver_id)
    .bind(&now)
    .bind(&deployment_id)
    .execute(&state.db)
    .await?;

    // Get the app so we can queue the deployment
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&deployment.app_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Queue the deployment job now that it is approved
    if let Err(e) = state.deploy_tx.send((deployment_id.clone(), app)).await {
        tracing::error!("Failed to queue approved deployment: {}", e);
        return Err(ApiError::internal("Failed to queue deployment job"));
    }

    let updated = sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = ?")
        .bind(&deployment_id)
        .fetch_one(&state.db)
        .await?;

    tracing::info!(
        deployment_id = %deployment_id,
        approved_by = %user.id,
        "Deployment approved and queued"
    );

    Ok(Json(updated))
}

/// Reject a pending deployment
/// POST /api/deployments/:id/reject
pub async fn reject_deployment(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(deployment_id): Path<String>,
    body: Option<Json<RejectDeployRequest>>,
) -> Result<Json<Deployment>, ApiError> {
    if let Err(e) = validate_uuid(&deployment_id, "deployment_id") {
        return Err(ApiError::validation_field("deployment_id", e));
    }

    // Only admins can reject deployments
    if user.role != "admin" {
        return Err(ApiError::forbidden("Only admins can reject deployments"));
    }

    // Get the deployment
    let deployment = sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = ?")
        .bind(&deployment_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Deployment not found"))?;

    // Must be in pending approval state
    if deployment.approval_status.as_deref() != Some("pending") {
        return Err(ApiError::bad_request("Deployment is not pending approval"));
    }

    let reason = body
        .and_then(|b| b.0.reason)
        .unwrap_or_else(|| "No reason provided".to_string());

    let now = chrono::Utc::now().to_rfc3339();

    let approver_id: Option<&str> = if user.id == "system" {
        None
    } else {
        Some(&user.id)
    };

    sqlx::query(
        r#"UPDATE deployments
           SET approval_status = 'rejected',
               approved_by = ?,
               approved_at = ?,
               rejection_reason = ?,
               status = 'failed',
               error_message = ?,
               finished_at = ?
           WHERE id = ?"#,
    )
    .bind(approver_id)
    .bind(&now)
    .bind(&reason)
    .bind(format!("Deployment rejected: {}", reason))
    .bind(&now)
    .bind(&deployment_id)
    .execute(&state.db)
    .await?;

    let updated = sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = ?")
        .bind(&deployment_id)
        .fetch_one(&state.db)
        .await?;

    tracing::info!(
        deployment_id = %deployment_id,
        rejected_by = %user.id,
        reason = %reason,
        "Deployment rejected"
    );

    Ok(Json(updated))
}

/// List pending-approval deployments for an app
/// GET /api/apps/:id/deployments/pending
pub async fn list_pending_deployments(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<Deployment>>, ApiError> {
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    let app_exists: Option<(String,)> = sqlx::query_as("SELECT id FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await?;

    if app_exists.is_none() {
        return Err(ApiError::not_found("App not found"));
    }

    let deployments = sqlx::query_as::<_, Deployment>(
        "SELECT * FROM deployments WHERE app_id = ? AND approval_status = 'pending' ORDER BY started_at DESC",
    )
    .bind(&app_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(deployments))
}
