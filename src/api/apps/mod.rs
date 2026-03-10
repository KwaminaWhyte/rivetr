use serde::Serialize;

use crate::db::{App, CreateAppRequest, UpdateAppRequest};

use super::error::{ApiError, ValidationErrorBuilder};
use super::validation::{
    validate_app_name, validate_base_directory, validate_branch, validate_build_target,
    validate_build_type, validate_cpu_limit, validate_custom_docker_options,
    validate_deployment_commands, validate_docker_image, validate_dockerfile, validate_domain,
    validate_domains, validate_environment, validate_extra_hosts, validate_git_url,
    validate_healthcheck, validate_memory_limit, validate_network_aliases, validate_port,
    validate_port_mappings, validate_watch_paths,
};

mod control;
mod crud;
mod logs;
mod sharing;
mod upload;

pub use control::{get_app_status, restart_app, start_app, stop_app};
pub use crud::{create_app, delete_app, get_app, list_apps, update_app};
pub use logs::stream_app_logs;
pub use sharing::{create_app_share, delete_app_share, list_app_shares, list_apps_with_sharing};
pub use upload::upload_create_app;

/// Query parameters for listing apps
#[derive(Debug, serde::Deserialize)]
pub struct ListAppsQuery {
    /// Filter by team ID. If provided, returns only apps belonging to this team.
    /// If not provided, returns all apps the user has access to.
    pub team_id: Option<String>,
}

/// Response for app status
#[derive(serde::Serialize)]
pub struct AppStatusResponse {
    pub app_id: String,
    pub container_id: Option<String>,
    pub running: bool,
    pub status: String,
    /// The host port the container is accessible on (for "Open App" functionality)
    pub host_port: Option<u16>,
    /// Blue/green deployment phase: "stable" | "deploying" | "health_checking" | "switching"
    pub deployment_phase: String,
    /// ID of the currently active deployment
    pub active_deployment_id: Option<String>,
    /// Seconds since the active deployment started (uptime indicator)
    pub uptime_seconds: Option<i64>,
}

/// Request to delete an app (requires password confirmation)
#[derive(serde::Deserialize)]
pub struct DeleteAppRequest {
    pub password: String,
}

/// Configuration for creating an app from upload
#[derive(serde::Deserialize)]
pub struct UploadAppConfig {
    pub name: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub domain: Option<String>,
    pub healthcheck: Option<String>,
    #[serde(default = "default_cpu_limit")]
    pub cpu_limit: String,
    #[serde(default = "default_memory_limit")]
    pub memory_limit: String,
    #[serde(default = "default_environment")]
    pub environment: String,
    /// Optional build type override (auto-detected if not specified)
    pub build_type: Option<String>,
    /// Optional publish directory for static sites
    pub publish_directory: Option<String>,
}

pub(super) fn default_port() -> u16 {
    3000
}

pub(super) fn default_cpu_limit() -> String {
    "1".to_string()
}

pub(super) fn default_memory_limit() -> String {
    "512m".to_string()
}

pub(super) fn default_environment() -> String {
    "development".to_string()
}

/// Response for upload app creation
#[derive(Serialize)]
pub struct UploadAppResponse {
    pub app: App,
    pub deployment_id: String,
    pub detected_build_type: crate::engine::BuildDetectionResult,
}

/// List apps with sharing information for a team
/// GET /api/apps/with-sharing?team_id=xxx
#[derive(Debug, serde::Deserialize)]
pub struct ListAppsWithSharingQuery {
    /// Team ID to get apps for (owned + shared)
    pub team_id: String,
}

