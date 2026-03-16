//! Webhook handlers for Git providers.
//!
//! Each provider has its own submodule. Shared utilities (signature verification,
//! watch-path filtering, preview cleanup) live here.

mod bitbucket;
mod dockerhub;
mod gitea;
mod github;
mod gitlab;

pub use bitbucket::bitbucket_webhook;
pub use dockerhub::dockerhub_webhook;
pub use gitea::gitea_webhook;
pub use github::github_webhook;
pub use gitlab::gitlab_webhook;

pub(super) use crate::api::metrics::increment_webhooks_received as incr_webhooks;
pub(super) use crate::api::webhook_events::log_webhook_event as log_wh_event;
pub(super) use crate::api::webhook_events::record_delivery_id;
pub(super) use crate::api::webhook_events::update_webhook_event as update_wh_event;

use axum::http::StatusCode;
use glob::Pattern;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::Arc;

use crate::db::{App, PreviewDeployment};
use crate::engine::preview::cleanup_preview;
use crate::AppState;

pub(super) type HmacSha256 = Hmac<Sha256>;

/// Verify GitHub/GitLab/Bitbucket webhook signature (sha256=<hex> format)
pub(super) fn verify_github_signature(
    secret: &str,
    signature_header: &str,
    payload: &[u8],
) -> bool {
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

/// Verify Gitea webhook signature (X-Gitea-Signature header) - plain hex HMAC-SHA256
pub(super) fn verify_gitea_signature(secret: &str, signature_header: &str, payload: &[u8]) -> bool {
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
pub(super) fn should_deploy_for_changed_files(app: &App, changed_files: &[String]) -> bool {
    let watch_paths = app.get_watch_paths();
    if watch_paths.is_empty() {
        return true;
    }

    let patterns: Vec<Pattern> = watch_paths
        .iter()
        .filter_map(|p| {
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
        return true;
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
pub(super) fn collect_changed_files(
    commits: impl IntoIterator<Item = impl ChangedFiles>,
) -> Vec<String> {
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
pub(super) trait ChangedFiles {
    fn added_files(&self) -> &[String];
    fn modified_files(&self) -> &[String];
    fn removed_files(&self) -> &[String];
}

/// Generic preview cleanup shared between GitLab and Gitea handlers
pub(super) async fn handle_generic_preview_cleanup(
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
