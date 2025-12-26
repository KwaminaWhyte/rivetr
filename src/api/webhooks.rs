use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::App;
use crate::AppState;

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
    _headers: HeaderMap, // TODO: Verify webhook signature
    Json(payload): Json<GitHubPushEvent>,
) -> Result<StatusCode, StatusCode> {
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
    Json(payload): Json<GitLabPushEvent>,
) -> Result<StatusCode, StatusCode> {
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
    Json(payload): Json<GiteaPushEvent>,
) -> Result<StatusCode, StatusCode> {
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
