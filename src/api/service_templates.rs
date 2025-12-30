//! API handlers for service templates

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{
    DeployTemplateRequest, DeployTemplateResponse, ServiceStatus, ServiceTemplate,
    ServiceTemplateResponse,
};
use crate::AppState;

/// Namespace container names in compose content to prevent global conflicts
/// Prefixes all container_name values with "rivetr-{service_name}-"
fn namespace_container_names(content: &str, service_name: &str) -> Result<String, String> {
    let mut yaml: serde_yaml::Value = serde_yaml::from_str(content)
        .map_err(|e| format!("Invalid YAML: {}", e))?;

    let prefix = format!("rivetr-{}-", service_name);

    if let Some(mapping) = yaml.as_mapping_mut() {
        if let Some(services) = mapping.get_mut(&serde_yaml::Value::String("services".to_string())) {
            if let Some(services_map) = services.as_mapping_mut() {
                for (_service_key, service_config) in services_map.iter_mut() {
                    if let Some(config_map) = service_config.as_mapping_mut() {
                        let container_name_key = serde_yaml::Value::String("container_name".to_string());
                        if let Some(container_name_val) = config_map.get_mut(&container_name_key) {
                            if let Some(name) = container_name_val.as_str() {
                                // Only add prefix if not already prefixed
                                if !name.starts_with(&prefix) && !name.starts_with("rivetr-") {
                                    *container_name_val = serde_yaml::Value::String(format!("{}{}", prefix, name));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    serde_yaml::to_string(&yaml)
        .map_err(|e| format!("Failed to serialize YAML: {}", e))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// Filter by category
    pub category: Option<String>,
}

/// List all service templates
pub async fn list_templates(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<ServiceTemplateResponse>>, StatusCode> {
    let templates = if let Some(category) = query.category {
        sqlx::query_as::<_, ServiceTemplate>(
            "SELECT * FROM service_templates WHERE category = ? ORDER BY name ASC",
        )
        .bind(&category)
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as::<_, ServiceTemplate>(
            "SELECT * FROM service_templates ORDER BY category ASC, name ASC",
        )
        .fetch_all(&state.db)
        .await
    }
    .map_err(|e| {
        tracing::error!("Failed to list service templates: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let responses: Vec<ServiceTemplateResponse> =
        templates.into_iter().map(|t| t.to_response()).collect();

    Ok(Json(responses))
}

/// Get a single service template by ID
pub async fn get_template(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ServiceTemplateResponse>, StatusCode> {
    let template =
        sqlx::query_as::<_, ServiceTemplate>("SELECT * FROM service_templates WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get service template: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(template.to_response()))
}

/// Deploy a service template as a new service
pub async fn deploy_template(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<DeployTemplateRequest>,
) -> Result<(StatusCode, Json<DeployTemplateResponse>), StatusCode> {
    // Get the template
    let template =
        sqlx::query_as::<_, ServiceTemplate>("SELECT * FROM service_templates WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get service template: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or(StatusCode::NOT_FOUND)?;

    // Validate service name
    if req.name.is_empty() {
        tracing::warn!("Service name is empty");
        return Err(StatusCode::BAD_REQUEST);
    }

    if !req
        .name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        tracing::warn!("Service name contains invalid characters: {}", req.name);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Get the env schema to validate required variables
    let env_schema = template.get_env_schema();
    for entry in &env_schema {
        if entry.required && !req.env_vars.contains_key(&entry.name) && entry.default.is_empty() {
            tracing::warn!("Missing required environment variable: {}", entry.name);
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Substitute environment variables in the compose template
    let mut compose_content = template.compose_template.clone();
    for entry in &env_schema {
        let value = req
            .env_vars
            .get(&entry.name)
            .cloned()
            .unwrap_or_else(|| entry.default.clone());

        // Replace ${VAR_NAME:-default} and ${VAR_NAME} patterns
        let pattern_with_default = format!("${{{}:-{}}}", entry.name, entry.default);
        let pattern_simple = format!("${{{}}}", entry.name);

        compose_content = compose_content.replace(&pattern_with_default, &value);
        compose_content = compose_content.replace(&pattern_simple, &value);
    }

    // Also substitute any env vars passed that weren't in the schema
    for (key, value) in &req.env_vars {
        let pattern_simple = format!("${{{}}}", key);
        compose_content = compose_content.replace(&pattern_simple, value);
    }

    // Create the service record
    let service_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO services (id, name, project_id, compose_content, status, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&service_id)
    .bind(&req.name)
    .bind(&req.project_id)
    .bind(&compose_content)
    .bind(ServiceStatus::Pending.to_string())
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create service: {}", e);
        if e.to_string().contains("UNIQUE") {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    // Start the service asynchronously using docker compose
    let state_clone = state.clone();
    let service_id_clone = service_id.clone();
    let service_name = req.name.clone();
    tokio::spawn(async move {
        if let Err(e) = start_compose_service(&state_clone, &service_id_clone, &service_name, &compose_content).await {
            tracing::error!("Failed to start service {}: {}", service_name, e);
            // Update status to failed
            let _ = sqlx::query(
                "UPDATE services SET status = ?, error_message = ?, updated_at = datetime('now') WHERE id = ?",
            )
            .bind(ServiceStatus::Failed.to_string())
            .bind(e.to_string())
            .bind(&service_id_clone)
            .execute(&state_clone.db)
            .await;
        }
    });

    tracing::info!(
        "Deploying service '{}' from template '{}'",
        req.name,
        template.name
    );

    Ok((
        StatusCode::CREATED,
        Json(DeployTemplateResponse {
            service_id,
            name: req.name,
            template_id: id,
            status: ServiceStatus::Pending.to_string(),
            message: format!("Service deployment started from template '{}'", template.name),
        }),
    ))
}

/// Internal function to start a compose service
async fn start_compose_service(
    state: &Arc<AppState>,
    service_id: &str,
    name: &str,
    compose_content: &str,
) -> anyhow::Result<()> {
    use std::io::Write;
    use tokio::process::Command;

    // Create temp directory for compose file
    let compose_dir = std::env::temp_dir().join(format!("rivetr-svc-{}", name));
    std::fs::create_dir_all(&compose_dir)?;

    // Namespace container names to prevent global conflicts
    let namespaced_content = namespace_container_names(compose_content, name)
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to namespace container names: {}. Using original content.", e);
            compose_content.to_string()
        });

    // Write compose file
    let compose_path = compose_dir.join("docker-compose.yml");
    let mut file = std::fs::File::create(&compose_path)?;
    file.write_all(namespaced_content.as_bytes())?;

    tracing::info!("Starting compose service: {} at {:?}", name, compose_path);

    // Run docker compose up
    let project_name = format!("rivetr-svc-{}", name);

    // Clean up any orphaned containers from previous failed deployments
    // This prevents "container name already in use" errors
    tracing::debug!("Cleaning up orphaned containers for project: {}", project_name);
    let _ = Command::new("docker")
        .args([
            "compose",
            "-p",
            &project_name,
            "-f",
            compose_path.to_str().unwrap_or("docker-compose.yml"),
            "down",
            "--remove-orphans",
        ])
        .output()
        .await;

    let output = Command::new("docker")
        .args([
            "compose",
            "-p",
            &project_name,
            "-f",
            compose_path.to_str().unwrap_or("docker-compose.yml"),
            "up",
            "-d",
        ])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::error!("Failed to start compose service: {}", stderr);
        return Err(anyhow::anyhow!("Docker compose failed: {}", stderr));
    }

    // Update service status to running
    sqlx::query("UPDATE services SET status = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(ServiceStatus::Running.to_string())
        .bind(service_id)
        .execute(&state.db)
        .await?;

    tracing::info!("Service {} started successfully", name);

    Ok(())
}

/// Get available template categories
pub async fn list_categories() -> Json<Vec<&'static str>> {
    Json(vec![
        "monitoring",
        "database",
        "storage",
        "development",
        "analytics",
        "networking",
        "security",
    ])
}
