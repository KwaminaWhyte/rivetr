use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{App, CreateAppRequest, UpdateAppRequest};
use crate::AppState;

use super::error::{ApiError, ValidationErrorBuilder};
use super::validation::{
    validate_app_name, validate_base_directory, validate_branch, validate_build_target,
    validate_cpu_limit, validate_custom_docker_options, validate_deployment_commands,
    validate_dockerfile, validate_domain, validate_domains, validate_environment,
    validate_extra_hosts, validate_git_url, validate_healthcheck, validate_memory_limit,
    validate_network_aliases, validate_port, validate_port_mappings, validate_uuid,
    validate_watch_paths,
};

/// Validate a CreateAppRequest
fn validate_create_request(req: &CreateAppRequest) -> Result<(), ApiError> {
    let mut errors = ValidationErrorBuilder::new();

    if let Err(e) = validate_app_name(&req.name) {
        errors.add("name", &e);
    }

    if let Err(e) = validate_git_url(&req.git_url) {
        errors.add("git_url", &e);
    }

    if let Err(e) = validate_branch(&req.branch) {
        errors.add("branch", &e);
    }

    if let Err(e) = validate_dockerfile(&req.dockerfile) {
        errors.add("dockerfile", &e);
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

    if let Some(ref git_url) = req.git_url {
        if let Err(e) = validate_git_url(git_url) {
            errors.add("git_url", &e);
        }
    }

    if let Some(ref branch) = req.branch {
        if let Err(e) = validate_branch(branch) {
            errors.add("branch", &e);
        }
    }

    if let Some(ref dockerfile) = req.dockerfile {
        if let Err(e) = validate_dockerfile(dockerfile) {
            errors.add("dockerfile", &e);
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

    // Generate auto_subdomain if configured
    let auto_subdomain = state
        .config
        .proxy
        .generate_subdomain(&req.name);

    sqlx::query(
        r#"
        INSERT INTO apps (id, name, git_url, branch, dockerfile, domain, port, healthcheck, memory_limit, cpu_limit, ssh_key_id, environment, project_id, dockerfile_path, base_directory, build_target, watch_paths, custom_docker_options, port_mappings, network_aliases, extra_hosts, domains, auto_subdomain, pre_deploy_commands, post_deploy_commands, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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

    // Check if app exists
    let _existing = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    let now = chrono::Utc::now().to_rfc3339();

    // Convert environment enum to string for binding
    let environment_str = req.environment.as_ref().map(|e| e.to_string());

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
    let pre_deploy_commands_json = req
        .pre_deploy_commands
        .as_ref()
        .map(|c| serde_json::to_string(c).unwrap_or_default());
    let post_deploy_commands_json = req
        .post_deploy_commands
        .as_ref()
        .map(|c| serde_json::to_string(c).unwrap_or_default());
    let domains_json = req
        .domains
        .as_ref()
        .map(|d| serde_json::to_string(d).unwrap_or_default());

    sqlx::query(
        r#"
        UPDATE apps SET
            name = COALESCE(?, name),
            git_url = COALESCE(?, git_url),
            branch = COALESCE(?, branch),
            dockerfile = COALESCE(?, dockerfile),
            domain = COALESCE(?, domain),
            port = COALESCE(?, port),
            healthcheck = COALESCE(?, healthcheck),
            memory_limit = COALESCE(?, memory_limit),
            cpu_limit = COALESCE(?, cpu_limit),
            ssh_key_id = COALESCE(?, ssh_key_id),
            environment = COALESCE(?, environment),
            project_id = COALESCE(?, project_id),
            dockerfile_path = COALESCE(?, dockerfile_path),
            base_directory = COALESCE(?, base_directory),
            build_target = COALESCE(?, build_target),
            watch_paths = COALESCE(?, watch_paths),
            custom_docker_options = COALESCE(?, custom_docker_options),
            port_mappings = COALESCE(?, port_mappings),
            network_aliases = COALESCE(?, network_aliases),
            extra_hosts = COALESCE(?, extra_hosts),
            pre_deploy_commands = COALESCE(?, pre_deploy_commands),
            post_deploy_commands = COALESCE(?, post_deploy_commands),
            domains = COALESCE(?, domains),
            updated_at = ?
        WHERE id = ?
        "#,
    )
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
    .bind(&environment_str)
    .bind(&req.project_id)
    .bind(&req.dockerfile_path)
    .bind(&req.base_directory)
    .bind(&req.build_target)
    .bind(&req.watch_paths)
    .bind(&req.custom_docker_options)
    .bind(&port_mappings_json)
    .bind(&network_aliases_json)
    .bind(&extra_hosts_json)
    .bind(&pre_deploy_commands_json)
    .bind(&post_deploy_commands_json)
    .bind(&domains_json)
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

pub async fn delete_app(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    let result = sqlx::query("DELETE FROM apps WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("App not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}
