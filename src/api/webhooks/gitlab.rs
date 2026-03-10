//! GitLab webhook handler (push events and merge request preview deployments).

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use super::{
    collect_changed_files, handle_generic_preview_cleanup, incr_webhooks,
    should_deploy_for_changed_files, ChangedFiles,
};
use crate::crypto;
use crate::db::App;
use crate::engine::preview::{
    find_or_create_preview, run_preview_deployment, PreviewDeploymentInfo,
};
use crate::AppState;

// ---------------------------------------------------------------------------
// GitLab payload types
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

#[derive(Debug, Deserialize)]
struct GitLabEventProbe {
    object_kind: Option<String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn gitlab_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, StatusCode> {
    incr_webhooks("gitlab");

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

    let changed_files = collect_changed_files(payload.commits.iter());

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

    Ok(StatusCode::OK)
}

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
