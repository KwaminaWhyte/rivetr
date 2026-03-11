use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{
    actions, resource_types, App, CreateAppRequest, TeamAuditAction, TeamAuditResourceType,
    UpdateAppRequest, User,
};
use crate::AppState;

use super::super::audit::{audit_log, extract_client_ip};
use super::super::auth::verify_password;
use super::super::error::ApiError;
use super::super::teams::log_team_audit;
use super::super::validation::validate_uuid;
use super::{
    merge_optional_json, merge_optional_string, validate_create_request, validate_update_request,
    DeleteAppRequest, ListAppsQuery,
};

pub async fn list_apps(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListAppsQuery>,
) -> Result<Json<Vec<App>>, ApiError> {
    let apps = if let Some(team_id) = &query.team_id {
        // Filter by team ID - returns apps belonging to the specified team
        // Also include apps with NULL team_id if team_id is empty (for backward compatibility)
        if team_id.is_empty() {
            // Return apps without a team (legacy/unassigned apps)
            sqlx::query_as::<_, App>(
                "SELECT * FROM apps WHERE team_id IS NULL ORDER BY created_at DESC",
            )
            .fetch_all(&state.db)
            .await?
        } else {
            // Return apps belonging to the specified team
            sqlx::query_as::<_, App>(
                "SELECT * FROM apps WHERE team_id = ? ORDER BY created_at DESC",
            )
            .bind(team_id)
            .fetch_all(&state.db)
            .await?
        }
    } else {
        // No team filter - return all apps (for backward compatibility)
        sqlx::query_as::<_, App>("SELECT * FROM apps ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await?
    };

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
    let nixpacks_config_json = req
        .nixpacks_config
        .as_ref()
        .map(|v| v.to_string());

    // Generate auto_subdomain: sslip (legacy) → base_domain → traefik.me → None
    let auto_subdomain = if state.config.proxy.sslip_enabled {
        state.config.proxy.generate_sslip_domain(None)
    } else {
        state.config.proxy.generate_auto_domain(&req.name)
    };

    sqlx::query(
        r#"
        INSERT INTO apps (id, name, git_url, branch, dockerfile, domain, port, healthcheck, memory_limit, cpu_limit, ssh_key_id, environment, project_id, team_id, dockerfile_path, base_directory, build_target, watch_paths, custom_docker_options, port_mappings, network_aliases, extra_hosts, domains, auto_subdomain, pre_deploy_commands, post_deploy_commands, docker_image, docker_image_tag, registry_url, registry_username, registry_password, container_labels, build_type, nixpacks_config, publish_directory, preview_enabled, git_provider_id, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
    .bind(&req.team_id)
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
    .bind(&nixpacks_config_json)
    .bind(&req.publish_directory)
    .bind(req.preview_enabled)
    .bind(&req.git_provider_id)
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

    // Log team audit event if app belongs to a team
    if let Some(ref team_id) = app.team_id {
        if let Err(e) = log_team_audit(
            &state.db,
            team_id,
            Some(&user.id),
            TeamAuditAction::AppCreated,
            TeamAuditResourceType::App,
            Some(&app.id),
            Some(serde_json::json!({
                "app_name": app.name,
                "git_url": req.git_url,
                "branch": req.branch,
            })),
        )
        .await
        {
            tracing::warn!("Failed to log team audit event: {}", e);
        }
    }

    Ok((StatusCode::CREATED, Json(app)))
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
    let nixpacks_config = match &req.nixpacks_config {
        Some(v) => Some(v.to_string()),
        None => existing.nixpacks_config.clone(),
    };
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
    let rollback_retention_count = req
        .rollback_retention_count
        .unwrap_or(existing.rollback_retention_count)
        .clamp(1, 50);

    // Approval and maintenance settings
    let require_approval = req
        .require_approval
        .unwrap_or(existing.require_approval != 0);
    let maintenance_mode = req
        .maintenance_mode
        .unwrap_or(existing.maintenance_mode != 0);
    let maintenance_message = req
        .maintenance_message
        .as_ref()
        .cloned()
        .or_else(|| existing.maintenance_message.clone());

    // Server assignment - empty string clears the assignment
    let server_id = merge_optional_string(&req.server_id, &existing.server_id);

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
            require_approval = ?,
            maintenance_mode = ?,
            maintenance_message = ?,
            server_id = ?,
            rollback_retention_count = ?,
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
    .bind(require_approval)
    .bind(maintenance_mode)
    .bind(&maintenance_message)
    .bind(&server_id)
    .bind(rollback_retention_count)
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

    // Re-register proxy routes if the app is currently running and has domains
    {
        let running: Option<(String, Option<String>)> = sqlx::query_as(
            "SELECT container_id, image_tag FROM deployments WHERE app_id = ? AND status = 'running' ORDER BY started_at DESC LIMIT 1"
        )
        .bind(&app.id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

        if let Some((container_id, _)) = running {
            if let Ok(info) = state.runtime.inspect(&container_id).await {
                if let Some(port) = info.port {
                    let all_domains = app.get_all_domain_names();
                    let route_table = state.routes.load();
                    for domain in &all_domains {
                        let backend = crate::proxy::Backend::new(
                            container_id.clone(),
                            "127.0.0.1".to_string(),
                            port,
                        )
                        .with_healthcheck(app.healthcheck.clone());
                        route_table.add_route(domain.clone(), backend);
                    }
                    if !all_domains.is_empty() {
                        tracing::info!(domains = ?all_domains, "Proxy routes updated after app settings change");
                    }
                }
            }
        }
    }

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

    // Log team audit event if app belongs to a team
    if let Some(ref team_id) = app.team_id {
        if let Err(e) = log_team_audit(
            &state.db,
            team_id,
            Some(&user.id),
            TeamAuditAction::AppUpdated,
            TeamAuditResourceType::App,
            Some(&app.id),
            Some(serde_json::json!({
                "app_name": app.name,
            })),
        )
        .await
        {
            tracing::warn!("Failed to log team audit event: {}", e);
        }
    }

    Ok(Json(app))
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
    if user.id != "system" && !verify_password(&req.password, &user.password_hash) {
        return Err(ApiError::forbidden("Invalid password"));
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

    // Log team audit event if app belonged to a team
    if let Some(ref team_id) = app.team_id {
        if let Err(e) = log_team_audit(
            &state.db,
            team_id,
            Some(&user.id),
            TeamAuditAction::AppDeleted,
            TeamAuditResourceType::App,
            Some(&app.id),
            Some(serde_json::json!({
                "app_name": app.name,
            })),
        )
        .await
        {
            tracing::warn!("Failed to log team audit event: {}", e);
        }
    }

    Ok(StatusCode::NO_CONTENT)
}
