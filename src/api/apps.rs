use axum::{
    extract::{Multipart, Path, State},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, Sse},
    Json,
};
use futures::stream::Stream;
use serde::Serialize;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use uuid::Uuid;

use crate::engine::{detect_build_type, extract_zip_and_find_root, BuildDetectionResult};

use crate::db::{
    actions, resource_types, App, CreateAppRequest, Deployment, UpdateAppRequest, User,
};
use crate::AppState;

use super::audit::{audit_log, extract_client_ip};
use super::auth::verify_password;

use super::error::{ApiError, ValidationErrorBuilder};
use super::validation::{
    validate_app_name, validate_base_directory, validate_branch, validate_build_target,
    validate_build_type, validate_cpu_limit, validate_custom_docker_options,
    validate_deployment_commands, validate_docker_image, validate_dockerfile, validate_domain,
    validate_domains, validate_environment, validate_extra_hosts, validate_git_url,
    validate_healthcheck, validate_memory_limit, validate_network_aliases, validate_port,
    validate_port_mappings, validate_uuid, validate_watch_paths,
};

/// Validate a CreateAppRequest
fn validate_create_request(req: &CreateAppRequest) -> Result<(), ApiError> {
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
fn validate_update_request(req: &UpdateAppRequest) -> Result<(), ApiError> {
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

pub async fn list_apps(State(state): State<Arc<AppState>>) -> Result<Json<Vec<App>>, ApiError> {
    let apps = sqlx::query_as::<_, App>("SELECT * FROM apps ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await?;

    Ok(Json(apps))
}

pub async fn get_app(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<App>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    Ok(Json(app))
}

pub async fn create_app(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Json(req): Json<CreateAppRequest>,
) -> Result<(StatusCode, Json<App>), ApiError> {
    // Validate request
    validate_create_request(&req)?;

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // Serialize network configuration to JSON strings
    let port_mappings_json = req
        .port_mappings
        .as_ref()
        .map(|pm| serde_json::to_string(pm).unwrap_or_default());
    let network_aliases_json = req
        .network_aliases
        .as_ref()
        .map(|na| serde_json::to_string(na).unwrap_or_default());
    let extra_hosts_json = req
        .extra_hosts
        .as_ref()
        .map(|eh| serde_json::to_string(eh).unwrap_or_default());
    let domains_json = req
        .domains
        .as_ref()
        .map(|d| serde_json::to_string(d).unwrap_or_default());
    let pre_deploy_commands_json = req
        .pre_deploy_commands
        .as_ref()
        .map(|c| serde_json::to_string(c).unwrap_or_default());
    let post_deploy_commands_json = req
        .post_deploy_commands
        .as_ref()
        .map(|c| serde_json::to_string(c).unwrap_or_default());
    let container_labels_json = req
        .container_labels
        .as_ref()
        .map(|l| serde_json::to_string(l).unwrap_or_default());

    // Generate auto_subdomain if configured
    let auto_subdomain = if state.config.proxy.sslip_enabled {
        // Generate sslip.io domain
        state.config.proxy.generate_sslip_domain(None)
    } else {
        state.config.proxy.generate_subdomain(&req.name)
    };

    sqlx::query(
        r#"
        INSERT INTO apps (id, name, git_url, branch, dockerfile, domain, port, healthcheck, memory_limit, cpu_limit, ssh_key_id, environment, project_id, dockerfile_path, base_directory, build_target, watch_paths, custom_docker_options, port_mappings, network_aliases, extra_hosts, domains, auto_subdomain, pre_deploy_commands, post_deploy_commands, docker_image, docker_image_tag, registry_url, registry_username, registry_password, container_labels, build_type, nixpacks_config, publish_directory, preview_enabled, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.git_url)
    .bind(&req.branch)
    .bind(&req.dockerfile)
    .bind(&req.domain)
    .bind(req.port)
    .bind(&req.healthcheck)
    .bind(&req.memory_limit)
    .bind(&req.cpu_limit)
    .bind(&req.ssh_key_id)
    .bind(req.environment.to_string())
    .bind(&req.project_id)
    .bind(&req.dockerfile_path)
    .bind(&req.base_directory)
    .bind(&req.build_target)
    .bind(&req.watch_paths)
    .bind(&req.custom_docker_options)
    .bind(&port_mappings_json)
    .bind(&network_aliases_json)
    .bind(&extra_hosts_json)
    .bind(&domains_json)
    .bind(&auto_subdomain)
    .bind(&pre_deploy_commands_json)
    .bind(&post_deploy_commands_json)
    .bind(&req.docker_image)
    .bind(&req.docker_image_tag)
    .bind(&req.registry_url)
    .bind(&req.registry_username)
    .bind(&req.registry_password)
    .bind(&container_labels_json)
    .bind(&req.build_type)
    .bind(&req.nixpacks_config)
    .bind(&req.publish_directory)
    .bind(req.preview_enabled)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create app: {}", e);
        // Check for unique constraint violation
        if e.to_string().contains("UNIQUE constraint failed") {
            ApiError::conflict("An app with this name already exists")
        } else {
            ApiError::database("Failed to create app")
        }
    })?;

    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::APP_CREATE,
        resource_types::APP,
        Some(&app.id),
        Some(&app.name),
        Some(&user.id),
        ip.as_deref(),
        Some(serde_json::json!({
            "git_url": req.git_url,
            "branch": req.branch,
            "docker_image": req.docker_image,
        })),
    )
    .await;

    Ok((StatusCode::CREATED, Json(app)))
}

/// Helper to merge optional string values
/// - None means "don't change" -> keep existing
/// - Some("") means "clear" -> set to None
/// - Some(value) means "set" -> use the value
fn merge_optional_string(new_val: &Option<String>, existing: &Option<String>) -> Option<String> {
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
fn merge_optional_json<T: serde::Serialize>(
    new_val: &Option<Vec<T>>,
    existing: &Option<String>,
) -> Option<String> {
    match new_val {
        Some(v) if v.is_empty() => None,          // Explicit clear
        Some(v) => serde_json::to_string(v).ok(), // New value
        None => existing.clone(),                 // Keep existing
    }
}

pub async fn update_app(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UpdateAppRequest>,
) -> Result<Json<App>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Validate request
    validate_update_request(&req)?;

    // Check if app exists and get current values for merging
    let existing = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    let now = chrono::Utc::now().to_rfc3339();

    // Merge all values - for optional fields, empty string means "clear"
    let name = req.name.clone().unwrap_or(existing.name.clone());
    let git_url = req.git_url.clone().unwrap_or(existing.git_url.clone());
    let branch = req.branch.clone().unwrap_or(existing.branch.clone());
    let dockerfile = req
        .dockerfile
        .clone()
        .unwrap_or(existing.dockerfile.clone());
    let port = req.port.unwrap_or(existing.port);
    let environment = req
        .environment
        .as_ref()
        .map(|e| e.to_string())
        .unwrap_or(existing.environment.clone());

    // Optional fields - can be cleared with empty string
    let domain = merge_optional_string(&req.domain, &existing.domain);
    let healthcheck = merge_optional_string(&req.healthcheck, &existing.healthcheck);
    let memory_limit = merge_optional_string(&req.memory_limit, &existing.memory_limit);
    let cpu_limit = merge_optional_string(&req.cpu_limit, &existing.cpu_limit);
    let ssh_key_id = merge_optional_string(&req.ssh_key_id, &existing.ssh_key_id);
    let project_id = merge_optional_string(&req.project_id, &existing.project_id);
    let dockerfile_path = merge_optional_string(&req.dockerfile_path, &existing.dockerfile_path);
    let base_directory = merge_optional_string(&req.base_directory, &existing.base_directory);
    let build_target = merge_optional_string(&req.build_target, &existing.build_target);
    let watch_paths = merge_optional_string(&req.watch_paths, &existing.watch_paths);
    let custom_docker_options =
        merge_optional_string(&req.custom_docker_options, &existing.custom_docker_options);

    // JSON array fields - can be cleared with empty array
    let port_mappings = merge_optional_json(&req.port_mappings, &existing.port_mappings);
    let network_aliases = merge_optional_json(&req.network_aliases, &existing.network_aliases);
    let extra_hosts = merge_optional_json(&req.extra_hosts, &existing.extra_hosts);
    let pre_deploy_commands =
        merge_optional_json(&req.pre_deploy_commands, &existing.pre_deploy_commands);
    let post_deploy_commands =
        merge_optional_json(&req.post_deploy_commands, &existing.post_deploy_commands);
    let domains = merge_optional_json(&req.domains, &existing.domains);

    // Docker Registry fields
    let docker_image = merge_optional_string(&req.docker_image, &existing.docker_image);
    let docker_image_tag = merge_optional_string(&req.docker_image_tag, &existing.docker_image_tag);
    let registry_url = merge_optional_string(&req.registry_url, &existing.registry_url);
    let registry_username =
        merge_optional_string(&req.registry_username, &existing.registry_username);
    let registry_password =
        merge_optional_string(&req.registry_password, &existing.registry_password);

    // Container labels - HashMap serialized as JSON
    let container_labels = match &req.container_labels {
        Some(labels) if labels.is_empty() => None, // Explicit clear
        Some(labels) => serde_json::to_string(labels).ok(), // New value
        None => existing.container_labels.clone(), // Keep existing
    };

    // Build type and Nixpacks fields
    let build_type = merge_optional_string(&req.build_type, &existing.build_type);
    let nixpacks_config = merge_optional_string(&req.nixpacks_config, &existing.nixpacks_config);
    let publish_directory =
        merge_optional_string(&req.publish_directory, &existing.publish_directory);
    let preview_enabled = req.preview_enabled.unwrap_or(existing.preview_enabled != 0);

    // Rollback settings
    let auto_rollback_enabled = req
        .auto_rollback_enabled
        .unwrap_or(existing.auto_rollback_enabled != 0);
    let registry_push_enabled = req
        .registry_push_enabled
        .unwrap_or(existing.registry_push_enabled != 0);
    let max_rollback_versions = req
        .max_rollback_versions
        .unwrap_or(existing.max_rollback_versions);

    sqlx::query(
        r#"
        UPDATE apps SET
            name = ?,
            git_url = ?,
            branch = ?,
            dockerfile = ?,
            domain = ?,
            port = ?,
            healthcheck = ?,
            memory_limit = ?,
            cpu_limit = ?,
            ssh_key_id = ?,
            environment = ?,
            project_id = ?,
            dockerfile_path = ?,
            base_directory = ?,
            build_target = ?,
            watch_paths = ?,
            custom_docker_options = ?,
            port_mappings = ?,
            network_aliases = ?,
            extra_hosts = ?,
            pre_deploy_commands = ?,
            post_deploy_commands = ?,
            domains = ?,
            docker_image = ?,
            docker_image_tag = ?,
            registry_url = ?,
            registry_username = ?,
            registry_password = ?,
            container_labels = ?,
            build_type = ?,
            nixpacks_config = ?,
            publish_directory = ?,
            preview_enabled = ?,
            auto_rollback_enabled = ?,
            registry_push_enabled = ?,
            max_rollback_versions = ?,
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&name)
    .bind(&git_url)
    .bind(&branch)
    .bind(&dockerfile)
    .bind(&domain)
    .bind(port)
    .bind(&healthcheck)
    .bind(&memory_limit)
    .bind(&cpu_limit)
    .bind(&ssh_key_id)
    .bind(&environment)
    .bind(&project_id)
    .bind(&dockerfile_path)
    .bind(&base_directory)
    .bind(&build_target)
    .bind(&watch_paths)
    .bind(&custom_docker_options)
    .bind(&port_mappings)
    .bind(&network_aliases)
    .bind(&extra_hosts)
    .bind(&pre_deploy_commands)
    .bind(&post_deploy_commands)
    .bind(&domains)
    .bind(&docker_image)
    .bind(&docker_image_tag)
    .bind(&registry_url)
    .bind(&registry_username)
    .bind(&registry_password)
    .bind(&container_labels)
    .bind(&build_type)
    .bind(&nixpacks_config)
    .bind(&publish_directory)
    .bind(preview_enabled)
    .bind(auto_rollback_enabled)
    .bind(registry_push_enabled)
    .bind(max_rollback_versions)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update app: {}", e);
        // Check for unique constraint violation
        if e.to_string().contains("UNIQUE constraint failed") {
            ApiError::conflict("An app with this name already exists")
        } else {
            ApiError::database("Failed to update app")
        }
    })?;

    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::APP_UPDATE,
        resource_types::APP,
        Some(&app.id),
        Some(&app.name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    Ok(Json(app))
}

