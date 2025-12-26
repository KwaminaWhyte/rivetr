mod apps;
pub mod auth;
mod deployments;
mod ssh_keys;
mod validation;
mod webhooks;
mod ws;

use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

use crate::AppState;

pub fn create_router(state: Arc<AppState>) -> Router {
    // Auth routes (public)
    let auth_routes = Router::new()
        .route("/login", post(auth::login))
        .route("/validate", get(auth::validate))
        .route("/setup-status", get(auth::setup_status))
        .route("/setup", post(auth::setup));

    // Protected API routes
    let api_routes = Router::new()
        // Apps
        .route("/apps", get(apps::list_apps))
        .route("/apps", post(apps::create_app))
        .route("/apps/:id", get(apps::get_app))
        .route("/apps/:id", put(apps::update_app))
        .route("/apps/:id", delete(apps::delete_app))
        // Deployments
        .route("/apps/:id/deploy", post(deployments::trigger_deploy))
        .route("/apps/:id/deployments", get(deployments::list_deployments))
        .route("/deployments/:id", get(deployments::get_deployment))
        .route("/deployments/:id/logs", get(deployments::get_logs))
        .route("/deployments/:id/rollback", post(deployments::rollback_deployment))
        // WebSocket for streaming logs (auth handled in handler via query param)
        .route("/deployments/:id/logs/stream", get(ws::deployment_logs_ws))
        // WebSocket for streaming runtime container logs
        .route("/apps/:id/logs/stream", get(ws::runtime_logs_ws))
        // SSH Keys
        .route("/ssh-keys", get(ssh_keys::list_ssh_keys))
        .route("/ssh-keys", post(ssh_keys::create_ssh_key))
        .route("/ssh-keys/:id", get(ssh_keys::get_ssh_key))
        .route("/ssh-keys/:id", put(ssh_keys::update_ssh_key))
        .route("/ssh-keys/:id", delete(ssh_keys::delete_ssh_key))
        .route("/apps/:id/ssh-keys", get(ssh_keys::get_app_ssh_keys))
        // Protected by auth
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ));

    let webhook_routes = Router::new()
        .route("/github", post(webhooks::github_webhook))
        .route("/gitlab", post(webhooks::gitlab_webhook))
        .route("/gitea", post(webhooks::gitea_webhook));

    Router::new()
        .route("/health", get(health_check))
        .nest("/api/auth", auth_routes)
        .nest("/api", api_routes)
        .nest("/webhooks", webhook_routes)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}