/// Validate a CreateAppRequest
pub(super) fn validate_create_request(req: &CreateAppRequest) -> Result<(), ApiError> {
    let mut errors = ValidationErrorBuilder::new();

    if let Err(e) = validate_app_name(&req.name) {
        errors.add("name", &e);
    }

    // Check deployment source: either git_url OR docker_image must be provided, not both
    let has_git_url = !req.git_url.is_empty();
    let has_docker_image = req
        .docker_image
        .as_ref()
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    if has_git_url && has_docker_image {
        errors.add(
            "docker_image",
            "Cannot specify both git_url and docker_image. Choose one deployment source.",
        );
    } else if !has_git_url && !has_docker_image {
        errors.add("git_url", "Either git_url or docker_image must be provided");
    }

    // Only validate git-related fields if using git source
    if has_git_url {
        if let Err(e) = validate_git_url(&req.git_url) {
            errors.add("git_url", &e);
        }

        if let Err(e) = validate_branch(&req.branch) {
            errors.add("branch", &e);
        }

        if let Err(e) = validate_dockerfile(&req.dockerfile) {
            errors.add("dockerfile", &e);
        }
    }

    // Validate docker_image if provided
    if has_docker_image {
        if let Err(e) = validate_docker_image(req.docker_image.as_deref()) {
            errors.add("docker_image", &e);
        }
    }

    if let Err(e) = validate_domain(&req.domain) {
        errors.add("domain", &e);
    }

    if let Err(e) = validate_port(req.port) {
        errors.add("port", &e);
    }

    if let Err(e) = validate_healthcheck(&req.healthcheck) {
        errors.add("healthcheck", &e);
    }

    if let Err(e) = validate_memory_limit(&req.memory_limit) {
        errors.add("memory_limit", &e);
    }

    if let Err(e) = validate_cpu_limit(&req.cpu_limit) {
        errors.add("cpu_limit", &e);
    }

    // Environment is validated through serde deserialization (enum), but we double-check here
    if let Err(e) = validate_environment(&req.environment.to_string()) {
        errors.add("environment", &e);
    }

    // Advanced build options
    if let Err(e) = validate_base_directory(&req.base_directory) {
        errors.add("base_directory", &e);
    }

    if let Err(e) = validate_build_target(&req.build_target) {
        errors.add("build_target", &e);
    }

    if let Err(e) = validate_watch_paths(&req.watch_paths) {
        errors.add("watch_paths", &e);
    }

    if let Err(e) = validate_custom_docker_options(&req.custom_docker_options) {
        errors.add("custom_docker_options", &e);
    }

    // Network configuration
    if let Err(e) = validate_port_mappings(&req.port_mappings) {
        errors.add("port_mappings", &e);
    }

    if let Err(e) = validate_network_aliases(&req.network_aliases) {
        errors.add("network_aliases", &e);
    }

    if let Err(e) = validate_extra_hosts(&req.extra_hosts) {
        errors.add("extra_hosts", &e);
    }

    // Deployment commands
    if let Err(e) = validate_deployment_commands(&req.pre_deploy_commands, "pre_deploy_commands") {
        errors.add("pre_deploy_commands", &e);
    }

    if let Err(e) = validate_deployment_commands(&req.post_deploy_commands, "post_deploy_commands")
    {
        errors.add("post_deploy_commands", &e);
    }

    // Domain management
    if let Err(e) = validate_domains(&req.domains) {
        errors.add("domains", &e);
    }

    // Build type validation
    if let Err(e) = validate_build_type(&req.build_type) {
        errors.add("build_type", &e);
    }

    errors.finish()
}

