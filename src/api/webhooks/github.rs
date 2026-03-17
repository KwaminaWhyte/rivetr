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
    collect_changed_files, handle_generic_preview_cleanup, incr_webhooks, record_delivery_id,
    should_deploy_for_changed_files, update_wh_event, verify_github_signature, ChangedFiles,
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
    pub after: String,
    /// True when a branch is deleted (GitHub sets `after` to all zeros).
    #[serde(default)]
    pub deleted: bool,
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
    fn added_files(&self) -> &[String] {
        &self.added
    }
    fn modified_files(&self) -> &[String] {
        &self.modified
    }
    fn removed_files(&self) -> &[String] {
        &self.removed
    }
}

impl ChangedFiles for &GitHubCommitDetail {
    fn added_files(&self) -> &[String] {
        &self.added
    }
    fn modified_files(&self) -> &[String] {
        &self.modified
    }
    fn removed_files(&self) -> &[String] {
        &self.removed
    }
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

    // Extract delivery ID for idempotency — GitHub sends X-GitHub-Delivery with every request.
    let delivery_id = headers
        .get("X-GitHub-Delivery")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned());

    // Atomic deduplication: INSERT OR IGNORE the delivery ID right away.
    // record_delivery_id returns true only when rows_affected == 1 (this request won).
    // Any concurrent or retry request for the same ID gets rows_affected == 0 (UNIQUE
    // constraint) and returns false — so we bail before doing any deployment work.
    // This closes the race window that existed in the old read-then-write approach.
    if let Some(ref did) = delivery_id {
        if !record_delivery_id(&state.db, "github", did).await {
            tracing::info!("GitHub webhook duplicate delivery {} — skipping", did);
            return Ok(StatusCode::OK);
        }
    }

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
        "pull_request" => handle_github_pull_request(state, &body, delivery_id.as_deref()).await,
        "push" => handle_github_push(state, &body, delivery_id.as_deref()).await,
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
    delivery_id: Option<&str>,
) -> Result<StatusCode, StatusCode> {
    let payload: GitHubPushEvent = serde_json::from_slice(body).map_err(|e| {
        tracing::error!("Failed to parse GitHub push webhook payload: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    // Ignore branch/tag deletion events — `deleted: true` or an all-zeros `after` SHA both
    // signal that a ref was removed, not that new code was pushed.
    if payload.deleted || payload.after.chars().all(|c| c == '0') {
        tracing::debug!(
            "GitHub push webhook is a deletion event for {} — ignoring",
            payload.git_ref
        );
        return Ok(StatusCode::OK);
    }

    // Only handle branch pushes — ignore tag refs (refs/tags/...).
    let branch = match payload.git_ref.strip_prefix("refs/heads/") {
        Some(b) => b,
        None => {
            tracing::debug!(
                "GitHub push webhook ref '{}' is not a branch — ignoring",
                payload.git_ref
            );
            return Ok(StatusCode::OK);
        }
    };

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
        update_wh_event(
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
            delivery_id,
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

        let commit_sha = payload.head_commit.as_ref().map(|c| c.id.clone());
        let commit_message = payload.head_commit.as_ref().map(|c| c.message.clone());

        // Atomically check for an active deployment and insert a new one if none exists.
        // Using BEGIN IMMEDIATE acquires SQLite's write lock upfront, so two concurrent
        // webhook requests for the same push (e.g. repo webhook + GitHub App) can't both
        // read 0 and both insert — the second waits for the first to commit, then sees 1.
        let deployment_id = {
            let mut conn = state
                .db
                .acquire()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            sqlx::query("BEGIN IMMEDIATE")
                .execute(&mut *conn)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // Check for an already-active deployment for this app + commit.
            // Use NOT IN terminal statuses so we catch deployments in 'building' and
            // 'starting' states too — the engine transitions out of 'pending' almost
            // immediately, so checking only ('pending', 'running') misses the window.
            let active: i64 = if let Some(ref sha) = commit_sha {
                sqlx::query_scalar(
                    "SELECT COUNT(*) FROM deployments \
                     WHERE app_id = ? AND commit_sha = ? \
                     AND status NOT IN ('succeeded', 'failed', 'cancelled', 'replaced')",
                )
                .bind(&app.id)
                .bind(sha)
                .fetch_one(&mut *conn)
                .await
                .unwrap_or(0)
            } else {
                0
            };

            if active > 0 {
                let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
                tracing::info!(
                    app = %app.name,
                    "Skipping duplicate webhook — active deployment for same commit already exists"
                );
                continue;
            }

            let id = Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();

            sqlx::query(
                "INSERT INTO deployments \
                 (id, app_id, commit_sha, commit_message, status, started_at) \
                 VALUES (?, ?, ?, ?, 'pending', ?)",
            )
            .bind(&id)
            .bind(&app.id)
            .bind(&commit_sha)
            .bind(&commit_message)
            .bind(&now)
            .execute(&mut *conn)
            .await
            .map_err(|_| {
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            sqlx::query("COMMIT")
                .execute(&mut *conn)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            id
        };

        if let Err(e) = state
            .deploy_tx
            .send((deployment_id.clone(), app.clone()))
            .await
        {
            tracing::error!("Failed to queue deployment: {}", e);
        }

        tracing::info!("Queued deployment {} for app {}", deployment_id, app.name);
    }

    update_wh_event(
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
        delivery_id,
    )
    .await;

    Ok(StatusCode::OK)
}

async fn handle_github_pull_request(
    state: Arc<AppState>,
    body: &[u8],
    _delivery_id: Option<&str>,
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
                        post_preview_comment(&db, &updated, "failed", encryption_key.as_ref()).await
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
