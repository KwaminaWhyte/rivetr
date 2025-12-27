mod apps;
pub mod auth;
mod basic_auth;
mod deployments;
mod env_vars;
pub mod error;
mod git_providers;
pub mod metrics;
mod projects;
pub mod rate_limit;
mod routes;
mod ssh_keys;
mod system;
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
use rate_limit::{rate_limit_api, rate_limit_auth, rate_limit_webhook};

pub fn create_router(state: Arc<AppState>) -> Router {
    // Auth routes (public, but rate limited)
    let auth_routes = Router::new()
        .route("/login", post(auth::login))
        .route("/validate", get(auth::validate))
        .route("/setup-status", get(auth::setup_status))
        .route("/setup", post(auth::setup))
        // OAuth routes
        .route("/oauth/:provider/authorize", get(git_providers::get_auth_url))
        .route("/oauth/:provider/callback", get(git_providers::oauth_callback))
        // Apply auth-tier rate limiting (stricter limits for auth endpoints)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit_auth,
        ));

    // WebSocket routes (auth handled in handlers via query param)
    let ws_routes = Router::new()
        .route("/deployments/:id/logs/stream", get(ws::deployment_logs_ws))
        .route("/apps/:id/logs/stream", get(ws::runtime_logs_ws))
        .route("/apps/:id/terminal", get(ws::terminal_ws));

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
        .route("/apps/:id/stats", get(deployments::get_app_stats))
        .route("/deployments/:id", get(deployments::get_deployment))
        .route("/deployments/:id/logs", get(deployments::get_logs))
        .route("/deployments/:id/rollback", post(deployments::rollback_deployment))
        // SSH Keys
        .route("/ssh-keys", get(ssh_keys::list_ssh_keys))
        .route("/ssh-keys", post(ssh_keys::create_ssh_key))
        .route("/ssh-keys/:id", get(ssh_keys::get_ssh_key))
        .route("/ssh-keys/:id", put(ssh_keys::update_ssh_key))
        .route("/ssh-keys/:id", delete(ssh_keys::delete_ssh_key))
        .route("/apps/:id/ssh-keys", get(ssh_keys::get_app_ssh_keys))
        // Environment Variables
        .route("/apps/:id/env-vars", get(env_vars::list_env_vars))
        .route("/apps/:id/env-vars", post(env_vars::create_env_var))
        .route("/apps/:id/env-vars/:key", get(env_vars::get_env_var))
        .route("/apps/:id/env-vars/:key", put(env_vars::update_env_var))
        .route("/apps/:id/env-vars/:key", delete(env_vars::delete_env_var))
        // HTTP Basic Auth
        .route("/apps/:id/basic-auth", get(basic_auth::get_basic_auth))
        .route("/apps/:id/basic-auth", put(basic_auth::update_basic_auth))
        .route("/apps/:id/basic-auth", delete(basic_auth::delete_basic_auth))
        // Routes (proxy management)
        .route("/routes", get(routes::list_routes))
        .route("/routes", post(routes::add_route))
        .route("/routes/domains", get(routes::list_domains))
        .route("/routes/health", get(routes::routes_health))
        .route("/routes/:domain", get(routes::get_route))
        .route("/routes/:domain", delete(routes::remove_route))
        .route("/routes/:domain/health", put(routes::update_route_health))
        // Git Providers (OAuth connections)
        .route("/git-providers", get(git_providers::list_providers))
        .route("/git-providers/:id", get(git_providers::get_provider))
        .route("/git-providers/:id", delete(git_providers::delete_provider))
        .route("/git-providers/:id/repos", get(git_providers::list_repos))
        // Projects
        .route("/projects", get(projects::list_projects))
        .route("/projects", post(projects::create_project))
        .route("/projects/:id", get(projects::get_project))
        .route("/projects/:id", put(projects::update_project))
        .route("/projects/:id", delete(projects::delete_project))
        .route("/apps/:id/project", put(projects::assign_app_project))
        // System stats and events
        .route("/system/stats", get(system::get_system_stats))
        .route("/events/recent", get(system::get_recent_events))
        // Protected by auth
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ))
        // Apply API-tier rate limiting (after auth middleware in the layer stack)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit_api,
        ))
        // Merge WS routes (they handle their own auth)
        .merge(ws_routes);

    // Webhook routes with higher rate limits
    let webhook_routes = Router::new()
        .route("/github", post(webhooks::github_webhook))
        .route("/gitlab", post(webhooks::gitlab_webhook))
        .route("/gitea", post(webhooks::gitea_webhook))
        // Apply webhook-tier rate limiting (higher limits for webhooks)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit_webhook,
        ));

    Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(metrics::metrics_endpoint))
        .nest("/api/auth", auth_routes)
        .nest("/api", api_routes)
        .nest("/webhooks", webhook_routes)
        .layer(middleware::from_fn(metrics::metrics_middleware))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}
