use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::App;
use crate::AppState;

type HmacSha256 = Hmac<Sha256>;

/// Verify GitHub webhook signature (X-Hub-Signature-256 header)
fn verify_github_signature(secret: &str, signature_header: &str, payload: &[u8]) -> bool {
    // Signature format: sha256=<hex>
    let signature = match signature_header.strip_prefix("sha256=") {
        Some(sig) => sig,
        None => return false,
    };

    let expected = match hex::decode(signature) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };

    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(payload);

    // Use constant-time comparison
    mac.verify_slice(&expected).is_ok()
}

/// Verify Gitea webhook signature (X-Gitea-Signature header) - uses HMAC-SHA256
fn verify_gitea_signature(secret: &str, signature_header: &str, payload: &[u8]) -> bool {
    let expected = match hex::decode(signature_header) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };

    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(payload);

    mac.verify_slice(&expected).is_ok()
}

#[derive(Debug, Deserialize)]
pub struct GitHubPushEvent {
    #[serde(rename = "ref")]
    pub git_ref: String,
    #[allow(dead_code)]
    pub after: String,
    pub repository: GitHubRepository,
    pub head_commit: Option<GitHubCommit>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubRepository {
    pub clone_url: String,
    pub ssh_url: String,
    pub full_name: String,
}

#[derive(Debug, Deserialize)]
pub struct GitHubCommit {
    pub id: String,
    pub message: String,
}

pub async fn github_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, StatusCode> {
    // Verify signature if secret is configured
    if let Some(ref secret) = state.config.webhooks.github_secret {
        let signature = headers
            .get("X-Hub-Signature-256")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                tracing::warn!("GitHub webhook missing X-Hub-Signature-256 header");
                StatusCode::UNAUTHORIZED
            })?;

