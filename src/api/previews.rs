//! Preview deployment API endpoints.
//!
//! Provides REST API for managing PR preview deployments:
//! - List preview deployments for an app
//! - Get single preview deployment details
//! - Delete preview deployment
//! - Trigger redeploy of preview

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::crypto;
use crate::db::{App, PreviewDeployment, PreviewDeploymentResponse, PreviewDeploymentStatus};
use crate::engine::preview::{cleanup_preview, run_preview_deployment, update_preview_status};
use crate::AppState;

/// List all preview deployments for an app
pub async fn list_app_previews(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<PreviewDeploymentResponse>>, StatusCode> {
    // Verify app exists
    let _app: Option<App> = sqlx::query_as("SELECT * FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if _app.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let previews: Vec<PreviewDeployment> = sqlx::query_as(
        r#"
        SELECT * FROM preview_deployments
        WHERE app_id = ?
        ORDER BY created_at DESC
        "#,
    )
    .bind(&app_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list preview deployments: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let responses: Vec<PreviewDeploymentResponse> =
        previews.into_iter().map(|p| p.into()).collect();

    Ok(Json(responses))
}

/// Get a single preview deployment by ID
pub async fn get_preview(
    State(state): State<Arc<AppState>>,
    Path(preview_id): Path<String>,
) -> Result<Json<PreviewDeploymentResponse>, StatusCode> {
    let preview: Option<PreviewDeployment> =
        sqlx::query_as("SELECT * FROM preview_deployments WHERE id = ?")
            .bind(&preview_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Database error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    match preview {
        Some(p) => Ok(Json(p.into())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Delete a preview deployment (stops container and removes resources)
pub async fn delete_preview(
    State(state): State<Arc<AppState>>,
    Path(preview_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let preview: Option<PreviewDeployment> =
        sqlx::query_as("SELECT * FROM preview_deployments WHERE id = ?")
            .bind(&preview_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Database error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    let preview = preview.ok_or(StatusCode::NOT_FOUND)?;

    tracing::info!(
        preview_id = %preview.id,
        pr = preview.pr_number,
        "Deleting preview deployment via API"
    );

    // Clean up in background
    let db = state.db.clone();
    let runtime = state.runtime.clone();
    let routes = state.routes.clone();

    tokio::spawn(async move {
        if let Err(e) = cleanup_preview(&db, runtime, routes, &preview).await {
            tracing::error!(
                preview_id = %preview.id,
                error = %e,
                "Preview cleanup failed"
            );
        }
    });

    Ok(StatusCode::ACCEPTED)
}

/// Request body for redeploy endpoint
#[derive(Debug, Deserialize)]
pub struct RedeployRequest {
    /// Optional new commit SHA to deploy
    pub commit_sha: Option<String>,
}

/// Trigger a redeploy of an existing preview deployment
pub async fn redeploy_preview(
    State(state): State<Arc<AppState>>,
    Path(preview_id): Path<String>,
    Json(payload): Json<Option<RedeployRequest>>,
) -> Result<Json<PreviewDeploymentResponse>, StatusCode> {
    let preview: Option<PreviewDeployment> =
        sqlx::query_as("SELECT * FROM preview_deployments WHERE id = ?")
            .bind(&preview_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Database error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    let mut preview = preview.ok_or(StatusCode::NOT_FOUND)?;

    // Check if already closed
    if preview.status_enum() == PreviewDeploymentStatus::Closed {
        return Err(StatusCode::GONE);
    }

    // Get the app for this preview
    let app: App = sqlx::query_as("SELECT * FROM apps WHERE id = ?")
        .bind(&preview.app_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    tracing::info!(
        preview_id = %preview.id,
        app = %app.name,
        pr = preview.pr_number,
        "Redeploying preview via API"
    );

    // Update commit SHA if provided
    if let Some(ref req) = payload {
        if let Some(ref sha) = req.commit_sha {
            preview.commit_sha = Some(sha.clone());
            sqlx::query("UPDATE preview_deployments SET commit_sha = ? WHERE id = ?")
                .bind(sha)
                .bind(&preview.id)
                .execute(&state.db)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }

    // Reset status to pending
    update_preview_status(
        &state.db,
        &preview.id,
        PreviewDeploymentStatus::Pending,
        None,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to update preview status: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Run preview deployment in background
    let db = state.db.clone();
    let runtime = state.runtime.clone();
    let routes = state.routes.clone();
    let preview_clone = preview.clone();
    let app_clone = app.clone();
    let encryption_key = state
        .config
        .auth
        .encryption_key
        .as_ref()
        .map(|secret| crypto::derive_key(secret));

    tokio::spawn(async move {
        if let Err(e) = run_preview_deployment(
            &db,
            runtime,
            routes,
            &preview_clone,
            &app_clone,
            encryption_key.as_ref(),
        )
        .await
        {
            tracing::error!(
                preview_id = %preview_clone.id,
                error = %e,
                "Preview redeploy failed"
            );
        }
    });

    // Return updated preview status
    preview.status = "pending".to_string();
    Ok(Json(preview.into()))
}

/// List all preview deployments across all apps (for admin view)
pub async fn list_all_previews(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<PreviewDeploymentResponse>>, StatusCode> {
    let previews: Vec<PreviewDeployment> = sqlx::query_as(
        r#"
        SELECT * FROM preview_deployments
        ORDER BY created_at DESC
        LIMIT 100
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list all preview deployments: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let responses: Vec<PreviewDeploymentResponse> =
        previews.into_iter().map(|p| p.into()).collect();

    Ok(Json(responses))
}

/// Get preview deployments by status
pub async fn list_previews_by_status(
    State(state): State<Arc<AppState>>,
    Path(status): Path<String>,
) -> Result<Json<Vec<PreviewDeploymentResponse>>, StatusCode> {
    // Validate status
    let valid_statuses = [
        "pending", "cloning", "building", "starting", "running", "failed", "closed",
    ];
    if !valid_statuses.contains(&status.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let previews: Vec<PreviewDeployment> = sqlx::query_as(
        r#"
        SELECT * FROM preview_deployments
        WHERE status = ?
        ORDER BY created_at DESC
        "#,
    )
    .bind(&status)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list preview deployments by status: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let responses: Vec<PreviewDeploymentResponse> =
        previews.into_iter().map(|p| p.into()).collect();

    Ok(Json(responses))
}
