//! Gitea webhook handler (push events and pull request preview deployments).

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use super::{
    collect_changed_files, handle_generic_preview_cleanup, incr_webhooks, log_wh_event,
    should_deploy_for_changed_files, verify_gitea_signature, ChangedFiles,
};
use crate::crypto;
use crate::db::App;
use crate::engine::preview::{
    find_or_create_preview, run_preview_deployment, PreviewDeploymentInfo,
};
use crate::AppState;

// ---------------------------------------------------------------------------
// Gitea payload types
// ---------------------------------------------------------------------------

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
    #[serde(default)]
    pub added: Vec<String>,
    #[serde(default)]
    pub modified: Vec<String>,
    #[serde(default)]
    pub removed: Vec<String>,
}

impl ChangedFiles for &GiteaCommit {
    fn added_files(&self) -> &[String] { &self.added }
    fn modified_files(&self) -> &[String] { &self.modified }
    fn removed_files(&self) -> &[String] { &self.removed }
}

#[derive(Debug, Deserialize)]
pub struct GiteaPullRequestEvent {
    pub action: String,
    pub number: i64,
    pub pull_request: GiteaPullRequest,
    pub repository: GiteaRepository,
}

#[derive(Debug, Deserialize)]
pub struct GiteaPullRequest {
    pub title: String,
    pub html_url: String,
    pub head: GiteaPullRequestRef,
    pub base: GiteaPullRequestRef,
    pub user: GiteaUser,
}

#[derive(Debug, Deserialize)]
pub struct GiteaPullRequestRef {
    #[serde(rename = "ref")]
    pub branch: String,
    pub sha: String,
}

#[derive(Debug, Deserialize)]
pub struct GiteaUser {
    pub login: String,
}

#[derive(Debug, Deserialize)]
struct GiteaEventProbe {
    action: Option<String>,
    pull_request: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn gitea_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, StatusCode> {
    incr_webhooks("gitea");

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

    let event_type = headers
        .get("X-Gitea-Event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if event_type == "pull_request" {
        return handle_gitea_pull_request(state, &body).await;
    }

    if let Ok(probe) = serde_json::from_slice::<GiteaEventProbe>(&body) {
        if probe.pull_request.is_some() && probe.action.is_some() {
            return handle_gitea_pull_request(state, &body).await;
        }
    }

    handle_gitea_push(state, &body).await
}

async fn handle_gitea_push(
    state: Arc<AppState>,
    body: &[u8],
) -> Result<StatusCode, StatusCode> {
    let payload: GiteaPushEvent = serde_json::from_slice(body).map_err(|e| {
        tracing::error!("Failed to parse Gitea push webhook payload: {}", e);
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

    let changed_files = collect_changed_files(payload.commits.iter());
    let first_commit_sha = payload.commits.first().map(|c| c.id.as_str());
    let apps_count = apps.len() as i64;

    for app in apps {
        if !should_deploy_for_changed_files(&app, &changed_files) {
            tracing::info!(app = %app.name, "Skipping deployment: no watched files changed");
            continue;
        }

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

    log_wh_event(
        &state.db,
        "gitea",
        "push",
        Some(&payload.repository.full_name),
        Some(branch),
        first_commit_sha,
        body.len(),
        apps_count,
        if apps_count > 0 { "processed" } else { "ignored" },
        None,
    )
    .await;

    Ok(StatusCode::OK)
}

async fn handle_gitea_pull_request(
    state: Arc<AppState>,
    body: &[u8],
) -> Result<StatusCode, StatusCode> {
    let payload: GiteaPullRequestEvent = serde_json::from_slice(body).map_err(|e| {
        tracing::error!("Failed to parse Gitea PR webhook payload: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    tracing::info!(
        "Gitea PR webhook received: {} PR #{} action={}",
        payload.repository.full_name,
        payload.number,
        payload.action
    );

    let apps = sqlx::query_as::<_, App>(
        "SELECT * FROM apps WHERE (git_url LIKE ? OR git_url LIKE ?) AND preview_enabled = 1",
    )
    .bind(format!("%{}", payload.repository.clone_url))
    .bind(format!("%{}", payload.repository.ssh_url))
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if apps.is_empty() {
        return Ok(StatusCode::OK);
    }

    match payload.action.as_str() {
        "opened" | "synchronized" | "synchronize" | "reopened" => {
            for app in apps {
                handle_gitea_pr_deploy(&state, &app, &payload).await?;
            }
        }
        "closed" => {
            for app in apps {
                handle_generic_preview_cleanup(&state, &app, payload.number).await?;
            }
        }
        _ => {
            tracing::debug!("Ignoring Gitea PR action: {}", payload.action);
        }
    }

    Ok(StatusCode::OK)
}

async fn handle_gitea_pr_deploy(
    state: &Arc<AppState>,
    app: &App,
    payload: &GiteaPullRequestEvent,
) -> Result<(), StatusCode> {
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
        commit_message: None,
        provider_type: "gitea".to_string(),
        repo_full_name: payload.repository.full_name.clone(),
    };

    let preview = find_or_create_preview(&state.db, app, &info, &base_domain)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create preview deployment for Gitea PR: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!(
        preview_id = %preview.id,
        app = %app.name,
        pr = payload.number,
        domain = %preview.preview_domain,
        "Starting Gitea PR preview deployment"
    );

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
                "Gitea PR preview deployment failed"
            );
        }
    });

    Ok(())
}