        if !verify_github_signature(secret, signature, &body) {
            tracing::warn!("GitHub webhook signature verification failed");
            return Err(StatusCode::UNAUTHORIZED);
        }
        tracing::debug!("GitHub webhook signature verified");
    }

    // Parse the JSON payload
    let payload: GitHubPushEvent = serde_json::from_slice(&body)
        .map_err(|e| {
            tracing::error!("Failed to parse GitHub webhook payload: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    // Extract branch from ref (refs/heads/main -> main)
    let branch = payload
        .git_ref
        .strip_prefix("refs/heads/")
        .unwrap_or(&payload.git_ref);

    tracing::info!(
        "GitHub webhook received: {} branch {}",
        payload.repository.full_name,
        branch
    );

    // Find matching app by git URL
    let apps = sqlx::query_as::<_, App>(
        "SELECT * FROM apps WHERE (git_url LIKE ? OR git_url LIKE ?) AND branch = ?",
    )
    .bind(format!("%{}", payload.repository.clone_url))
    .bind(format!("%{}", payload.repository.ssh_url))
    .bind(branch)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if apps.is_empty() {
        tracing::warn!("No matching app found for webhook");
        return Ok(StatusCode::OK);
    }

    for app in apps {
        let deployment_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let commit_sha = payload.head_commit.as_ref().map(|c| c.id.clone());
        let commit_message = payload.head_commit.as_ref().map(|c| c.message.clone());

        sqlx::query(
            r#"
            INSERT INTO deployments (id, app_id, commit_sha, commit_message, status, started_at)
            VALUES (?, ?, ?, ?, 'pending', ?)
            "#,
        )
        .bind(&deployment_id)
        .bind(&app.id)
        .bind(&commit_sha)
        .bind(&commit_message)
        .bind(&now)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Err(e) = state.deploy_tx.send((deployment_id.clone(), app.clone())).await {
            tracing::error!("Failed to queue deployment: {}", e);
        }

        tracing::info!("Queued deployment {} for app {}", deployment_id, app.name);
    }

    Ok(StatusCode::OK)
}

#[derive(Debug, Deserialize)]
pub struct GitLabPushEvent {
    #[serde(rename = "ref")]
    pub git_ref: String,
    #[allow(dead_code)]
    pub after: String,
    pub project: GitLabProject,
    pub commits: Vec<GitLabCommit>,
}

#[derive(Debug, Deserialize)]
pub struct GitLabProject {
    pub git_http_url: String,
    pub git_ssh_url: String,
    pub path_with_namespace: String,
}

#[derive(Debug, Deserialize)]
pub struct GitLabCommit {
    pub id: String,
    pub message: String,
}

pub async fn gitlab_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, StatusCode> {
    // Verify token if configured (GitLab uses X-Gitlab-Token header)
    if let Some(ref expected_token) = state.config.webhooks.gitlab_token {
        let token = headers
            .get("X-Gitlab-Token")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                tracing::warn!("GitLab webhook missing X-Gitlab-Token header");
                StatusCode::UNAUTHORIZED
            })?;

        if token != expected_token {
            tracing::warn!("GitLab webhook token verification failed");
            return Err(StatusCode::UNAUTHORIZED);
        }
        tracing::debug!("GitLab webhook token verified");
    }

    // Parse the JSON payload
    let payload: GitLabPushEvent = serde_json::from_slice(&body)
        .map_err(|e| {
            tracing::error!("Failed to parse GitLab webhook payload: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    let branch = payload
        .git_ref
        .strip_prefix("refs/heads/")
        .unwrap_or(&payload.git_ref);

    tracing::info!(
        "GitLab webhook received: {} branch {}",
        payload.project.path_with_namespace,
        branch
    );

    let apps = sqlx::query_as::<_, App>(
        "SELECT * FROM apps WHERE (git_url LIKE ? OR git_url LIKE ?) AND branch = ?",
    )
    .bind(format!("%{}", payload.project.git_http_url))
    .bind(format!("%{}", payload.project.git_ssh_url))
    .bind(branch)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for app in apps {
        let deployment_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let commit = payload.commits.first();

        sqlx::query(
            r#"
            INSERT INTO deployments (id, app_id, commit_sha, commit_message, status, started_at)
            VALUES (?, ?, ?, ?, 'pending', ?)
            "#,
        )
        .bind(&deployment_id)
        .bind(&app.id)
        .bind(commit.map(|c| c.id.clone()))
        .bind(commit.map(|c| c.message.clone()))
        .bind(&now)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Err(e) = state.deploy_tx.send((deployment_id, app.clone())).await {
            tracing::error!("Failed to queue deployment: {}", e);
        }
    }

    Ok(StatusCode::OK)
}

#[derive(Debug, Deserialize)]
pub struct GiteaPushEvent {
    #[serde(rename = "ref")]
    pub git_ref: String,
    #[allow(dead_code)]
    pub after: String,
    pub repository: GiteaRepository,
    pub commits: Vec<GiteaCommit>,
}

#[derive(Debug, Deserialize)]
pub struct GiteaRepository {
    pub clone_url: String,
    pub ssh_url: String,
    pub full_name: String,
}

#[derive(Debug, Deserialize)]
pub struct GiteaCommit {
    pub id: String,
    pub message: String,
}

pub async fn gitea_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, StatusCode> {
    // Verify signature if secret is configured (Gitea uses X-Gitea-Signature header)
    if let Some(ref secret) = state.config.webhooks.gitea_secret {
        let signature = headers
            .get("X-Gitea-Signature")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                tracing::warn!("Gitea webhook missing X-Gitea-Signature header");
                StatusCode::UNAUTHORIZED
            })?;

        if !verify_gitea_signature(secret, signature, &body) {
            tracing::warn!("Gitea webhook signature verification failed");
            return Err(StatusCode::UNAUTHORIZED);
        }
        tracing::debug!("Gitea webhook signature verified");
    }

    // Parse the JSON payload
    let payload: GiteaPushEvent = serde_json::from_slice(&body)
        .map_err(|e| {
            tracing::error!("Failed to parse Gitea webhook payload: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    let branch = payload
        .git_ref
        .strip_prefix("refs/heads/")
        .unwrap_or(&payload.git_ref);

    tracing::info!(
        "Gitea webhook received: {} branch {}",
        payload.repository.full_name,
        branch
    );

    let apps = sqlx::query_as::<_, App>(
        "SELECT * FROM apps WHERE (git_url LIKE ? OR git_url LIKE ?) AND branch = ?",
    )
    .bind(format!("%{}", payload.repository.clone_url))
    .bind(format!("%{}", payload.repository.ssh_url))
    .bind(branch)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for app in apps {
        let deployment_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let commit = payload.commits.first();

        sqlx::query(
            r#"
            INSERT INTO deployments (id, app_id, commit_sha, commit_message, status, started_at)
            VALUES (?, ?, ?, ?, 'pending', ?)
            "#,
        )
        .bind(&deployment_id)
        .bind(&app.id)
        .bind(commit.map(|c| c.id.clone()))
        .bind(commit.map(|c| c.message.clone()))
        .bind(&now)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Err(e) = state.deploy_tx.send((deployment_id, app.clone())).await {
            tracing::error!("Failed to queue deployment: {}", e);
        }
    }

    Ok(StatusCode::OK)
}