/// Validate an UpdateAppRequest (only validates provided fields)
pub(super) fn validate_update_request(req: &UpdateAppRequest) -> Result<(), ApiError> {
    let mut errors = ValidationErrorBuilder::new();

    if let Some(ref name) = req.name {
        if let Err(e) = validate_app_name(name) {
            errors.add("name", &e);
        }
    }

    // Only validate git_url if it's provided and non-empty
    // Empty string means "clear" which is valid when using docker_image
    if let Some(ref git_url) = req.git_url {
        if !git_url.is_empty() {
            if let Err(e) = validate_git_url(git_url) {
                errors.add("git_url", &e);
            }
        }
    }

    // Only validate branch if it's provided and non-empty
    // Empty string means "clear" which is valid when using docker_image
    if let Some(ref branch) = req.branch {
        if !branch.is_empty() {
            if let Err(e) = validate_branch(branch) {
                errors.add("branch", &e);
            }
        }
    }

    // Only validate dockerfile if it's provided and non-empty
    // Empty string means "clear" which is valid when using docker_image
    if let Some(ref dockerfile) = req.dockerfile {
        if !dockerfile.is_empty() {
            if let Err(e) = validate_dockerfile(dockerfile) {
                errors.add("dockerfile", &e);
            }
        }
    }

    if let Err(e) = validate_domain(&req.domain) {
        errors.add("domain", &e);
    }

    if let Some(port) = req.port {
        if let Err(e) = validate_port(port) {
            errors.add("port", &e);
        }
    }

    if let Err(e) = validate_healthcheck(&req.healthcheck) {
        errors.add("healthcheck", &e);
    }

    if let Err(e) = validate_memory_limit(&req.memory_limit) {
        errors.add("memory_limit", &e);
    }

    if let Err(e) = validate_cpu_limit(&req.cpu_limit) {
        errors.add("cpu_limit", &e);
    }

    if let Some(ref environment) = req.environment {
        if let Err(e) = validate_environment(&environment.to_string()) {
            errors.add("environment", &e);
        }
    }

    // Advanced build options
    if let Err(e) = validate_base_directory(&req.base_directory) {
        errors.add("base_directory", &e);
    }

    if let Err(e) = validate_build_target(&req.build_target) {
        errors.add("build_target", &e);
    }

    if let Err(e) = validate_watch_paths(&req.watch_paths) {
        errors.add("watch_paths", &e);
    }

    if let Err(e) = validate_custom_docker_options(&req.custom_docker_options) {
        errors.add("custom_docker_options", &e);
    }

    // Network configuration
    if let Err(e) = validate_port_mappings(&req.port_mappings) {
        errors.add("port_mappings", &e);
    }

    if let Err(e) = validate_network_aliases(&req.network_aliases) {
        errors.add("network_aliases", &e);
    }

    if let Err(e) = validate_extra_hosts(&req.extra_hosts) {
        errors.add("extra_hosts", &e);
    }

    // Deployment commands
    if let Err(e) = validate_deployment_commands(&req.pre_deploy_commands, "pre_deploy_commands") {
        errors.add("pre_deploy_commands", &e);
    }

    if let Err(e) = validate_deployment_commands(&req.post_deploy_commands, "post_deploy_commands")
    {
        errors.add("post_deploy_commands", &e);
    }

    // Domain management
    if let Err(e) = validate_domains(&req.domains) {
        errors.add("domains", &e);
    }

    // Build type validation (only if provided)
    if let Some(ref build_type) = req.build_type {
        if !build_type.is_empty() {
            if let Err(e) = validate_build_type(build_type) {
                errors.add("build_type", &e);
            }
        }
    }

    errors.finish()
}

/// Helper to merge optional string values
/// - None means "don't change" -> keep existing
/// - Some("") means "clear" -> set to None
/// - Some(value) means "set" -> use the value
pub(super) fn merge_optional_string(
    new_val: &Option<String>,
    existing: &Option<String>,
) -> Option<String> {
    match new_val {
        Some(s) if s.is_empty() => None, // Explicit clear
        Some(s) => Some(s.clone()),      // New value
        None => existing.clone(),        // Keep existing
    }
}

/// Helper to merge optional vectors (serialized as JSON)
/// - None means "don't change"
/// - Some(empty vec) means "clear"
/// - Some(vec) means "set"
pub(super) fn merge_optional_json<T: serde::Serialize>(
    new_val: &Option<Vec<T>>,
    existing: &Option<String>,
) -> Option<String> {
    match new_val {
        Some(v) if v.is_empty() => None,          // Explicit clear
        Some(v) => serde_json::to_string(v).ok(), // New value
        None => existing.clone(),                 // Keep existing
    }
}
