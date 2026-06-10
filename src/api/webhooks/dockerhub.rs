//! DockerHub webhook handler — deploy apps when an image is pushed.

use super::{incr_webhooks, log_wh_event};
use crate::{db::App, AppState};
use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use std::sync::Arc;
use subtle::ConstantTimeEq;
use uuid::Uuid;

/// DockerHub sends no signature header, so auth is a shared token in the URL:
/// configure the webhook as `/webhooks/dockerhub?token=<webhooks.dockerhub_secret>`.
#[derive(Debug, Deserialize)]
pub struct DockerHubAuthQuery {
    pub token: Option<String>,
}

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
    pub repo_name: String, // e.g. "myuser/myimage"
    #[allow(dead_code)]
    pub name: String, // image name
    #[allow(dead_code)]
    pub namespace: String, // user/org
    #[allow(dead_code)]
    pub is_private: bool,
}

pub async fn dockerhub_webhook(
    State(state): State<Arc<AppState>>,
    Query(auth): Query<DockerHubAuthQuery>,
    _headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, StatusCode> {
    incr_webhooks("dockerhub");

    // SEC-H1: fail closed. DockerHub can't sign requests, so require a configured
    // shared token supplied as ?token=... and compare it in constant time.
    let expected = state.config.webhooks.dockerhub_secret.as_ref().ok_or_else(|| {
        tracing::error!(
            "DockerHub webhook rejected: webhooks.dockerhub_secret is not configured (fail-closed)."
        );
        StatusCode::UNAUTHORIZED
    })?;
    let provided = auth.token.as_deref().unwrap_or("");
    let token_ok = provided.len() == expected.len()
        && provided.as_bytes().ct_eq(expected.as_bytes()).into();
    if !token_ok {
        tracing::warn!("DockerHub webhook rejected: missing/invalid ?token=");
        return Err(StatusCode::UNAUTHORIZED);
    }

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
        "SELECT * FROM apps WHERE (docker_image LIKE ? OR docker_image LIKE ?)",
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

        if let Err(e) = state
            .deploy_tx
            .send((deployment_id.clone(), app.clone()))
            .await
        {
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
        None,
    )
    .await;

    // DockerHub expects a callback if callback_url is provided.
    // SEC-H1: the callback_url is attacker-controlled — validate it points at a
    // public address (no internal/metadata SSRF) and never follow redirects.
    if let Some(callback_url) = &payload.callback_url {
        match crate::api::ssrf::validate_external_url(callback_url).await {
            Ok(()) => {
                if let Ok(client) = reqwest::Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .build()
                {
                    let _ = client
                        .post(callback_url)
                        .json(&serde_json::json!({"state": "success", "description": "Deployment queued"}))
                        .send()
                        .await;
                }
            }
            Err(_) => {
                tracing::warn!(
                    "Rejected DockerHub callback_url (failed SSRF validation): {}",
                    callback_url
                );
            }
        }
    }

    Ok(StatusCode::OK)
}
