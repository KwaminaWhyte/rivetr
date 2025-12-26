use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{App, Deployment, DeploymentLog};
use crate::AppState;

pub async fn trigger_deploy(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<(StatusCode, Json<Deployment>), StatusCode> {
    // Check if app exists
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

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
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Queue the deployment job
    if let Err(e) = state.deploy_tx.send((deployment_id.clone(), app)).await {
        tracing::error!("Failed to queue deployment: {}", e);
    }

    let deployment = sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = ?")
        .bind(&deployment_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::ACCEPTED, Json(deployment)))
}

pub async fn list_deployments(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<Deployment>>, StatusCode> {
    let deployments = sqlx::query_as::<_, Deployment>(
        "SELECT * FROM deployments WHERE app_id = ? ORDER BY started_at DESC LIMIT 50",
    )
    .bind(&app_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(deployments))
}

pub async fn get_deployment(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Deployment>, StatusCode> {
    let deployment = sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(deployment))
}

pub async fn get_logs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<DeploymentLog>>, StatusCode> {
    let logs = sqlx::query_as::<_, DeploymentLog>(
        "SELECT * FROM deployment_logs WHERE deployment_id = ? ORDER BY id ASC",
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(logs))
}
