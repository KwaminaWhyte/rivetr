//! Per-service deploy configuration: build source, images, registry, hooks.

use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::api::audit::{audit_log, ClientIp};
use crate::api::error::ApiError;
use crate::db::{
    actions, resource_types, service_source_type, Service, ServiceDeployConfig,
    ServiceDeployConfigResponse, ServiceDeployHook, UpdateServiceDeployConfigRequest, User,
};
use crate::engine::service_deploy::{build::validate_build_config, load_deploy_config};
use crate::AppState;

fn encryption_key(state: &AppState) -> Option<[u8; 32]> {
    state
        .config
        .auth
        .encryption_key
        .as_ref()
        .map(|s| crate::crypto::derive_key(s))
}

/// Compose service names declared in a compose file.
pub(crate) fn compose_service_names(compose: &str) -> Vec<String> {
    serde_yaml::from_str::<serde_yaml::Value>(compose)
        .ok()
        .and_then(|y| y.get("services").and_then(|s| s.as_mapping()).cloned())
        .map(|m| {
            m.keys()
                .filter_map(|k| k.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Validate hooks against the compose services they run in.
pub(crate) fn validate_hooks(
    hooks: &[ServiceDeployHook],
    compose_services: &[String],
    kind: &str,
) -> Result<(), String> {
    for (i, h) in hooks.iter().enumerate() {
        let n = i + 1;
        if h.command.trim().is_empty() {
            return Err(format!("{} hook {} has no command", kind, n));
        }
        if h.command.contains('\0') {
            return Err(format!("{} hook {}: invalid command", kind, n));
        }
        let svc = h.service.trim();
        if svc.is_empty() {
            return Err(format!(
                "{} hook {}: pick the compose service to run it in",
                kind, n
            ));
        }
        if !compose_services.iter().any(|s| s == svc) {
            return Err(format!(
                "{} hook {}: '{}' is not a service in the compose file",
                kind, n, svc
            ));
        }
        if let Some(t) = h.timeout_secs {
            if t == 0 || t > 6 * 60 * 60 {
                return Err(format!(
                    "{} hook {}: timeout must be between 1 second and 6 hours",
                    kind, n
                ));
            }
        }
    }
    Ok(())
}

fn non_empty(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// GET /api/services/:id/deploy-config
pub async fn get_deploy_config(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ServiceDeployConfigResponse>, ApiError> {
    let cfg = load_deploy_config(&state.db, &id)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?;
    Ok(Json(cfg.to_response()))
}

/// PUT /api/services/:id/deploy-config
pub async fn update_deploy_config(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(id): Path<String>,
    Json(req): Json<UpdateServiceDeployConfigRequest>,
) -> Result<Json<ServiceDeployConfigResponse>, ApiError> {
    let service: Service = sqlx::query_as("SELECT * FROM services WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("Service not found"))?;
    let mut cfg: ServiceDeployConfig = load_deploy_config(&state.db, &id)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?;

    if let Some(v) = req.source_type {
        cfg.source_type = v.trim().to_string();
    }
    if let Some(v) = req.git_url {
        cfg.git_url = non_empty(Some(v));
    }
    if let Some(v) = req.branch {
        cfg.branch = v.trim().to_string();
    }
    if let Some(v) = req.github_app_installation_id {
        cfg.github_app_installation_id = non_empty(Some(v));
    }
    if let Some(v) = req.git_provider_id {
        cfg.git_provider_id = non_empty(Some(v));
    }
    if let Some(v) = req.source_path {
        cfg.source_path = non_empty(Some(v));
    }
    if let Some(v) = req.revision_file {
        cfg.revision_file = non_empty(Some(v));
    }
    if let Some(v) = req.images {
        cfg.images = serde_json::to_string(&v).map_err(|e| ApiError::bad_request(e.to_string()))?;
    }
    if let Some(v) = req.push_images {
        cfg.push_images = v as i32;
    }
    if let Some(v) = req.registry_username {
        cfg.registry_username = non_empty(Some(v));
    }
    if let Some(v) = req.registry_password {
        cfg.registry_password = if v.is_empty() {
            None
        } else {
            Some(
                crate::crypto::encrypt_if_key_available(&v, encryption_key(&state).as_ref())
                    .map_err(|e| ApiError::internal(format!("encrypt: {}", e)))?,
            )
        };
    }
    if let Some(v) = req.pre_deploy_hooks {
        cfg.pre_deploy_hooks =
            serde_json::to_string(&v).map_err(|e| ApiError::bad_request(e.to_string()))?;
    }
    if let Some(v) = req.post_deploy_hooks {
        cfg.post_deploy_hooks =
            serde_json::to_string(&v).map_err(|e| ApiError::bad_request(e.to_string()))?;
    }
    if let Some(v) = req.auto_deploy {
        cfg.auto_deploy = v as i32;
    }
    if let Some(v) = req.inject_env_file {
        cfg.inject_env_file = v as i32;
    }
    if cfg.branch.is_empty() {
        cfg.branch = "main".to_string();
    }
    if cfg.source_type != service_source_type::GIT {
        cfg.auto_deploy = 0;
    }

    validate_build_config(&cfg).map_err(ApiError::bad_request)?;
    let names = compose_service_names(&service.compose_content);
    validate_hooks(&cfg.pre_hooks(), &names, "Pre-deploy").map_err(ApiError::bad_request)?;
    validate_hooks(&cfg.post_hooks(), &names, "Post-deploy").map_err(ApiError::bad_request)?;

    // Credentials must belong to the caller (or their team).
    if let Some(inst) = &cfg.github_app_installation_id {
        crate::api::authz::authorize_github_installation(&state.db, &user, inst, false).await?;
    }
    if let Some(pid) = &cfg.git_provider_id {
        let owner: Option<String> =
            sqlx::query_scalar("SELECT user_id FROM git_providers WHERE id = ?")
                .bind(pid)
                .fetch_optional(&state.db)
                .await
                .map_err(|e| ApiError::database(e.to_string()))?;
        match owner {
            None => return Err(ApiError::bad_request("Git provider not found")),
            Some(o) if o != user.id && !crate::api::authz::is_privileged_user(&user) => {
                return Err(ApiError::forbidden(
                    "That git provider belongs to another user",
                ))
            }
            _ => {}
        }
    }

    sqlx::query(
        "INSERT INTO service_deploy_configs (service_id, source_type, git_url, branch, \
         github_app_installation_id, git_provider_id, source_path, revision_file, images, \
         push_images, registry_username, registry_password, pre_deploy_hooks, post_deploy_hooks, \
         auto_deploy, pinned_images, inject_env_file, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now')) \
         ON CONFLICT(service_id) DO UPDATE SET source_type = excluded.source_type, \
         git_url = excluded.git_url, branch = excluded.branch, \
         github_app_installation_id = excluded.github_app_installation_id, \
         git_provider_id = excluded.git_provider_id, source_path = excluded.source_path, \
         revision_file = excluded.revision_file, images = excluded.images, \
         push_images = excluded.push_images, registry_username = excluded.registry_username, \
         registry_password = excluded.registry_password, \
         pre_deploy_hooks = excluded.pre_deploy_hooks, post_deploy_hooks = excluded.post_deploy_hooks, \
         auto_deploy = excluded.auto_deploy, inject_env_file = excluded.inject_env_file, \
         updated_at = datetime('now')",
    )
    .bind(&id)
    .bind(&cfg.source_type)
    .bind(&cfg.git_url)
    .bind(&cfg.branch)
    .bind(&cfg.github_app_installation_id)
    .bind(&cfg.git_provider_id)
    .bind(&cfg.source_path)
    .bind(&cfg.revision_file)
    .bind(&cfg.images)
    .bind(cfg.push_images)
    .bind(&cfg.registry_username)
    .bind(&cfg.registry_password)
    .bind(&cfg.pre_deploy_hooks)
    .bind(&cfg.post_deploy_hooks)
    .bind(cfg.auto_deploy)
    .bind(&cfg.pinned_images)
    .bind(cfg.inject_env_file)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::database(e.to_string()))?;

    audit_log(
        &state,
        actions::SERVICE_UPDATE,
        resource_types::SERVICE,
        Some(&id),
        Some(&service.name),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "deploy_config": true, "source_type": cfg.source_type })),
    )
    .await;

    let cfg = load_deploy_config(&state.db, &id)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?;
    Ok(Json(cfg.to_response()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hooks_must_target_a_compose_service() {
        let names =
            compose_service_names("services:\n  api:\n    image: a\n  web:\n    image: a\n");
        assert_eq!(names, vec!["api", "web"]);
        let ok = ServiceDeployHook {
            name: None,
            service: "api".into(),
            command: "bun run db:migrate".into(),
            timeout_secs: Some(300),
        };
        assert!(validate_hooks(std::slice::from_ref(&ok), &names, "Pre-deploy").is_ok());
        let wrong = ServiceDeployHook {
            service: "nope".into(),
            ..ok.clone()
        };
        assert!(validate_hooks(&[wrong], &names, "Pre-deploy").is_err());
        let empty = ServiceDeployHook {
            command: "  ".into(),
            ..ok
        };
        assert!(validate_hooks(&[empty], &names, "Pre-deploy").is_err());
    }
}
