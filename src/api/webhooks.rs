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

use crate::crypto;
use crate::db::{App, PreviewDeployment};
use crate::engine::preview::{
    cleanup_preview, find_or_create_preview, run_preview_deployment, PreviewDeploymentInfo,
};
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

/// GitHub Pull Request event payload
#[derive(Debug, Deserialize)]
pub struct GitHubPullRequestEvent {
    pub action: String,
    pub number: i64,
    pub pull_request: GitHubPullRequest,
    pub repository: GitHubRepository,
}

#[derive(Debug, Deserialize)]
pub struct GitHubPullRequest {
    pub title: String,
    pub html_url: String,
    pub head: GitHubPullRequestRef,
    pub base: GitHubPullRequestRef,
    pub user: GitHubUser,
}

#[derive(Debug, Deserialize)]
pub struct GitHubPullRequestRef {
    #[serde(rename = "ref")]
    pub branch: String,
    pub sha: String,
}

#[derive(Debug, Deserialize)]
pub struct GitHubUser {
    pub login: String,
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

    // Check the event type from X-GitHub-Event header
    let event_type = headers
        .get("X-GitHub-Event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("push");

    match event_type {
        "pull_request" => handle_github_pull_request(state, &body).await,
        "push" => handle_github_push(state, &body).await,
        "ping" => {
            tracing::info!("GitHub ping received");
            Ok(StatusCode::OK)
        }
        _ => {
            tracing::debug!("Ignoring GitHub event type: {}", event_type);
            Ok(StatusCode::OK)
        }
    }
}

/// Handle GitHub push events (regular deployments)
async fn handle_github_push(
    state: Arc<AppState>,
    body: &[u8],
) -> Result<StatusCode, StatusCode> {
    // Parse the JSON payload
    let payload: GitHubPushEvent = serde_json::from_slice(body)
        .map_err(|e| {
            tracing::error!("Failed to parse GitHub push webhook payload: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    // Extract branch from ref (refs/heads/main -> main)
    let branch = payload
        .git_ref
        .strip_prefix("refs/heads/")
        .unwrap_or(&payload.git_ref);

    tracing::info!(
        "GitHub push webhook received: {} branch {}",
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
        tracing::warn!("No matching app found for push webhook");
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

/// Handle GitHub pull request events (preview deployments)
async fn handle_github_pull_request(
    state: Arc<AppState>,
    body: &[u8],
) -> Result<StatusCode, StatusCode> {
    // Parse the JSON payload
    let payload: GitHubPullRequestEvent = serde_json::from_slice(body)
        .map_err(|e| {
            tracing::error!("Failed to parse GitHub PR webhook payload: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    tracing::info!(
        "GitHub PR webhook received: {} PR #{} action={}",
        payload.repository.full_name,
        payload.number,
        payload.action
    );

    // Find apps that match this repository and have preview_enabled
    let apps = sqlx::query_as::<_, App>(
        "SELECT * FROM apps WHERE (git_url LIKE ? OR git_url LIKE ?) AND preview_enabled = 1",
    )
    .bind(format!("%{}", payload.repository.clone_url))
    .bind(format!("%{}", payload.repository.ssh_url))
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if apps.is_empty() {
        tracing::debug!(
            "No apps with preview_enabled found for PR webhook: {}",
            payload.repository.full_name
        );
        return Ok(StatusCode::OK);
    }

    match payload.action.as_str() {
        "opened" | "synchronize" | "reopened" => {
            // Deploy or redeploy preview
            for app in apps {
                handle_preview_deploy(&state, &app, &payload).await?;
            }
        }
        "closed" => {
            // Clean up preview deployment
            for app in apps {
                handle_preview_cleanup(&state, &app, &payload).await?;
            }
        }
        _ => {
            tracing::debug!("Ignoring PR action: {}", payload.action);
        }
    }

    Ok(StatusCode::OK)
}

/// Deploy or redeploy a preview environment for a PR
async fn handle_preview_deploy(
    state: &Arc<AppState>,
    app: &App,
    payload: &GitHubPullRequestEvent,
) -> Result<(), StatusCode> {
    // Get preview domain base from config, default to "preview.localhost"
    let base_domain = state
        .config
        .proxy
        .preview_domain
        .clone()
        .unwrap_or_else(|| "preview.localhost".to_string());

    let info = PreviewDeploymentInfo {
        app_id: app.id.clone(),
        pr_number: payload.number,
        pr_title: Some(payload.pull_request.title.clone()),
        pr_source_branch: payload.pull_request.head.branch.clone(),
        pr_target_branch: payload.pull_request.base.branch.clone(),
        pr_author: Some(payload.pull_request.user.login.clone()),
        pr_url: Some(payload.pull_request.html_url.clone()),
        commit_sha: Some(payload.pull_request.head.sha.clone()),
        commit_message: None, // Not available in PR event
        provider_type: "github".to_string(),
        repo_full_name: payload.repository.full_name.clone(),
    };

    // Find or create the preview deployment
    let preview = find_or_create_preview(&state.db, app, &info, &base_domain)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create preview deployment: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!(
        preview_id = %preview.id,
        app = %app.name,
        pr = payload.number,
        domain = %preview.preview_domain,
        "Starting preview deployment"
    );

    // Run the preview deployment in a background task
    let db = state.db.clone();
    let runtime = state.runtime.clone();
    let routes = state.routes.clone();
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
            &preview,
            &app_clone,
            encryption_key.as_ref(),
        )
        .await
        {
            tracing::error!(
                preview_id = %preview.id,
                error = %e,
                "Preview deployment failed"
            );
        }
    });

    Ok(())
}

/// Clean up a preview environment when a PR is closed
async fn handle_preview_cleanup(
    state: &Arc<AppState>,
    app: &App,
    payload: &GitHubPullRequestEvent,
) -> Result<(), StatusCode> {
    // Find the preview deployment
    let preview: Option<PreviewDeployment> = sqlx::query_as(
        "SELECT * FROM preview_deployments WHERE app_id = ? AND pr_number = ?",
    )
    .bind(&app.id)
    .bind(payload.number)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(preview) = preview {
        tracing::info!(
            preview_id = %preview.id,
            app = %app.name,
            pr = payload.number,
            "Cleaning up preview deployment"
        );

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
    } else {
        tracing::debug!(
            app = %app.name,
            pr = payload.number,
            "No preview deployment found for cleanup"
        );
    }

    Ok(())
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
