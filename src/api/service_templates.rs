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
    let mut yaml: serde_yaml::Value =
        serde_yaml::from_str(content).map_err(|e| format!("Invalid YAML: {}", e))?;

    let prefix = format!("rivetr-{}-", service_name);

    if let Some(mapping) = yaml.as_mapping_mut() {
        if let Some(services) = mapping.get_mut(serde_yaml::Value::String("services".to_string())) {
            if let Some(services_map) = services.as_mapping_mut() {
                for (_service_key, service_config) in services_map.iter_mut() {
                    if let Some(config_map) = service_config.as_mapping_mut() {
                        let container_name_key =
                            serde_yaml::Value::String("container_name".to_string());
                        if let Some(container_name_val) = config_map.get_mut(&container_name_key) {
                            if let Some(name) = container_name_val.as_str() {
                                // Only add prefix if not already prefixed
                                if !name.starts_with(&prefix) && !name.starts_with("rivetr-") {
                                    *container_name_val =
                                        serde_yaml::Value::String(format!("{}{}", prefix, name));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    serde_yaml::to_string(&yaml).map_err(|e| format!("Failed to serialize YAML: {}", e))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// Filter by category
    pub category: Option<String>,
    /// Full-text search on name and description
    pub search: Option<String>,
}

/// List all service templates
pub async fn list_templates(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<ServiceTemplateResponse>>, StatusCode> {
    let search_pattern = query
        .search
        .as_ref()
        .map(|s| format!("%{}%", s.to_lowercase()));

    let templates = sqlx::query_as::<_, ServiceTemplate>(
        r#"
        SELECT * FROM service_templates
        WHERE (? IS NULL OR LOWER(name) LIKE ? OR LOWER(description) LIKE ?)
          AND (? IS NULL OR category = ?)
        ORDER BY name ASC
        "#,
    )
    .bind(&search_pattern)
    .bind(&search_pattern)
    .bind(&search_pattern)
    .bind(&query.category)
    .bind(&query.category)
    .fetch_all(&state.db)
    .await
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
) -> Result<(StatusCode, Json<DeployTemplateResponse>), (StatusCode, Json<serde_json::Value>)> {
    let make_internal_err = || {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                serde_json::json!({ "error": { "code": "internal_error", "message": "An internal error occurred" } }),
            ),
        )
    };

    // Get the template
    let template =
        sqlx::query_as::<_, ServiceTemplate>("SELECT * FROM service_templates WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get service template: {}", e);
                make_internal_err()
            })?
            .ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": { "code": "not_found", "message": "Template not found" } })),
                )
            })?;

    // Validate service name
    if req.name.is_empty() {
        tracing::warn!("Service name is empty");
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({ "error": { "code": "bad_request", "message": "Service name is required" } }),
            ),
        ));
    }

    if !req
        .name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        tracing::warn!("Service name contains invalid characters: {}", req.name);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({ "error": { "code": "bad_request", "message": "Service name contains invalid characters" } }),
            ),
        ));
    }

    // Check for port conflict if a PORT env var is provided
    if let Some(port_str) = req.env_vars.get("PORT") {
        if let Ok(port) = port_str.parse::<i64>() {
            // Check if port is already used by another service
            let existing_service: Option<(String,)> =
                sqlx::query_as("SELECT name FROM services WHERE port = ?")
                    .bind(port)
                    .fetch_optional(&state.db)
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to check port conflict in services: {}", e);
                        make_internal_err()
                    })?;

            if let Some((name,)) = existing_service {
                return Err((
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({
                        "error": {
                            "code": "conflict",
                            "message": format!("Port {} is already used by service '{}'", port, name)
                        }
                    })),
                ));
            }

            // Check if port is already used by a public database
            let existing_db: Option<(String,)> = sqlx::query_as(
                "SELECT name FROM databases WHERE external_port = ? AND public_access = 1",
            )
            .bind(port as i32)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to check port conflict in databases: {}", e);
                make_internal_err()
            })?;

            if let Some((name,)) = existing_db {
                return Err((
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({
                        "error": {
                            "code": "conflict",
                            "message": format!("Port {} is already used by database '{}'", port, name)
                        }
                    })),
                ));
            }
        }
    }

    // Get the env schema to validate required variables
    let env_schema = template.get_env_schema();
    for entry in &env_schema {
        if entry.required && !req.env_vars.contains_key(&entry.name) && entry.default.is_empty() {
            tracing::warn!("Missing required environment variable: {}", entry.name);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(
                    serde_json::json!({ "error": { "code": "bad_request", "message": format!("Missing required variable: {}", entry.name) } }),
                ),
            ));
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

    // Auto-generate SERVICE_PASSWORD_*, SERVICE_USER_*, SERVICE_BASE64_* magic variables
    {
        use base64::Engine as _;
        use rand::Rng;

        let magic_var_re =
            regex::Regex::new(r"\$\{(SERVICE_(?:PASSWORD|USER|BASE64)_([A-Z0-9_]+))(?::-[^}]*)?\}")
                .expect("invalid magic var regex");

        let mut generated: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        // Collect all magic variable names first (so each gets a stable generated value)
        for cap in magic_var_re.captures_iter(&compose_content.clone()) {
            let full_var = cap[1].to_string();
            let name_part = cap[2].to_string();
            if generated.contains_key(&full_var) {
                continue;
            }
            let value: String = if full_var.starts_with("SERVICE_PASSWORD_") {
                let mut rng = rand::rng();
                let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
                (0..32)
                    .map(|_| chars[rng.random_range(0..chars.len())] as char)
                    .collect()
            } else if full_var.starts_with("SERVICE_USER_") {
                name_part.to_lowercase()
            } else {
                // SERVICE_BASE64_*
                let mut rng = rand::rng();
                let bytes: Vec<u8> = (0..32).map(|_| rng.random::<u8>()).collect();
                base64::engine::general_purpose::STANDARD.encode(&bytes)
            };
            generated.insert(full_var, value);
        }

        // Apply generated variables — replace both ${VAR} and ${VAR:-default} forms
        for (var, value) in &generated {
            // Simple form: ${VAR}
            compose_content = compose_content.replace(&format!("${{{}}}", var), value);
            // Default form: ${VAR:-something}
            let re2 = regex::Regex::new(&format!(r"\$\{{{var}:-[^}}]*\}}"))
                .expect("invalid replacement regex");
            compose_content = re2
                .replace_all(&compose_content, value.as_str())
                .to_string();
        }
    }

    // Auto-generate subdomain for the service
    let domain = state.config.proxy.generate_auto_domain(&req.name);

    // Determine proxy port: prefer PORT env var, then extract first host port
    // from the rendered compose file (e.g. "3000:3000" → 3000).
    let port: i32 = req
        .env_vars
        .get("PORT")
        .and_then(|p| p.parse::<i32>().ok())
        .or_else(|| extract_first_host_port(&compose_content))
        .unwrap_or(80);

    // Create the service record
    let service_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO services (id, name, project_id, compose_content, domain, port, status, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&service_id)
    .bind(&req.name)
    .bind(&req.project_id)
    .bind(&compose_content)
    .bind(&domain)
    .bind(port)
    .bind(ServiceStatus::Pending.to_string())
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create service: {}", e);
        if e.to_string().contains("UNIQUE") {
            (
                StatusCode::CONFLICT,
                Json(serde_json::json!({ "error": { "code": "conflict", "message": "A service with this name already exists" } })),
            )
        } else {
            make_internal_err()
        }
    })?;

    // Start the service asynchronously using docker compose
    let state_clone = state.clone();
    let service_id_clone = service_id.clone();
    let service_name = req.name.clone();
    tokio::spawn(async move {
        if let Err(e) = start_compose_service(
            &state_clone,
            &service_id_clone,
            &service_name,
            &compose_content,
        )
        .await
        {
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
            message: format!(
                "Service deployment started from template '{}'",
                template.name
            ),
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
    let namespaced_content = namespace_container_names(compose_content, name).unwrap_or_else(|e| {
        tracing::warn!(
            "Failed to namespace container names: {}. Using original content.",
            e
        );
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
    tracing::debug!(
        "Cleaning up orphaned containers for project: {}",
        project_name
    );
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

    // Register proxy route if the service has a domain configured
    let service = sqlx::query_as::<_, crate::db::Service>("SELECT * FROM services WHERE id = ?")
        .bind(service_id)
        .fetch_optional(&state.db)
        .await?;

    if let Some(ref svc) = service {
        if let Some(ref domain) = svc.domain {
            if !domain.is_empty() {
                let backend = crate::proxy::Backend::new(
                    format!("rivetr-svc-{}", svc.name),
                    "127.0.0.1".to_string(),
                    svc.port as u16,
                );
                state.routes.load().add_route(domain.clone(), backend);
                tracing::info!(
                    "Registered proxy route for template service: {} -> port {}",
                    domain,
                    svc.port
                );
            }
        }
    }

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

// ---------------------------------------------------------------------------
// Template Suggestions
// ---------------------------------------------------------------------------

/// Request body for submitting a template suggestion
#[derive(Debug, serde::Deserialize)]
pub struct TemplateSuggestionRequest {
    pub name: String,
    pub description: String,
    pub docker_image: String,
    pub category: String,
    pub website_url: Option<String>,
    pub notes: Option<String>,
}

/// Row model for a template suggestion
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct TemplateSuggestion {
    pub id: String,
    pub name: String,
    pub description: String,
    pub docker_image: String,
    pub category: String,
    pub website_url: Option<String>,
    pub notes: Option<String>,
    pub status: String,
    pub submitted_by: Option<String>,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<String>,
    pub created_at: String,
}

/// Submit a new template suggestion (no auth required)
pub async fn suggest_template(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TemplateSuggestionRequest>,
) -> Result<(StatusCode, Json<TemplateSuggestion>), StatusCode> {
    if req.name.trim().is_empty() || req.docker_image.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO template_suggestions
            (id, name, description, docker_image, category, website_url, notes, status, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, 'pending', ?)
        "#,
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.docker_image)
    .bind(&req.category)
    .bind(&req.website_url)
    .bind(&req.notes)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to insert template suggestion: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let suggestion =
        sqlx::query_as::<_, TemplateSuggestion>("SELECT * FROM template_suggestions WHERE id = ?")
            .bind(&id)
            .fetch_one(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(suggestion)))
}

/// List all template suggestions (admin only — caller should protect this route)
pub async fn list_template_suggestions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<TemplateSuggestion>>, StatusCode> {
    let suggestions = sqlx::query_as::<_, TemplateSuggestion>(
        "SELECT * FROM template_suggestions ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list template suggestions: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(suggestions))
}

/// Approve a template suggestion and seed it into service_templates
pub async fn approve_template_suggestion(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    // Fetch the suggestion
    let suggestion =
        sqlx::query_as::<_, TemplateSuggestion>("SELECT * FROM template_suggestions WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

    // Build a minimal compose template from the docker image
    let compose_template = format!(
        "version: '3'\nservices:\n  {}:\n    image: {}\n    restart: unless-stopped\n",
        suggestion.name.to_lowercase().replace(' ', "_"),
        suggestion.docker_image
    );

    let template_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO service_templates
            (id, name, description, category, compose_template, env_schema, icon, website_url, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, '[]', NULL, ?, ?, ?)
        "#,
    )
    .bind(&template_id)
    .bind(&suggestion.name)
    .bind(&suggestion.description)
    .bind(&suggestion.category)
    .bind(&compose_template)
    .bind(&suggestion.website_url)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to seed approved template suggestion: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Mark suggestion as approved
    sqlx::query(
        "UPDATE template_suggestions SET status = 'approved', reviewed_at = ? WHERE id = ?",
    )
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

/// Extract the first host-side port from a rendered docker-compose YAML string.
/// Only reads entries inside a `ports:` block. Handles "HOST:CONTAINER" and
/// "HOST:CONTAINER/proto" formats. Returns None if no ports section is found.
fn extract_first_host_port(compose: &str) -> Option<i32> {
    let mut in_ports = false;
    for line in compose.lines() {
        let trimmed = line.trim();
        if trimmed == "ports:" {
            in_ports = true;
            continue;
        }
        if in_ports {
            if trimmed.starts_with("- ") {
                let value = trimmed
                    .trim_start_matches("- ")
                    .trim_matches('"')
                    .trim_matches('\'');
                // Remove /tcp or /udp suffix
                let value = value.split('/').next().unwrap_or(value);
                // Host port is before the first ':'
                let host_part = value.split(':').next().unwrap_or("");
                if let Ok(port) = host_part.trim().parse::<i32>() {
                    if port > 0 && port < 65536 {
                        return Some(port);
                    }
                }
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                // Left the ports block
                in_ports = false;
            }
        }
    }
    None
}
