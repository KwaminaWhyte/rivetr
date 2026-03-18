//! Environment clone API endpoint.
//!
//! POST /api/projects/:project_id/environments/:env_id/clone
//!
//! Duplicates an environment and all its associated resources (apps, env vars,
//! volumes, databases, services) with fresh IDs and cleared deployment state.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{App, ManagedDatabase, ProjectEnvironment, Service};
use crate::AppState;

use super::error::ApiError;
use super::validation::validate_uuid;

/// Request body for cloning an environment
#[derive(Debug, Deserialize)]
pub struct CloneEnvironmentRequest {
    /// Name for the new (cloned) environment
    pub name: String,
}

/// Response for a successful clone — the newly created environment
#[derive(Debug, Serialize)]
pub struct CloneEnvironmentResponse {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
    /// Summary counts of what was cloned
    pub cloned_apps: usize,
    pub cloned_databases: usize,
    pub cloned_services: usize,
}

/// POST /api/projects/:project_id/environments/:env_id/clone
///
/// Creates a new environment that is a copy of the source environment.
/// All apps (with their env vars and volumes), databases, and services are
/// duplicated with new IDs. Deployment state (container IDs, status) is reset.
/// Domains are cleared so they do not conflict.
pub async fn clone_environment(
    State(state): State<Arc<AppState>>,
    Path((project_id, env_id)): Path<(String, String)>,
    Json(req): Json<CloneEnvironmentRequest>,
) -> Result<(StatusCode, Json<CloneEnvironmentResponse>), ApiError> {
    // Validate path params
    if let Err(e) = validate_uuid(&project_id, "project_id") {
        return Err(ApiError::validation_field("project_id", e));
    }
    if let Err(e) = validate_uuid(&env_id, "environment_id") {
        return Err(ApiError::validation_field("environment_id", e));
    }

    // Validate new name
    let new_name = req.name.trim().to_string();
    if new_name.is_empty() {
        return Err(ApiError::validation_field("name", "Environment name is required"));
    }
    if new_name.len() > 50 {
        return Err(ApiError::validation_field(
            "name",
            "Environment name is too long (max 50 characters)",
        ));
    }

    // Verify project exists
    let project_exists =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM projects WHERE id = ?")
            .bind(&project_id)
            .fetch_one(&state.db)
            .await?;
    if project_exists == 0 {
        return Err(ApiError::not_found("Project not found"));
    }

    // Fetch source environment
    let source_env =
        sqlx::query_as::<_, ProjectEnvironment>("SELECT * FROM environments WHERE id = ? AND project_id = ?")
            .bind(&env_id)
            .bind(&project_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| ApiError::not_found("Environment not found"))?;

    let now = chrono::Utc::now().to_rfc3339();
    let new_env_id = Uuid::new_v4().to_string();

    // -----------------------------------------------------------------------
    // 1. Create the new environment record
    // -----------------------------------------------------------------------
    sqlx::query(
        r#"
        INSERT INTO environments (id, project_id, name, description, is_default, created_at, updated_at)
        VALUES (?, ?, ?, ?, 0, ?, ?)
        "#,
    )
    .bind(&new_env_id)
    .bind(&project_id)
    .bind(&new_name)
    .bind(&source_env.description)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint failed") {
            ApiError::conflict("An environment with this name already exists in this project")
        } else {
            tracing::error!("Failed to create cloned environment: {}", e);
            ApiError::database("Failed to create cloned environment")
        }
    })?;

    // -----------------------------------------------------------------------
    // 2. Clone environment-level env vars
    // -----------------------------------------------------------------------
    let env_vars = sqlx::query_as::<_, EnvironmentEnvVarRow>(
        "SELECT * FROM environment_env_vars WHERE environment_id = ?",
    )
    .bind(&env_id)
    .fetch_all(&state.db)
    .await?;

    for var in &env_vars {
        let new_var_id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO environment_env_vars (id, environment_id, key, value, is_secret, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&new_var_id)
        .bind(&new_env_id)
        .bind(&var.key)
        .bind(&var.value)
        .bind(var.is_secret)
        .bind(&now)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to clone environment env var: {}", e);
            ApiError::database("Failed to clone environment variable")
        })?;
    }

    // -----------------------------------------------------------------------
    // 3. Clone apps from the source environment
    // -----------------------------------------------------------------------
    let apps = sqlx::query_as::<_, App>(
        "SELECT * FROM apps WHERE environment_id = ? AND project_id = ?",
    )
    .bind(&env_id)
    .bind(&project_id)
    .fetch_all(&state.db)
    .await?;

    let cloned_apps = apps.len();

    for app in &apps {
        let new_app_id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO apps (
                id, name, git_url, branch, dockerfile, domain, port, healthcheck,
                memory_limit, cpu_limit, ssh_key_id, environment, project_id,
                environment_id, team_id, dockerfile_path, base_directory, build_target,
                watch_paths, custom_docker_options, port_mappings, network_aliases,
                extra_hosts, domains, auto_subdomain, pre_deploy_commands,
                post_deploy_commands, docker_image, docker_image_tag, registry_url,
                registry_username, registry_password, container_labels, build_type,
                nixpacks_config, publish_directory, preview_enabled,
                github_app_installation_id, restart_policy, privileged, cap_add,
                devices, shm_size, init_process, build_platforms, build_secrets,
                docker_cap_drop, docker_gpus, docker_ulimits, docker_security_opt,
                require_approval, maintenance_mode, maintenance_message,
                auto_rollback_enabled, registry_push_enabled, max_rollback_versions,
                replica_count, server_id, build_server_id, rollback_retention_count,
                deployment_source, created_at, updated_at
            )
            VALUES (
                ?, ?, ?, ?, ?, NULL, ?, ?,
                ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?,
                ?, ?, ?, ?,
                ?, NULL, NULL, ?,
                ?, ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?
            )
            "#,
        )
        // Core identity
        .bind(&new_app_id)
        .bind(&app.name)
        .bind(&app.git_url)
        .bind(&app.branch)
        .bind(&app.dockerfile)
        // domain cleared (NULL above)
        .bind(app.port)
        .bind(&app.healthcheck)
        // Resources
        .bind(&app.memory_limit)
        .bind(&app.cpu_limit)
        .bind(&app.ssh_key_id)
        .bind(&app.environment)
        .bind(&app.project_id)
        // Environment
        .bind(&new_env_id)
        .bind(&app.team_id)
        // Build options
        .bind(&app.dockerfile_path)
        .bind(&app.base_directory)
        .bind(&app.build_target)
        .bind(&app.watch_paths)
        .bind(&app.custom_docker_options)
        // Network
        .bind(&app.port_mappings)
        .bind(&app.network_aliases)
        .bind(&app.extra_hosts)
        // domains JSON cleared (NULL above), auto_subdomain cleared (NULL above)
        // Deploy commands
        .bind(&app.pre_deploy_commands)
        .bind(&app.post_deploy_commands)
        // Registry
        .bind(&app.docker_image)
        .bind(&app.docker_image_tag)
        .bind(&app.registry_url)
        .bind(&app.registry_username)
        .bind(&app.registry_password)
        // Build config
        .bind(&app.container_labels)
        .bind(&app.build_type)
        .bind(&app.nixpacks_config)
        .bind(&app.publish_directory)
        .bind(app.preview_enabled)
        .bind(&app.github_app_installation_id)
        // Runtime options
        .bind(&app.restart_policy)
        .bind(app.privileged)
        .bind(&app.cap_add)
        .bind(&app.devices)
        .bind(&app.shm_size)
        .bind(app.init_process)
        // Build extras
        .bind(&app.build_platforms)
        .bind(&app.build_secrets)
        .bind(&app.docker_cap_drop)
        .bind(&app.docker_gpus)
        .bind(&app.docker_ulimits)
        .bind(&app.docker_security_opt)
        // Deployment policy
        .bind(app.require_approval)
        .bind(app.maintenance_mode)
        .bind(&app.maintenance_message)
        .bind(app.auto_rollback_enabled)
        .bind(app.registry_push_enabled)
        .bind(app.max_rollback_versions)
        .bind(app.replica_count)
        .bind(&app.server_id)
        .bind(&app.build_server_id)
        .bind(app.rollback_retention_count)
        .bind(&app.deployment_source)
        .bind(&now)
        .bind(&now)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to clone app {}: {}", app.id, e);
            ApiError::database("Failed to clone app")
        })?;

        // Clone app env vars
        let app_env_vars = sqlx::query_as::<_, AppEnvVarRow>(
            "SELECT * FROM env_vars WHERE app_id = ?",
        )
        .bind(&app.id)
        .fetch_all(&state.db)
        .await?;

        for ev in &app_env_vars {
            let new_ev_id = Uuid::new_v4().to_string();
            sqlx::query(
                r#"
                INSERT INTO env_vars (id, app_id, key, value, is_secret, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&new_ev_id)
            .bind(&new_app_id)
            .bind(&ev.key)
            .bind(&ev.value)
            .bind(ev.is_secret)
            .bind(&now)
            .bind(&now)
            .execute(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to clone env var for app {}: {}", app.id, e);
                ApiError::database("Failed to clone app environment variable")
            })?;
        }

        // Clone volumes
        let volumes = sqlx::query_as::<_, VolumeRow>(
            "SELECT * FROM volumes WHERE app_id = ?",
        )
        .bind(&app.id)
        .fetch_all(&state.db)
        .await?;

        for vol in &volumes {
            let new_vol_id = Uuid::new_v4().to_string();
            sqlx::query(
                r#"
                INSERT INTO volumes (id, app_id, name, host_path, container_path, read_only, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&new_vol_id)
            .bind(&new_app_id)
            .bind(&vol.name)
            .bind(&vol.host_path)
            .bind(&vol.container_path)
            .bind(vol.read_only)
            .bind(&now)
            .bind(&now)
            .execute(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to clone volume for app {}: {}", app.id, e);
                ApiError::database("Failed to clone app volume")
            })?;
        }
    }

    // -----------------------------------------------------------------------
    // 4. Clone databases
    //    Note: databases are project-scoped, not environment-scoped in the DB
    //    schema. We check if the source environment has apps that use DBs, but
    //    since databases only have project_id we clone all project databases
    //    that are associated with apps in the source environment.
    //    Actually the task says clone databases "in the environment", which
    //    means project-level databases scoped to this project.
    //    We clone ALL project databases since there is no environment_id on
    //    databases. Fresh container_id (NULL) and status = 'pending'.
    // -----------------------------------------------------------------------
    let databases = sqlx::query_as::<_, ManagedDatabase>(
        "SELECT * FROM databases WHERE project_id = ?",
    )
    .bind(&project_id)
    .fetch_all(&state.db)
    .await?;

    let cloned_databases = databases.len();

    for db in &databases {
        let new_db_id = Uuid::new_v4().to_string();
        let container_slug = ManagedDatabase::build_slug(&new_db_id);

        // Build a new volume path under the new ID to avoid path conflicts
        let volume_dir = format!("{}-{}", db.name, &new_db_id[..8.min(new_db_id.len())]);
        let new_volume_path = db.volume_path.as_ref().and_then(|old_path| {
            // Replace the last path segment (old volume_dir) with the new one
            std::path::Path::new(old_path)
                .parent()
                .map(|parent| parent.join(&volume_dir).to_string_lossy().to_string())
        });

        sqlx::query(
            r#"
            INSERT INTO databases (
                id, name, db_type, version, status, internal_port, external_port,
                public_access, credentials, volume_name, volume_path, memory_limit,
                cpu_limit, project_id, team_id, created_at, updated_at, container_slug,
                custom_image, init_commands
            ) VALUES (?, ?, ?, ?, 'pending', ?, 0, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&new_db_id)
        .bind(&db.name)
        .bind(&db.db_type)
        .bind(&db.version)
        .bind(db.internal_port)
        // external_port = 0 (not started yet)
        .bind(db.public_access)
        .bind(&db.credentials)
        .bind(&db.volume_name)
        .bind(&new_volume_path)
        .bind(&db.memory_limit)
        .bind(&db.cpu_limit)
        .bind(&project_id)
        .bind(&db.team_id)
        .bind(&now)
        .bind(&now)
        .bind(&container_slug)
        .bind(&db.custom_image)
        .bind(&db.init_commands)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to clone database {}: {}", db.id, e);
            ApiError::database("Failed to clone database")
        })?;
    }

    // -----------------------------------------------------------------------
    // 5. Clone services
    // -----------------------------------------------------------------------
    let services = sqlx::query_as::<_, Service>(
        "SELECT * FROM services WHERE project_id = ?",
    )
    .bind(&project_id)
    .fetch_all(&state.db)
    .await?;

    let cloned_services = services.len();

    for svc in &services {
        let new_svc_id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO services (
                id, name, project_id, team_id, compose_content, domain, port,
                status, isolated_network, raw_compose_mode, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, NULL, ?, 'pending', ?, ?, ?, ?)
            "#,
        )
        .bind(&new_svc_id)
        .bind(&svc.name)
        .bind(&project_id)
        .bind(&svc.team_id)
        .bind(&svc.compose_content)
        // domain cleared (NULL above) — user must configure their own domain
        .bind(svc.port)
        // status = 'pending' (no running container)
        .bind(svc.isolated_network)
        .bind(svc.raw_compose_mode)
        .bind(&now)
        .bind(&now)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to clone service {}: {}", svc.id, e);
            ApiError::database("Failed to clone service")
        })?;
    }

    // -----------------------------------------------------------------------
    // 6. Fetch the newly created environment and return it
    // -----------------------------------------------------------------------
    let new_env =
        sqlx::query_as::<_, ProjectEnvironment>("SELECT * FROM environments WHERE id = ?")
            .bind(&new_env_id)
            .fetch_one(&state.db)
            .await?;

    Ok((
        StatusCode::CREATED,
        Json(CloneEnvironmentResponse {
            id: new_env.id,
            project_id: new_env.project_id,
            name: new_env.name,
            description: new_env.description,
            is_default: new_env.is_default != 0,
            created_at: new_env.created_at,
            updated_at: new_env.updated_at,
            cloned_apps,
            cloned_databases,
            cloned_services,
        }),
    ))
}

// ---------------------------------------------------------------------------
// Minimal row types for queries inside this file (avoid pulling full models
// just for SELECT *)
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct EnvironmentEnvVarRow {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub environment_id: String,
    pub key: String,
    pub value: String,
    pub is_secret: i32,
    #[allow(dead_code)]
    pub created_at: String,
}

#[derive(sqlx::FromRow)]
struct AppEnvVarRow {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub app_id: String,
    pub key: String,
    pub value: String,
    pub is_secret: i32,
    #[allow(dead_code)]
    pub created_at: String,
    #[allow(dead_code)]
    pub updated_at: String,
}

#[derive(sqlx::FromRow)]
struct VolumeRow {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub app_id: String,
    pub name: String,
    pub host_path: String,
    pub container_path: String,
    pub read_only: i32,
    #[allow(dead_code)]
    pub created_at: String,
    #[allow(dead_code)]
    pub updated_at: String,
}
