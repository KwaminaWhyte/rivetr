use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use glob::Pattern;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use std::sync::Arc;
use uuid::Uuid;

use crate::crypto;
use crate::db::{App, PreviewDeployment};
use crate::engine::preview::{
    cleanup_preview, find_or_create_preview, post_preview_comment, run_preview_deployment,
    PreviewDeploymentInfo,
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

/// Check if any of the changed files match the app's watch_paths patterns.
/// Returns true if deployment should proceed:
///   - If watch_paths is empty/null, always deploy (backward compatible)
///   - If watch_paths is configured, only deploy if at least one changed file matches
fn should_deploy_for_changed_files(app: &App, changed_files: &[String]) -> bool {
    let watch_paths = app.get_watch_paths();
    if watch_paths.is_empty() {
        return true; // No watch paths configured, always deploy
    }

    // Compile glob patterns
    let patterns: Vec<Pattern> = watch_paths
        .iter()
        .filter_map(|p| {
            // If pattern doesn't contain a glob char and ends with '/', treat as directory prefix
            let pattern_str = if p.ends_with('/') {
                format!("{}**", p)
            } else {
                p.clone()
            };
            match Pattern::new(&pattern_str) {
                Ok(pat) => Some(pat),
                Err(e) => {
                    tracing::warn!("Invalid watch_path glob pattern '{}': {}", p, e);
                    None
                }
            }
        })
        .collect();

    if patterns.is_empty() {
        return true; // All patterns were invalid, deploy to be safe
    }

    for file in changed_files {
        for pattern in &patterns {
            if pattern.matches(file) {
                tracing::debug!(
                    "Watch path match: file '{}' matches pattern '{}'",
                    file,
                    pattern
                );
                return true;
            }
        }
    }

    false
}

/// Collect all changed files from a list of commits with added/modified/removed arrays
fn collect_changed_files(commits: impl IntoIterator<Item = impl ChangedFiles>) -> Vec<String> {
    let mut files = Vec::new();
    for commit in commits {
        files.extend(commit.added_files().iter().cloned());
        files.extend(commit.modified_files().iter().cloned());
        files.extend(commit.removed_files().iter().cloned());
    }
    files.sort();
    files.dedup();
    files
}

/// Trait for commit types that carry file change information
trait ChangedFiles {
    fn added_files(&self) -> &[String];
    fn modified_files(&self) -> &[String];
    fn removed_files(&self) -> &[String];
}

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
async fn handle_github_push(state: Arc<AppState>, body: &[u8]) -> Result<StatusCode, StatusCode> {
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

        if let Err(e) = state
            .deploy_tx
            .send((deployment_id.clone(), app.clone()))
            .await
        {
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

/// Deploy or redeploy a preview environment for a GitHub PR
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
                // Re-fetch preview to get updated state after deployment
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

/// Clean up a preview environment when a GitHub PR is closed
async fn handle_preview_cleanup(
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
        tracing::info!(
            preview_id = %preview.id,
            app = %app.name,
            pr = payload.number,
            "Cleaning up preview deployment"
        );

        let db = state.db.clone();
        let runtime = state.runtime.clone();
        let routes = state.routes.clone();
        let encryption_key = state
            .config
            .auth
            .encryption_key
            .as_ref()
            .map(|secret| crypto::derive_key(secret));

        tokio::spawn(async move {
            if let Err(e) = cleanup_preview(&db, runtime, routes, &preview).await {
                tracing::error!(
                    preview_id = %preview.id,
                    error = %e,
                    "Preview cleanup failed"
                );
            }

            // Post "closed" comment on the PR
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
    } else {
        tracing::debug!(
            app = %app.name,
            pr = payload.number,
            "No preview deployment found for cleanup"
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// GitLab Merge Request webhook types and handlers
// ---------------------------------------------------------------------------

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
    #[serde(default)]
    pub added: Vec<String>,
    #[serde(default)]
    pub modified: Vec<String>,
    #[serde(default)]
    pub removed: Vec<String>,
}

impl ChangedFiles for &GitLabCommit {
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

/// GitLab Merge Request event payload (object_kind = "merge_request")
#[derive(Debug, Deserialize)]
pub struct GitLabMergeRequestEvent {
    pub object_attributes: GitLabMergeRequestAttributes,
    pub project: GitLabProject,
    pub user: GitLabUser,
}

#[derive(Debug, Deserialize)]
pub struct GitLabMergeRequestAttributes {
    pub iid: i64,
    pub title: String,
    pub source_branch: String,
    pub target_branch: String,
    pub url: String,
    pub action: Option<String>,
    pub last_commit: Option<GitLabLastCommit>,
}

#[derive(Debug, Deserialize)]
pub struct GitLabLastCommit {
    pub id: String,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GitLabUser {
    pub username: String,
}

/// Generic GitLab webhook payload used for event type detection
#[derive(Debug, Deserialize)]
struct GitLabEventProbe {
    object_kind: Option<String>,
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

    // Detect event type from payload's object_kind field
    let probe: GitLabEventProbe = serde_json::from_slice(&body).map_err(|e| {
        tracing::error!("Failed to parse GitLab webhook payload: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    let event_type = headers
        .get("X-Gitlab-Event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    match (probe.object_kind.as_deref(), event_type) {
        (Some("merge_request"), _) | (_, "Merge Request Hook") => {
            handle_gitlab_merge_request(state, &body).await
        }
        _ => handle_gitlab_push(state, &body).await,
    }
}

/// Handle GitLab push events (regular deployments)
async fn handle_gitlab_push(state: Arc<AppState>, body: &[u8]) -> Result<StatusCode, StatusCode> {
    let payload: GitLabPushEvent = serde_json::from_slice(body).map_err(|e| {
        tracing::error!("Failed to parse GitLab push webhook payload: {}", e);
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

/// Handle GitLab Merge Request events (preview deployments)
async fn handle_gitlab_merge_request(
    state: Arc<AppState>,
    body: &[u8],
) -> Result<StatusCode, StatusCode> {
    let payload: GitLabMergeRequestEvent = serde_json::from_slice(body).map_err(|e| {
        tracing::error!("Failed to parse GitLab MR webhook payload: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    let action = payload
        .object_attributes
        .action
        .as_deref()
        .unwrap_or("unknown");

    tracing::info!(
        "GitLab MR webhook received: {} MR !{} action={}",
        payload.project.path_with_namespace,
        payload.object_attributes.iid,
        action
    );

    let apps = sqlx::query_as::<_, App>(
        "SELECT * FROM apps WHERE (git_url LIKE ? OR git_url LIKE ?) AND preview_enabled = 1",
    )
    .bind(format!("%{}", payload.project.git_http_url))
    .bind(format!("%{}", payload.project.git_ssh_url))
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if apps.is_empty() {
        tracing::debug!(
            "No apps with preview_enabled found for GitLab MR webhook: {}",
            payload.project.path_with_namespace
        );
        return Ok(StatusCode::OK);
    }

    match action {
        "open" | "reopen" | "update" => {
            for app in apps {
                handle_gitlab_mr_deploy(&state, &app, &payload).await?;
            }
        }
        "close" | "merge" => {
            for app in apps {
                handle_generic_preview_cleanup(&state, &app, payload.object_attributes.iid).await?;
            }
        }
        _ => {
            tracing::debug!("Ignoring GitLab MR action: {}", action);
        }
    }

    Ok(StatusCode::OK)
}

/// Deploy or redeploy a preview environment for a GitLab Merge Request
async fn handle_gitlab_mr_deploy(
    state: &Arc<AppState>,
    app: &App,
    payload: &GitLabMergeRequestEvent,
) -> Result<(), StatusCode> {
    let base_domain = state
        .config
        .proxy
        .preview_domain
        .clone()
        .unwrap_or_else(|| "preview.localhost".to_string());

    let commit_sha = payload
        .object_attributes
        .last_commit
        .as_ref()
        .map(|c| c.id.clone());
    let commit_message = payload
        .object_attributes
        .last_commit
        .as_ref()
        .and_then(|c| c.message.clone());

    let info = PreviewDeploymentInfo {
        app_id: app.id.clone(),
        pr_number: payload.object_attributes.iid,
        pr_title: Some(payload.object_attributes.title.clone()),
        pr_source_branch: payload.object_attributes.source_branch.clone(),
        pr_target_branch: payload.object_attributes.target_branch.clone(),
        pr_author: Some(payload.user.username.clone()),
        pr_url: Some(payload.object_attributes.url.clone()),
        commit_sha,
        commit_message,
        provider_type: "gitlab".to_string(),
        repo_full_name: payload.project.path_with_namespace.clone(),
    };

    let preview = find_or_create_preview(&state.db, app, &info, &base_domain)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create preview deployment for GitLab MR: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!(
        preview_id = %preview.id,
        app = %app.name,
        mr = payload.object_attributes.iid,
        domain = %preview.preview_domain,
        "Starting GitLab MR preview deployment"
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
                "GitLab MR preview deployment failed"
            );
        }
    });

    Ok(())
}

// ---------------------------------------------------------------------------
// Gitea Pull Request webhook types and handlers
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

/// Gitea Pull Request event payload
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

/// Generic probe to detect Gitea event type from payload
#[derive(Debug, Deserialize)]
struct GiteaEventProbe {
    action: Option<String>,
    pull_request: Option<serde_json::Value>,
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

    // Detect if this is a pull_request event by checking the X-Gitea-Event header
    let event_type = headers
        .get("X-Gitea-Event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if event_type == "pull_request" {
        return handle_gitea_pull_request(state, &body).await;
    }

    // Fallback: probe the payload for a pull_request field
    if let Ok(probe) = serde_json::from_slice::<GiteaEventProbe>(&body) {
        if probe.pull_request.is_some() && probe.action.is_some() {
            return handle_gitea_pull_request(state, &body).await;
        }
    }

    // Default: handle as push event
    handle_gitea_push(state, &body).await
}

/// Handle Gitea push events (regular deployments)
async fn handle_gitea_push(state: Arc<AppState>, body: &[u8]) -> Result<StatusCode, StatusCode> {
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

/// Handle Gitea pull request events (preview deployments)
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
        tracing::debug!(
            "No apps with preview_enabled found for Gitea PR webhook: {}",
            payload.repository.full_name
        );
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

/// Deploy or redeploy a preview environment for a Gitea Pull Request
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

// ---------------------------------------------------------------------------
// Shared helpers for preview cleanup (used by GitLab and Gitea)
// ---------------------------------------------------------------------------

/// Generic preview cleanup that works for any provider by app_id + pr_number
async fn handle_generic_preview_cleanup(
    state: &Arc<AppState>,
    app: &App,
    pr_number: i64,
) -> Result<(), StatusCode> {
    let preview: Option<PreviewDeployment> =
        sqlx::query_as("SELECT * FROM preview_deployments WHERE app_id = ? AND pr_number = ?")
            .bind(&app.id)
            .bind(pr_number)
            .fetch_optional(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(preview) = preview {
        tracing::info!(
            preview_id = %preview.id,
            app = %app.name,
            pr = pr_number,
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
            pr = pr_number,
            "No preview deployment found for cleanup"
        );
    }

    Ok(())
}

// ============================================================================
// Bitbucket Webhook Handler
// ============================================================================

/// Verify Bitbucket webhook signature (X-Hub-Signature header) - uses HMAC-SHA256
/// Bitbucket Cloud uses the same format as GitHub: sha256=<hex>
fn verify_bitbucket_signature(secret: &str, signature_header: &str, payload: &[u8]) -> bool {
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

    mac.verify_slice(&expected).is_ok()
}

#[derive(Debug, Deserialize)]
pub struct BitbucketPushEvent {
    pub push: BitbucketPush,
    pub repository: BitbucketRepository,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketPush {
    pub changes: Vec<BitbucketChange>,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketChange {
    pub new: Option<BitbucketRef>,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketRef {
    #[serde(rename = "type")]
    pub ref_type: String,
    pub name: String,
    pub target: BitbucketTarget,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketTarget {
    pub hash: String,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketRepository {
    pub full_name: String,
    pub links: BitbucketRepoLinks,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketRepoLinks {
    pub html: BitbucketLink,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketLink {
    pub href: String,
}

/// Bitbucket Pull Request event payload
#[derive(Debug, Deserialize)]
pub struct BitbucketPullRequestEvent {
    pub pullrequest: BitbucketPullRequest,
    pub repository: BitbucketRepository,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketPullRequest {
    pub id: i64,
    pub title: String,
    pub source: BitbucketPullRequestEndpoint,
    pub destination: BitbucketPullRequestEndpoint,
    #[allow(dead_code)]
    pub state: String,
    pub links: BitbucketPRLinks,
    pub author: BitbucketAuthor,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketPullRequestEndpoint {
    pub branch: BitbucketBranch,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketBranch {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketPRLinks {
    pub html: BitbucketLink,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketAuthor {
    pub display_name: String,
}

pub async fn bitbucket_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, StatusCode> {
    // Verify signature if secret is configured (Bitbucket uses X-Hub-Signature header)
    if let Some(ref secret) = state.config.webhooks.bitbucket_secret {
        let signature = headers
            .get("X-Hub-Signature")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                tracing::warn!("Bitbucket webhook missing X-Hub-Signature header");
                StatusCode::UNAUTHORIZED
            })?;

        if !verify_bitbucket_signature(secret, signature, &body) {
            tracing::warn!("Bitbucket webhook signature verification failed");
            return Err(StatusCode::UNAUTHORIZED);
        }
        tracing::debug!("Bitbucket webhook signature verified");
    }

    // Check the event type from X-Event-Key header
    let event_key = headers
        .get("X-Event-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    match event_key {
        "repo:push" => handle_bitbucket_push(state, &body).await,
        "pullrequest:created" | "pullrequest:updated" => {
            handle_bitbucket_pull_request(state, &body, "opened").await
        }
        "pullrequest:fulfilled" => handle_bitbucket_pull_request(state, &body, "closed").await,
        "pullrequest:rejected" => handle_bitbucket_pull_request(state, &body, "closed").await,
        "diagnostics:ping" | "" => {
            tracing::info!("Bitbucket ping received");
            Ok(StatusCode::OK)
        }
        _ => {
            tracing::debug!("Ignoring Bitbucket event type: {}", event_key);
            Ok(StatusCode::OK)
        }
    }
}

/// Handle Bitbucket push events (regular deployments)
async fn handle_bitbucket_push(
    state: Arc<AppState>,
    body: &[u8],
) -> Result<StatusCode, StatusCode> {
    let payload: BitbucketPushEvent = serde_json::from_slice(body).map_err(|e| {
        tracing::error!("Failed to parse Bitbucket push webhook payload: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    for change in &payload.push.changes {
        let new_ref = match &change.new {
            Some(r) => r,
            None => continue,
        };

        if new_ref.ref_type != "branch" {
            continue;
        }

        let branch = &new_ref.name;

        tracing::info!(
            "Bitbucket push webhook received: {} branch {}",
            payload.repository.full_name,
            branch
        );

        // Construct potential clone URLs for matching
        let https_url = format!("https://bitbucket.org/{}.git", payload.repository.full_name);
        let ssh_url = format!("git@bitbucket.org:{}.git", payload.repository.full_name);

        let apps = sqlx::query_as::<_, App>(
            "SELECT * FROM apps WHERE (git_url LIKE ? OR git_url LIKE ? OR git_url LIKE ?) AND branch = ?",
        )
        .bind(format!("%{}", https_url))
        .bind(format!("%{}", ssh_url))
        .bind(format!("%{}%", payload.repository.full_name))
        .bind(branch)
        .fetch_all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if apps.is_empty() {
            tracing::warn!(
                "No matching app found for Bitbucket push webhook: {}",
                payload.repository.full_name
            );
            continue;
        }

        for app in apps {
            let deployment_id = Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();

            sqlx::query(
                r#"
                INSERT INTO deployments (id, app_id, commit_sha, commit_message, status, started_at)
                VALUES (?, ?, ?, ?, 'pending', ?)
                "#,
            )
            .bind(&deployment_id)
            .bind(&app.id)
            .bind(Some(&new_ref.target.hash))
            .bind(new_ref.target.message.as_deref())
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
    }

    Ok(StatusCode::OK)
}

/// Handle Bitbucket pull request events (preview deployments)
async fn handle_bitbucket_pull_request(
    state: Arc<AppState>,
    body: &[u8],
    action: &str,
) -> Result<StatusCode, StatusCode> {
    let payload: BitbucketPullRequestEvent = serde_json::from_slice(body).map_err(|e| {
        tracing::error!("Failed to parse Bitbucket PR webhook payload: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    tracing::info!(
        "Bitbucket PR webhook received: {} PR #{} action={}",
        payload.repository.full_name,
        payload.pullrequest.id,
        action
    );

    let https_url = format!("https://bitbucket.org/{}.git", payload.repository.full_name);
    let ssh_url = format!("git@bitbucket.org:{}.git", payload.repository.full_name);

    let apps = sqlx::query_as::<_, App>(
        "SELECT * FROM apps WHERE (git_url LIKE ? OR git_url LIKE ? OR git_url LIKE ?) AND preview_enabled = 1",
    )
    .bind(format!("%{}", https_url))
    .bind(format!("%{}", ssh_url))
    .bind(format!("%{}%", payload.repository.full_name))
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if apps.is_empty() {
        tracing::debug!(
            "No apps with preview_enabled found for Bitbucket PR webhook: {}",
            payload.repository.full_name
        );
        return Ok(StatusCode::OK);
    }

    match action {
        "opened" => {
            for app in apps {
                handle_bitbucket_preview_deploy(&state, &app, &payload).await?;
            }
        }
        "closed" => {
            for app in apps {
                handle_bitbucket_preview_cleanup(&state, &app, &payload).await?;
            }
        }
        _ => {
            tracing::debug!("Ignoring Bitbucket PR action: {}", action);
        }
    }

    Ok(StatusCode::OK)
}

/// Deploy or redeploy a preview environment for a Bitbucket PR
async fn handle_bitbucket_preview_deploy(
    state: &Arc<AppState>,
    app: &App,
    payload: &BitbucketPullRequestEvent,
) -> Result<(), StatusCode> {
    let base_domain = state
        .config
        .proxy
        .preview_domain
        .clone()
        .unwrap_or_else(|| "preview.localhost".to_string());

    let info = PreviewDeploymentInfo {
        app_id: app.id.clone(),
        pr_number: payload.pullrequest.id,
        pr_title: Some(payload.pullrequest.title.clone()),
        pr_source_branch: payload.pullrequest.source.branch.name.clone(),
        pr_target_branch: payload.pullrequest.destination.branch.name.clone(),
        pr_author: Some(payload.pullrequest.author.display_name.clone()),
        pr_url: Some(payload.pullrequest.links.html.href.clone()),
        commit_sha: None,
        commit_message: None,
        provider_type: "bitbucket".to_string(),
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
        pr = payload.pullrequest.id,
        domain = %preview.preview_domain,
        "Starting Bitbucket preview deployment"
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
                "Bitbucket preview deployment failed"
            );
        }
    });

    Ok(())
}

/// Clean up a preview environment when a Bitbucket PR is closed/merged/declined
async fn handle_bitbucket_preview_cleanup(
    state: &Arc<AppState>,
    app: &App,
    payload: &BitbucketPullRequestEvent,
) -> Result<(), StatusCode> {
    let preview: Option<PreviewDeployment> =
        sqlx::query_as("SELECT * FROM preview_deployments WHERE app_id = ? AND pr_number = ?")
            .bind(&app.id)
            .bind(payload.pullrequest.id)
            .fetch_optional(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(preview) = preview {
        tracing::info!(
            preview_id = %preview.id,
            app = %app.name,
            pr = payload.pullrequest.id,
            "Cleaning up Bitbucket preview deployment"
        );

        let db = state.db.clone();
        let runtime = state.runtime.clone();
        let routes = state.routes.clone();

        tokio::spawn(async move {
            if let Err(e) = cleanup_preview(&db, runtime, routes, &preview).await {
                tracing::error!(
                    preview_id = %preview.id,
                    error = %e,
                    "Bitbucket preview cleanup failed"
                );
            }
        });
    } else {
        tracing::debug!(
            app = %app.name,
            pr = payload.pullrequest.id,
            "No preview deployment found for Bitbucket cleanup"
        );
    }

    Ok(())
}
