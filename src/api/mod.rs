mod apps;
mod audit;
pub mod auth;
mod basic_auth;
mod database_backups;
mod databases;
mod deployments;
mod env_vars;
pub mod error;
mod git_providers;
mod github_apps;
pub mod metrics;
mod notifications;
mod previews;
mod projects;
pub mod rate_limit;
mod routes;
mod service_templates;
mod services;
mod ssh_keys;
mod system;
mod teams;
mod validation;
mod volumes;
mod webhooks;
mod ws;

use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;
use tower_http::services::{ServeDir, ServeFile};
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
        // GitHub App callbacks (public - GitHub redirects here)
        .route("/github-apps/callback", get(github_apps::manifest_callback))
        .route("/github-apps/installation/callback", get(github_apps::installation_callback))
        // Apply auth-tier rate limiting (stricter limits for auth endpoints)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit_auth,
        ));

    // WebSocket routes (auth handled in handlers via query param)
    let ws_routes = Router::new()
        .route("/deployments/:id/logs/stream", get(ws::deployment_logs_ws))
        .route("/apps/:id/terminal", get(ws::terminal_ws));

    // Protected API routes
    let api_routes = Router::new()
        // Apps
        .route("/apps", get(apps::list_apps))
        .route("/apps", post(apps::create_app))
        .route("/apps/:id", get(apps::get_app))
        .route("/apps/:id", put(apps::update_app))
        .route("/apps/:id", delete(apps::delete_app))
        .route("/apps/:id/status", get(apps::get_app_status))
        .route("/apps/:id/start", post(apps::start_app))
        .route("/apps/:id/stop", post(apps::stop_app))
        .route("/apps/:id/restart", post(apps::restart_app))
        .route("/apps/:id/logs/stream", get(apps::stream_app_logs))
        // Deployments
        .route("/apps/:id/deploy", post(deployments::trigger_deploy))
        .route("/apps/:id/deploy/upload", post(deployments::upload_deploy))
        .route("/apps/:id/deployments", get(deployments::list_deployments))
        .route("/apps/:id/stats", get(deployments::get_app_stats))
        .route("/deployments/:id", get(deployments::get_deployment))
        .route("/deployments/:id/logs", get(deployments::get_logs))
        .route("/deployments/:id/rollback", post(deployments::rollback_deployment))
        // Build detection
        .route("/build/detect", post(deployments::detect_build_type_from_upload))
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
        // Volumes
        .route("/apps/:id/volumes", get(volumes::list_volumes))
        .route("/apps/:id/volumes", post(volumes::create_volume))
        .route("/volumes/:id", get(volumes::get_volume))
        .route("/volumes/:id", put(volumes::update_volume))
        .route("/volumes/:id", delete(volumes::delete_volume))
        .route("/volumes/:id/backup", post(volumes::backup_volume))
        // Routes (proxy management)
        .route("/routes", get(routes::list_routes))
        .route("/routes", post(routes::add_route))
        .route("/routes/domains", get(routes::list_domains))
        .route("/routes/health", get(routes::routes_health))
        .route("/routes/:domain", get(routes::get_route))
        .route("/routes/:domain", delete(routes::remove_route))
        .route("/routes/:domain/health", put(routes::update_route_health))
        // Git Providers (OAuth connections and PAT)
        .route("/git-providers", get(git_providers::list_providers))
        .route("/git-providers", post(git_providers::add_token_provider))
        .route("/git-providers/:id", get(git_providers::get_provider))
        .route("/git-providers/:id", delete(git_providers::delete_provider))
        .route("/git-providers/:id/repos", get(git_providers::list_repos))
        // Projects
        .route("/projects", get(projects::list_projects))
        .route("/projects", post(projects::create_project))
        .route("/projects/:id", get(projects::get_project))
        .route("/projects/:id", put(projects::update_project))
        .route("/projects/:id", delete(projects::delete_project))
        .route("/projects/:id/apps/upload", post(apps::upload_create_app))
        .route("/apps/:id/project", put(projects::assign_app_project))
        // Teams
        .route("/teams", get(teams::list_teams))
        .route("/teams", post(teams::create_team))
        .route("/teams/:id", get(teams::get_team))
        .route("/teams/:id", put(teams::update_team))
        .route("/teams/:id", delete(teams::delete_team))
        .route("/teams/:id/members", get(teams::list_members))
        .route("/teams/:id/members", post(teams::invite_member))
        .route("/teams/:id/members/:user_id", put(teams::update_member_role))
        .route("/teams/:id/members/:user_id", delete(teams::remove_member))
        // Notification Channels
        .route("/notification-channels", get(notifications::list_channels))
        .route("/notification-channels", post(notifications::create_channel))
        .route("/notification-channels/:id", get(notifications::get_channel))
        .route("/notification-channels/:id", put(notifications::update_channel))
        .route("/notification-channels/:id", delete(notifications::delete_channel))
        .route("/notification-channels/:id/test", post(notifications::test_channel))
        .route("/notification-channels/:id/subscriptions", get(notifications::list_subscriptions))
        .route("/notification-channels/:id/subscriptions", post(notifications::create_subscription))
        .route("/notification-subscriptions/:id", delete(notifications::delete_subscription))
        // Managed Databases
        .route("/databases", get(databases::list_databases))
        .route("/databases", post(databases::create_database))
        .route("/databases/:id", get(databases::get_database))
        .route("/databases/:id", put(databases::update_database))
        .route("/databases/:id", delete(databases::delete_database))
        .route("/databases/:id/start", post(databases::start_database))
        .route("/databases/:id/stop", post(databases::stop_database))
        .route("/databases/:id/logs", get(databases::get_database_logs))
        .route("/databases/:id/stats", get(databases::get_database_stats))
        // Database Backups
        .route("/databases/:id/backups", get(database_backups::list_backups))
        .route("/databases/:id/backups", post(database_backups::create_backup))
        .route("/databases/:id/backups/:backup_id", get(database_backups::get_backup))
        .route("/databases/:id/backups/:backup_id", delete(database_backups::delete_backup))
        .route("/databases/:id/backups/:backup_id/download", get(database_backups::download_backup))
        .route("/databases/:id/backups/schedule", get(database_backups::get_schedule))
        .route("/databases/:id/backups/schedule", post(database_backups::upsert_schedule))
        .route("/databases/:id/backups/schedule", delete(database_backups::delete_schedule))
        // Docker Compose Services
        .route("/services", get(services::list_services))
        .route("/services", post(services::create_service))
        .route("/services/:id", get(services::get_service))
        .route("/services/:id", put(services::update_service))
        .route("/services/:id", delete(services::delete_service))
        .route("/services/:id/start", post(services::start_service))
        .route("/services/:id/stop", post(services::stop_service))
        .route("/services/:id/logs", get(services::get_service_logs))
        .route("/services/:id/logs/stream", get(services::stream_service_logs))
        // Service Templates
        .route("/templates", get(service_templates::list_templates))
        .route("/templates/categories", get(service_templates::list_categories))
        .route("/templates/:id", get(service_templates::get_template))
        .route("/templates/:id/deploy", post(service_templates::deploy_template))
        // System stats and events
        .route("/system/stats", get(system::get_system_stats))
        .route("/system/stats/history", get(system::get_stats_history))
        .route("/system/disk", get(system::get_disk_stats))
        .route("/system/health", get(system::get_detailed_health))
        .route("/events/recent", get(system::get_recent_events))
        // Audit logs
        .route("/audit", get(audit::list_logs))
        .route("/audit/actions", get(audit::list_action_types))
        .route("/audit/resource-types", get(audit::list_resource_types))
        // GitHub Apps (callbacks are in public auth_routes)
        .route("/github-apps", get(github_apps::list_apps))
        .route("/github-apps", post(github_apps::create_manifest))
        // List all installations (must be before :id routes)
        .route("/github-apps/installations", get(github_apps::list_all_installations))
        // Get repos by installation ID (simpler pattern for frontend)
        .route("/github-apps/installations/:installation_id/repos", get(github_apps::list_repos_by_installation))
        // Get branches for a repo
        .route("/github-apps/installations/:installation_id/repos/:owner/:repo/branches", get(github_apps::list_repo_branches))
        .route("/github-apps/:id", get(github_apps::get_app))
        .route("/github-apps/:id/install", get(github_apps::get_install_url))
        .route("/github-apps/:id/installations", get(github_apps::list_installations))
        .route("/github-apps/:id/installations/:iid/repos", get(github_apps::list_installation_repos))
        // Preview Deployments (PR previews)
        .route("/apps/:id/previews", get(previews::list_app_previews))
        .route("/previews", get(previews::list_all_previews))
        .route("/previews/status/:status", get(previews::list_previews_by_status))
        .route("/previews/:id", get(previews::get_preview))
        .route("/previews/:id", delete(previews::delete_preview))
        .route("/previews/:id/redeploy", post(previews::redeploy_preview))
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

    // Static file serving for SPA frontend
    // Serves from static/dist/client with fallback to __spa-fallback.html for client-side routing
    // Note: index.html is pre-rendered for "/" only, __spa-fallback.html is the proper SPA shell
    let static_dir = std::path::Path::new("static/dist/client");
    let fallback_file = static_dir.join("__spa-fallback.html");

    let serve_static = ServeDir::new(static_dir)
        .not_found_service(ServeFile::new(&fallback_file));

    Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(metrics::metrics_endpoint))
        .nest("/api/auth", auth_routes)
        .nest("/api", api_routes)
        .nest("/webhooks", webhook_routes)
        // Fallback to static files for frontend SPA
        .fallback_service(serve_static)
        .layer(middleware::from_fn(metrics::metrics_middleware))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}
