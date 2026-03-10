//! GitHub webhook handler (push events and pull request preview deployments).

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
    should_deploy_for_changed_files, verify_github_signature, ChangedFiles,
};
use crate::crypto;
use crate::db::{App, PreviewDeployment};
use crate::engine::preview::{
    find_or_create_preview, post_preview_comment, run_preview_deployment, PreviewDeploymentInfo,
};
use crate::AppState;

// ---------------------------------------------------------------------------
// GitHub payload types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct GitHubPushEvent {
    #[serde(rename = "ref")]
    pub git_ref: String,
    #[allow(dead_code)]
    pub after: String,
    pub repository: GitHubRepository,
    pub head_commit: Option<GitHubHeadCommit>,
    #[serde(default)]
    pub commits: Vec<GitHubCommitDetail>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubRepository {
    pub clone_url: String,
    pub ssh_url: String,
    pub full_name: String,
}

#[derive(Debug, Deserialize)]
pub struct GitHubHeadCommit {
    pub id: String,
    pub message: String,
}

/// Detailed commit info including file changes (used in the `commits` array)
#[derive(Debug, Deserialize)]
pub struct GitHubCommitDetail {
    #[allow(dead_code)]
    pub id: String,
    #[serde(default)]
    pub added: Vec<String>,
    #[serde(default)]
    pub modified: Vec<String>,
    #[serde(default)]
    pub removed: Vec<String>,
}

impl ChangedFiles for GitHubCommitDetail {
    fn added_files(&self) -> &[String] { &self.added }
    fn modified_files(&self) -> &[String] { &self.modified }
    fn removed_files(&self) -> &[String] { &self.removed }
}

impl ChangedFiles for &GitHubCommitDetail {
    fn added_files(&self) -> &[String] { &self.added }
    fn modified_files(&self) -> &[String] { &self.modified }
    fn removed_files(&self) -> &[String] { &self.removed }
}

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

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn github_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, StatusCode> {
    incr_webhooks("github");

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

async fn handle_github_push(
    state: Arc<AppState>,
    body: &[u8],
) -> Result<StatusCode, StatusCode> {
    let payload: GitHubPushEvent = serde_json::from_slice(body).map_err(|e| {
        tracing::error!("Failed to parse GitHub push webhook payload: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    let branch = payload
        .git_ref
        .strip_prefix("refs/heads/")
        .unwrap_or(&payload.git_ref);

    tracing::info!(
        "GitHub push webhook received: {} branch {}",
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

    if apps.is_empty() {
        tracing::warn!("No matching app found for push webhook");
        log_wh_event(
            &state.db,
            "github",
            "push",
            Some(&payload.repository.full_name),
            Some(branch),
            payload.head_commit.as_ref().map(|c| c.id.as_str()),
            body.len(),
            0,
            "ignored",
            None,
        )
        .await;
        return Ok(StatusCode::OK);
    }

    let changed_files = collect_changed_files(payload.commits.iter());
    let apps_count = apps.len() as i64;

    for app in apps {
        if !should_deploy_for_changed_files(&app, &changed_files) {
            tracing::info!(
                app = %app.name,
                "Skipping deployment: no watched files changed"
            );
            continue;
        }

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

        if let Err(e) = state
            .deploy_tx
            .send((deployment_id.clone(), app.clone()))
            .await
        {
            tracing::error!("Failed to queue deployment: {}", e);
        }

        tracing::info!("Queued deployment {} for app {}", deployment_id, app.name);
    }

    log_wh_event(
        &state.db,
        "github",
        "push",
        Some(&payload.repository.full_name),
        Some(branch),
        payload.head_commit.as_ref().map(|c| c.id.as_str()),
        body.len(),
        apps_count,
        "processed",
        None,
    )
    .await;

    Ok(StatusCode::OK)
}

async fn handle_github_pull_request(
    state: Arc<AppState>,
    body: &[u8],
) -> Result<StatusCode, StatusCode> {
    let payload: GitHubPullRequestEvent = serde_json::from_slice(body).map_err(|e| {
        tracing::error!("Failed to parse GitHub PR webhook payload: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    tracing::info!(
        "GitHub PR webhook received: {} PR #{} action={}",
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
        tracing::debug!(
            "No apps with preview_enabled found for PR webhook: {}",
            payload.repository.full_name
        );
        return Ok(StatusCode::OK);
    }

    match payload.action.as_str() {
        "opened" | "synchronize" | "reopened" => {
            for app in apps {
                handle_preview_deploy(&state, &app, &payload).await?;
            }
        }
        "closed" => {
            for app in &apps {
                handle_generic_preview_cleanup(&state, app, payload.number).await?;
            }
            // Also post closed comment
            for app in apps {
                handle_preview_closed_comment(&state, &app, &payload).await?;
            }
        }
        _ => {
            tracing::debug!("Ignoring PR action: {}", payload.action);
        }
    }

    Ok(StatusCode::OK)
}

async fn handle_preview_deploy(
    state: &Arc<AppState>,
    app: &App,
    payload: &GitHubPullRequestEvent,
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
        provider_type: "github".to_string(),
        repo_full_name: payload.repository.full_name.clone(),
    };

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
        let deploy_result = run_preview_deployment(
            &db,
            runtime,
            routes,
            &preview,
            &app_clone,
            encryption_key.as_ref(),
        )
        .await;

        match deploy_result {
            Ok(()) => {
                let updated_preview: Option<PreviewDeployment> =
                    sqlx::query_as("SELECT * FROM preview_deployments WHERE id = ?")
                        .bind(&preview.id)
                        .fetch_optional(&db)
                        .await
                        .unwrap_or(None);

                if let Some(updated) = updated_preview {
                    if let Err(e) =
                        post_preview_comment(&db, &updated, "running", encryption_key.as_ref())
                            .await
                    {
                        tracing::warn!(
                            preview_id = %preview.id,
                            error = %e,
                            "Failed to post preview PR comment"
                        );
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    preview_id = %preview.id,
                    error = %e,
                    "Preview deployment failed"
                );

                let updated_preview: Option<PreviewDeployment> =
                    sqlx::query_as("SELECT * FROM preview_deployments WHERE id = ?")
                        .bind(&preview.id)
                        .fetch_optional(&db)
                        .await
                        .unwrap_or(None);

                if let Some(updated) = updated_preview {
                    if let Err(comment_err) =
                        post_preview_comment(&db, &updated, "failed", encryption_key.as_ref())
                            .await
                    {
                        tracing::warn!(
                            preview_id = %preview.id,
                            error = %comment_err,
                            "Failed to post failure PR comment"
                        );
                    }
                }
            }
        }
    });

    Ok(())
}

async fn handle_preview_closed_comment(
    state: &Arc<AppState>,
    app: &App,
    payload: &GitHubPullRequestEvent,
) -> Result<(), StatusCode> {
    let preview: Option<PreviewDeployment> =
        sqlx::query_as("SELECT * FROM preview_deployments WHERE app_id = ? AND pr_number = ?")
            .bind(&app.id)
            .bind(payload.number)
            .fetch_optional(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(preview) = preview {
        let db = state.db.clone();
        let encryption_key = state
            .config
            .auth
            .encryption_key
            .as_ref()
            .map(|secret| crypto::derive_key(secret));

        tokio::spawn(async move {
            if let Err(e) =
                post_preview_comment(&db, &preview, "closed", encryption_key.as_ref()).await
            {
                tracing::warn!(
                    preview_id = %preview.id,
                    error = %e,
                    "Failed to post cleanup PR comment"
                );
            }
        });
    }

    Ok(())
}