/// Request to delete an app (requires password confirmation)
#[derive(serde::Deserialize)]
pub struct DeleteAppRequest {
    pub password: String,
}

pub async fn delete_app(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    user: User,
    Json(req): Json<DeleteAppRequest>,
) -> Result<StatusCode, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Verify password
    if req.password.is_empty() {
        return Err(ApiError::validation_field(
            "password",
            "Password is required".to_string(),
        ));
    }

    // For system user (API token auth), skip password verification
    if user.id != "system" {
        if !verify_password(&req.password, &user.password_hash) {
            return Err(ApiError::forbidden("Invalid password"));
        }
    }

    // Check if app exists before deleting
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Stop any running containers for this app
    let deployments: Vec<(String,)> = sqlx::query_as(
        "SELECT container_id FROM deployments WHERE app_id = ? AND container_id IS NOT NULL AND container_id != ''"
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await?;

    for (container_id,) in deployments {
        if !container_id.is_empty() {
            // Try to stop and remove the container, but don't fail if it doesn't exist
            if let Err(e) = state.runtime.stop(&container_id).await {
                tracing::warn!(container = %container_id, error = %e, "Failed to stop container during app deletion");
            }
            if let Err(e) = state.runtime.remove(&container_id).await {
                tracing::warn!(container = %container_id, error = %e, "Failed to remove container during app deletion");
            }
        }
    }

    // Remove the proxy route if app has a domain
    if let Some(domain) = &app.domain {
        if !domain.is_empty() {
            state.routes.load().remove_route(domain);
            tracing::info!(domain = %domain, "Route removed during app deletion");
        }
    }

    // Delete the app (cascades to deployments, env_vars, volumes, etc.)
    let result = sqlx::query("DELETE FROM apps WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("App not found"));
    }

    tracing::info!(app_id = %id, app_name = %app.name, "App deleted successfully");

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::APP_DELETE,
        resource_types::APP,
        Some(&app.id),
        Some(&app.name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
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
}

/// Get current running status of an app
pub async fn get_app_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<AppStatusResponse>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Check if app exists
    let _app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Get the latest running deployment
    let deployment: Option<(String,)> = sqlx::query_as(
        "SELECT container_id FROM deployments WHERE app_id = ? AND status = 'running' ORDER BY started_at DESC LIMIT 1"
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?;

    let (container_id, running, status, host_port) = if let Some((cid,)) = deployment {
        if cid.is_empty() {
            (None, false, "no_container".to_string(), None)
        } else {
            // Check if container is running
            match state.runtime.inspect(&cid).await {
                Ok(info) => (
                    Some(cid),
                    info.running,
                    if info.running { "running" } else { "stopped" }.to_string(),
                    info.host_port,
                ),
                Err(_) => (Some(cid), false, "not_found".to_string(), None),
            }
        }
    } else {
        (None, false, "not_deployed".to_string(), None)
    };

    Ok(Json(AppStatusResponse {
        app_id: id,
        container_id,
        running,
        status,
        host_port,
    }))
}

/// Start an app's container
pub async fn start_app(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<AppStatusResponse>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Check if app exists
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Get the latest running or stopped deployment with a container
    let deployment: Option<(String,)> = sqlx::query_as(
        "SELECT container_id FROM deployments WHERE app_id = ? AND status IN ('running', 'stopped') AND container_id IS NOT NULL ORDER BY started_at DESC LIMIT 1"
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?;

    let container_id = deployment
        .and_then(|(cid,)| if cid.is_empty() { None } else { Some(cid) })
        .ok_or_else(|| {
            ApiError::bad_request("No deployment with container found. Deploy the app first.")
        })?;

    // Start the container
    state.runtime.start(&container_id).await.map_err(|e| {
        tracing::error!(container = %container_id, error = %e, "Failed to start container");
        ApiError::internal(&format!("Failed to start container: {}", e))
    })?;

    tracing::info!(app = %app.name, container = %container_id, "App container started");

    // Get container info for the port (used for both routing and response)
    let host_port = state
        .runtime
        .inspect(&container_id)
        .await
        .ok()
        .and_then(|info| info.host_port);

    // Re-register the route if app has a domain
    if let Some(domain) = &app.domain {
        if !domain.is_empty() {
            if let Some(port) = host_port {
                let backend =
                    crate::proxy::Backend::new(container_id.clone(), "127.0.0.1".to_string(), port)
                        .with_healthcheck(app.healthcheck.clone());

                state.routes.load().add_route(domain.clone(), backend);
                tracing::info!(domain = %domain, "Route re-registered after start");
            }
        }
    }

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::APP_START,
        resource_types::APP,
        Some(&app.id),
        Some(&app.name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    Ok(Json(AppStatusResponse {
        app_id: id,
        container_id: Some(container_id),
        running: true,
        status: "running".to_string(),
        host_port,
    }))
}

/// Stop an app's container
pub async fn stop_app(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<AppStatusResponse>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Check if app exists
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Get the latest running deployment with a container
    let deployment: Option<(String,)> = sqlx::query_as(
        "SELECT container_id FROM deployments WHERE app_id = ? AND status = 'running' AND container_id IS NOT NULL ORDER BY started_at DESC LIMIT 1"
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?;

    let container_id = deployment
        .and_then(|(cid,)| if cid.is_empty() { None } else { Some(cid) })
        .ok_or_else(|| ApiError::bad_request("No running deployment found"))?;

    // Stop the container
    state.runtime.stop(&container_id).await.map_err(|e| {
        tracing::error!(container = %container_id, error = %e, "Failed to stop container");
        ApiError::internal(&format!("Failed to stop container: {}", e))
    })?;

    tracing::info!(app = %app.name, container = %container_id, "App container stopped");

    // Remove the route if app has a domain
    if let Some(domain) = &app.domain {
        if !domain.is_empty() {
            state.routes.load().remove_route(domain);
            tracing::info!(domain = %domain, "Route removed after stop");
        }
    }

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::APP_STOP,
        resource_types::APP,
        Some(&app.id),
        Some(&app.name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    Ok(Json(AppStatusResponse {
        app_id: id,
        container_id: Some(container_id),
        running: false,
        status: "stopped".to_string(),
        host_port: None, // Container is stopped, no port exposed
    }))
}

/// Restart an app's container.
/// This stops and starts the container, which picks up new environment variables
/// and other configuration changes without requiring a full rebuild.
pub async fn restart_app(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<AppStatusResponse>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Check if app exists
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Get the latest running or stopped deployment with a container
    let deployment: Option<(String,)> = sqlx::query_as(
        "SELECT container_id FROM deployments WHERE app_id = ? AND status IN ('running', 'stopped') AND container_id IS NOT NULL ORDER BY started_at DESC LIMIT 1"
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?;

    let container_id = deployment
        .and_then(|(cid,)| if cid.is_empty() { None } else { Some(cid) })
        .ok_or_else(|| {
            ApiError::bad_request("No deployment with container found. Deploy the app first.")
        })?;

    // First stop the container (ignore errors if already stopped)
    let _ = state.runtime.stop(&container_id).await;

    // Brief pause to ensure clean stop
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Start the container
    state.runtime.start(&container_id).await.map_err(|e| {
        tracing::error!(container = %container_id, error = %e, "Failed to restart container");
        ApiError::internal(&format!("Failed to restart container: {}", e))
    })?;

    tracing::info!(app = %app.name, container = %container_id, "App container restarted");

    // Get container info for the port (used for both routing and response)
    let host_port = state
        .runtime
        .inspect(&container_id)
        .await
        .ok()
        .and_then(|info| info.host_port);

    // Re-register the route if app has a domain
    if let Some(domain) = &app.domain {
        if !domain.is_empty() {
            if let Some(port) = host_port {
                let backend =
                    crate::proxy::Backend::new(container_id.clone(), "127.0.0.1".to_string(), port)
                        .with_healthcheck(app.healthcheck.clone());

                state.routes.load().add_route(domain.clone(), backend);
                tracing::info!(domain = %domain, port = port, "Route re-registered after restart");
            }
        }
    }

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::APP_RESTART,
        resource_types::APP,
        Some(&app.id),
        Some(&app.name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    Ok(Json(AppStatusResponse {
        app_id: id,
        container_id: Some(container_id),
        running: true,
        status: "running".to_string(),
        host_port,
    }))
}

// -------------------------------------------------------------------------
// Upload-based App Creation
// -------------------------------------------------------------------------

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

fn default_port() -> u16 {
    3000
}

fn default_cpu_limit() -> String {
    "1".to_string()
}

fn default_memory_limit() -> String {
    "512m".to_string()
}

fn default_environment() -> String {
    "development".to_string()
}

/// Response for upload app creation
#[derive(Serialize)]
pub struct UploadAppResponse {
    pub app: App,
    pub deployment_id: String,
    pub detected_build_type: BuildDetectionResult,
}

/// Create an app and deploy from uploaded ZIP file
/// POST /api/projects/:project_id/apps/upload
pub async fn upload_create_app(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<UploadAppResponse>), ApiError> {
    // Validate project_id format
    if let Err(e) = validate_uuid(&project_id, "project_id") {
        return Err(ApiError::validation_field("project_id", e));
    }

    // Verify project exists
    let _project: Option<(String,)> = sqlx::query_as("SELECT id FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_optional(&state.db)
        .await?;

    if _project.is_none() {
        return Err(ApiError::not_found("Project not found"));
    }

    // Parse multipart form data
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut config: Option<UploadAppConfig> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(&format!("Failed to read multipart: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            file_name = field.file_name().map(|s| s.to_string());
            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::bad_request(&format!("Failed to read file: {}", e)))?;
            file_data = Some(data.to_vec());
        } else if name == "config" {
            let text = field
                .text()
                .await
                .map_err(|e| ApiError::bad_request(&format!("Failed to read config: {}", e)))?;
            config = Some(
                serde_json::from_str(&text)
                    .map_err(|e| ApiError::bad_request(&format!("Invalid config JSON: {}", e)))?,
            );
        }
    }

    let file_data = file_data.ok_or_else(|| ApiError::bad_request("No file uploaded"))?;
    let config = config.ok_or_else(|| ApiError::bad_request("No config provided"))?;

    // Validate file is a ZIP
    if let Some(ref name) = file_name {
        if !name.to_lowercase().ends_with(".zip") {
            return Err(ApiError::bad_request("Only ZIP files are supported"));
        }
    }

    // Validate app name
    if let Err(e) = validate_app_name(&config.name) {
        return Err(ApiError::validation_field("name", e));
    }

    // Create a unique deployment ID for the temp directory
    let deployment_id = Uuid::new_v4().to_string();
    let work_dir = std::env::temp_dir().join(format!("rivetr-upload-{}", deployment_id));

    // Extract ZIP and find project root
    let project_root = extract_zip_and_find_root(&file_data, &work_dir)
        .await
        .map_err(|e| ApiError::bad_request(&format!("Failed to extract ZIP: {}", e)))?;

    // Auto-detect build type
    let detected = detect_build_type(&project_root)
        .await
        .map_err(|e| ApiError::internal(&format!("Failed to detect build type: {}", e)))?;
    tracing::info!(
        build_type = %detected.build_type,
        confidence = %detected.confidence,
        detected_from = %detected.detected_from,
        "Build type detected for uploaded project"
    );

    // Determine final build type (use override or detected)
    let build_type = config
        .build_type
        .clone()
        .unwrap_or_else(|| detected.build_type.to_string());
    let publish_directory = config
        .publish_directory
        .clone()
        .or_else(|| detected.publish_directory.clone());

    // Clone detected for audit log before moving
    let detected_from_log = detected.detected_from.clone();

    // Create the app
    let app_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO apps (
            id, name, git_url, branch, dockerfile, domain, port, healthcheck,
            memory_limit, cpu_limit, environment, project_id, build_type,
            publish_directory, deployment_source, created_at, updated_at
        ) VALUES (?, ?, '', 'main', 'Dockerfile', ?, ?, ?, ?, ?, ?, ?, ?, ?, 'upload', ?, ?)
        "#,
    )
    .bind(&app_id)
    .bind(&config.name)
    .bind(&config.domain)
    .bind(config.port as i32)
    .bind(&config.healthcheck)
    .bind(&config.memory_limit)
    .bind(&config.cpu_limit)
    .bind(&config.environment)
    .bind(&project_id)
    .bind(&build_type)
    .bind(&publish_directory)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create app: {}", e);
        if e.to_string().contains("UNIQUE constraint failed") {
            ApiError::conflict("An app with this name already exists")
        } else {
            ApiError::database("Failed to create app")
        }
    })?;

    // Create deployment record
    sqlx::query(
        r#"
        INSERT INTO deployments (id, app_id, status, started_at, commit_sha)
        VALUES (?, ?, 'pending', ?, ?)
        "#,
    )
    .bind(&deployment_id)
    .bind(&app_id)
    .bind(&now)
    .bind(project_root.to_string_lossy().to_string()) // Store source path in commit_sha
    .execute(&state.db)
    .await?;

    // Fetch the created app
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_one(&state.db)
        .await?;

    // Queue the deployment
    if let Err(e) = state
        .deploy_tx
        .send((deployment_id.clone(), app.clone()))
        .await
    {
        tracing::error!("Failed to queue deployment: {}", e);
        return Err(ApiError::internal("Failed to queue deployment"));
    }

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::APP_CREATE,
        resource_types::APP,
        Some(&app.id),
        Some(&app.name),
        Some(&user.id),
        ip.as_deref(),
        Some(serde_json::json!({
            "source": "upload",
            "build_type": build_type,
            "detected_from": detected_from_log
        })),
    )
    .await;

    tracing::info!(
        app_id = %app_id,
        deployment_id = %deployment_id,
        build_type = %build_type,
        "App created from upload and deployment queued"
    );

    Ok((
        StatusCode::CREATED,
        Json(UploadAppResponse {
            app,
            deployment_id,
            detected_build_type: detected,
        }),
    ))
}

/// Stream runtime logs for an app via SSE
/// GET /api/apps/:id/logs/stream
pub async fn stream_app_logs(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    _user: User, // Require authentication
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, (StatusCode, Json<serde_json::Value>)>
{
    // Validate app_id
    validate_uuid(&app_id, "app_id").map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e})),
        )
    })?;

    // Find the latest running deployment for this app
    let deployment = sqlx::query_as::<_, Deployment>(
        "SELECT * FROM deployments WHERE app_id = ? AND status = 'running' ORDER BY started_at DESC LIMIT 1",
    )
    .bind(&app_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Database error fetching deployment: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Database error"})),
        )
    })?;

    let deployment = deployment.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "No running container found for this app"})),
        )
    })?;

    let container_id = deployment.container_id.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "No container ID found for this deployment"})),
        )
    })?;

    tracing::info!(app_id = %app_id, container_id = %container_id, "Starting log stream for app");

    // Start docker logs with --follow
    let mut cmd = Command::new("docker");
    cmd.arg("logs")
        .arg("--follow")
        .arg("--timestamps")
        .arg("--tail")
        .arg("100") // Start with last 100 lines
        .arg(&container_id)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        tracing::error!("Failed to spawn docker logs: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to start log stream: {}", e)})),
        )
    })?;

    let stdout = child.stdout.take().ok_or_else(|| {
        tracing::error!("Failed to get stdout from docker logs");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to get log output stream"})),
        )
    })?;

    let stderr = child.stderr.take();

    let container_id_clone = container_id.clone();

    // Create the SSE stream using async_stream
    let stream = async_stream::stream! {
        // Send connected message first
        let connected_msg = serde_json::json!({
            "type": "connected",
            "container_id": container_id_clone,
        });
        yield Ok(Event::default().data(connected_msg.to_string()));

        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        // Also read stderr in parallel
        let stderr_task = if let Some(stderr) = stderr {
            let stderr_reader = BufReader::new(stderr);
            Some(tokio::spawn(async move {
                let mut stderr_lines = stderr_reader.lines();
                let mut stderr_msgs = Vec::new();
                while let Ok(Some(line)) = stderr_lines.next_line().await {
                    stderr_msgs.push(line);
                }
                stderr_msgs
            }))
        } else {
            None
        };

        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    // Parse docker log line format: 2024-01-01T00:00:00.000000000Z message
                    let (timestamp, message) = if let Some(idx) = line.find(' ') {
                        let ts = &line[..idx];
                        let msg = &line[idx + 1..];
                        (Some(ts.to_string()), msg.to_string())
                    } else {
                        (None, line)
                    };

                    let log_entry = serde_json::json!({
                        "type": "log",
                        "timestamp": timestamp,
                        "message": message,
                        "stream": "stdout",
                    });
                    yield Ok(Event::default().data(log_entry.to_string()));
                }
                Ok(None) => {
                    // Stream ended - container stopped or exited
                    let end_msg = serde_json::json!({
                        "type": "end",
                        "message": "Log stream ended"
                    });
                    yield Ok(Event::default().data(end_msg.to_string()));
                    break;
                }
                Err(e) => {
                    tracing::warn!("Error reading log line: {}", e);
                    let error_msg = serde_json::json!({
                        "type": "error",
                        "message": format!("{}", e)
                    });
                    yield Ok(Event::default().data(error_msg.to_string()));
                    break;
                }
            }
        }

        // Cleanup the stderr task
        if let Some(task) = stderr_task {
            let _ = task.await;
        }
    };

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}
