use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{App, CreateAppRequest, UpdateAppRequest, User};
use crate::AppState;

use super::auth::verify_password;

use super::error::{ApiError, ValidationErrorBuilder};
use super::validation::{
    validate_app_name, validate_base_directory, validate_branch, validate_build_target,
    validate_cpu_limit, validate_custom_docker_options, validate_deployment_commands,
    validate_docker_image, validate_dockerfile, validate_domain, validate_domains,
    validate_environment, validate_extra_hosts, validate_git_url, validate_healthcheck,
    validate_memory_limit, validate_network_aliases, validate_port, validate_port_mappings,
    validate_uuid, validate_watch_paths,
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
        errors.add(
            "git_url",
            "Either git_url or docker_image must be provided",
        );
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

    errors.finish()
}

pub async fn list_apps(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<App>>, ApiError> {
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
        INSERT INTO apps (id, name, git_url, branch, dockerfile, domain, port, healthcheck, memory_limit, cpu_limit, ssh_key_id, environment, project_id, dockerfile_path, base_directory, build_target, watch_paths, custom_docker_options, port_mappings, network_aliases, extra_hosts, domains, auto_subdomain, pre_deploy_commands, post_deploy_commands, docker_image, docker_image_tag, registry_url, registry_username, registry_password, container_labels, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        Some(v) if v.is_empty() => None, // Explicit clear
        Some(v) => serde_json::to_string(v).ok(), // New value
        None => existing.clone(), // Keep existing
    }
}

pub async fn update_app(
    State(state): State<Arc<AppState>>,
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
    let dockerfile = req.dockerfile.clone().unwrap_or(existing.dockerfile.clone());
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

    Ok(Json(app))
}

/// Request to delete an app (requires password confirmation)
#[derive(serde::Deserialize)]
pub struct DeleteAppRequest {
    pub password: String,
}

pub async fn delete_app(
    State(state): State<Arc<AppState>>,
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
        return Err(ApiError::validation_field("password", "Password is required".to_string()));
    }

    // For system user (API token auth), skip password verification
    if user.id != "system" {
        if !verify_password(&req.password, &user.password_hash) {
            return Err(ApiError::forbidden("Invalid password"));
        }
    }

    // Check if app exists before deleting
    let _app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    let result = sqlx::query("DELETE FROM apps WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("App not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Response for app status
#[derive(serde::Serialize)]
pub struct AppStatusResponse {
    pub app_id: String,
    pub container_id: Option<String>,
    pub running: bool,
    pub status: String,
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

    let (container_id, running, status) = if let Some((cid,)) = deployment {
        if cid.is_empty() {
            (None, false, "no_container".to_string())
        } else {
            // Check if container is running
            match state.runtime.inspect(&cid).await {
                Ok(info) => (Some(cid), info.running, if info.running { "running" } else { "stopped" }.to_string()),
                Err(_) => (Some(cid), false, "not_found".to_string()),
            }
        }
    } else {
        (None, false, "not_deployed".to_string())
    };

    Ok(Json(AppStatusResponse {
        app_id: id,
        container_id,
        running,
        status,
    }))
}

/// Start an app's container
pub async fn start_app(
    State(state): State<Arc<AppState>>,
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
        .ok_or_else(|| ApiError::bad_request("No deployment with container found. Deploy the app first."))?;

    // Start the container
    state.runtime.start(&container_id).await.map_err(|e| {
        tracing::error!(container = %container_id, error = %e, "Failed to start container");
        ApiError::internal(&format!("Failed to start container: {}", e))
    })?;

    tracing::info!(app = %app.name, container = %container_id, "App container started");

    // Re-register the route if app has a domain
    if let Some(domain) = &app.domain {
        if !domain.is_empty() {
            // Get container info for the port
            if let Ok(info) = state.runtime.inspect(&container_id).await {
                if let Some(host_port) = info.host_port {
                    let backend = crate::proxy::Backend::new(
                        container_id.clone(),
                        "127.0.0.1".to_string(),
                        host_port,
                    )
                    .with_healthcheck(app.healthcheck.clone());

                    state.routes.load().add_route(domain.clone(), backend);
                    tracing::info!(domain = %domain, "Route re-registered after start");
                }
            }
        }
    }

    Ok(Json(AppStatusResponse {
        app_id: id,
        container_id: Some(container_id),
        running: true,
        status: "running".to_string(),
    }))
}

/// Stop an app's container
pub async fn stop_app(
    State(state): State<Arc<AppState>>,
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

    Ok(Json(AppStatusResponse {
        app_id: id,
        container_id: Some(container_id),
        running: false,
        status: "stopped".to_string(),
    }))
}
