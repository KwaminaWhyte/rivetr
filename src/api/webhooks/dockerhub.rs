//! DockerHub webhook handler — deploy apps when an image is pushed.

use axum::{extract::State, http::{HeaderMap, StatusCode}, body::Bytes};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;
use crate::{db::App, AppState};
use super::{incr_webhooks, log_wh_event};

#[derive(Debug, Deserialize)]
pub struct DockerHubWebhookPayload {
    pub push_data: DockerHubPushData,
    pub repository: DockerHubRepository,
    pub callback_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DockerHubPushData {
    pub tag: String,
    pub pusher: String,
}

#[derive(Debug, Deserialize)]
pub struct DockerHubRepository {
    pub repo_name: String,     // e.g. "myuser/myimage"
    #[allow(dead_code)]
    pub name: String,          // image name
    #[allow(dead_code)]
    pub namespace: String,     // user/org
    #[allow(dead_code)]
    pub is_private: bool,
}

pub async fn dockerhub_webhook(
    State(state): State<Arc<AppState>>,
    _headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, StatusCode> {
    incr_webhooks("dockerhub");

    // DockerHub doesn't sign webhooks — just parse the payload
    let payload: DockerHubWebhookPayload = serde_json::from_slice(&body).map_err(|e| {
        tracing::error!("Failed to parse DockerHub webhook payload: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    let image_name = &payload.repository.repo_name; // "user/image"
    let tag = &payload.push_data.tag;

    tracing::info!(
        "DockerHub webhook: {}:{} pushed by {}",
        image_name,
        tag,
        payload.push_data.pusher
    );

    // Find apps that use this Docker image as their source.
    // Match on docker_image field: either exact "image:tag" or "image:latest" when tag is "latest".
    let apps = sqlx::query_as::<_, App>(
        "SELECT * FROM apps WHERE (docker_image LIKE ? OR docker_image LIKE ?)"
    )
    .bind(format!("{}:{}", image_name, tag))
    .bind(format!("{}:latest", image_name)) // also match :latest when 'latest' tag pushed
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if apps.is_empty() {
        tracing::debug!("No apps found for DockerHub image {}:{}", image_name, tag);
        log_wh_event(
            &state.db,
            "dockerhub",
            "push",
            Some(image_name),
            None,
            Some(tag.as_str()),
            body.len(),
            0,
            "ignored",
            None,
        )
        .await;
        return Ok(StatusCode::OK);
    }

    let apps_count = apps.len() as i64;
    for app in apps {
        let deployment_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO deployments (id, app_id, commit_sha, commit_message, status, started_at) VALUES (?, ?, ?, ?, 'pending', ?)"
        )
        .bind(&deployment_id)
        .bind(&app.id)
        .bind(format!("{}:{}", image_name, tag))
        .bind(format!("DockerHub push: {}:{}", image_name, tag))
        .bind(&now)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Err(e) = state.deploy_tx.send((deployment_id.clone(), app.clone())).await {
            tracing::error!("Failed to queue deployment: {}", e);
        }

        tracing::info!(
            "Queued deployment {} for app {} on DockerHub push",
            deployment_id,
            app.name
        );
    }

    log_wh_event(
        &state.db,
        "dockerhub",
        "push",
        Some(image_name),
        None,
        Some(tag.as_str()),
        body.len(),
        apps_count,
        "processed",
        None,
    )
    .await;

    // DockerHub expects a callback if callback_url is provided
    if let Some(callback_url) = &payload.callback_url {
        let client = reqwest::Client::new();
        let _ = client
            .post(callback_url)
            .json(&serde_json::json!({"state": "success", "description": "Deployment queued"}))
            .send()
            .await;
    }

    Ok(StatusCode::OK)
}
