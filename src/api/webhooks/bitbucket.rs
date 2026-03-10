//! Bitbucket webhook handler (push events and pull request preview deployments).

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use super::{handle_generic_preview_cleanup, verify_github_signature};
use crate::crypto;
use crate::db::App;
use crate::engine::preview::{
    find_or_create_preview, run_preview_deployment, PreviewDeploymentInfo,
};
use crate::AppState;

// ---------------------------------------------------------------------------
// Bitbucket payload types
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn bitbucket_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, StatusCode> {
    if let Some(ref secret) = state.config.webhooks.bitbucket_secret {
        let signature = headers
            .get("X-Hub-Signature")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                tracing::warn!("Bitbucket webhook missing X-Hub-Signature header");
                StatusCode::UNAUTHORIZED
            })?;

        // Bitbucket uses same sha256= prefix format as GitHub
        if !verify_github_signature(secret, signature, &body) {
            tracing::warn!("Bitbucket webhook signature verification failed");
            return Err(StatusCode::UNAUTHORIZED);
        }
        tracing::debug!("Bitbucket webhook signature verified");
    }

    let event_key = headers
        .get("X-Event-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    match event_key {
        "repo:push" => handle_bitbucket_push(state, &body).await,
        "pullrequest:created" | "pullrequest:updated" => {
            handle_bitbucket_pull_request(state, &body, "opened").await
        }
        "pullrequest:fulfilled" | "pullrequest:rejected" => {
            handle_bitbucket_pull_request(state, &body, "closed").await
        }
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
        let https_url = format!("https://bitbucket.org/{}.git", payload.repository.full_name);
        let ssh_url = format!("git@bitbucket.org:{}.git", payload.repository.full_name);

        tracing::info!(
            "Bitbucket push webhook received: {} branch {}",
            payload.repository.full_name,
            branch
        );

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
                handle_generic_preview_cleanup(&state, &app, payload.pullrequest.id).await?;
            }
        }
        _ => {}
    }

    Ok(StatusCode::OK)
}

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
