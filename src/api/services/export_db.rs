//! Database dump export handler for Docker Compose services.
//!
//! Runs the appropriate dump command inside the target container and streams
//! the output as a file download.

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::db::{Service, ServiceStatus};
use crate::AppState;

/// Query parameters for the export endpoint
#[derive(Debug, Deserialize)]
pub struct ExportDbQuery {
    /// The container name to target. If omitted, auto-detected.
    pub container_name: Option<String>,
    /// The database name to dump. Defaults to "app".
    pub database: Option<String>,
    /// Output format: "sql" (default) or "gz" (compressed / pg_custom).
    pub format: Option<String>,
}

/// GET /api/services/:id/export-db
///
/// Query params:
///   - `container_name` — which container in the compose stack to target (optional)
///   - `database`       — the database name to dump (defaults to "app")
///   - `format`         — "sql" (default) or "gz"
pub async fn export_service_db(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<ExportDbQuery>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // ── 1. Fetch service ──────────────────────────────────────────────────────
    let service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get service: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to get service"})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Service not found"})),
            )
        })?;

    if service.get_status() != ServiceStatus::Running {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({"error": "Service must be running to export a database dump"}),
            ),
        ));
    }

    // ── 2. Resolve the target container ──────────────────────────────────────
    let project_name = service.compose_project_name();

    let container_id = match query.container_name.as_deref().filter(|n| !n.is_empty()) {
        Some(name) => name.to_string(),
        None => {
            let containers = state
                .runtime
                .list_compose_containers(&project_name)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to list compose containers: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": format!("Failed to list containers: {}", e)})),
                    )
                })?;

            containers
                .into_iter()
                .find(|c| c.running)
                .map(|c| c.name)
                .ok_or_else(|| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({"error": "No running containers found for this service. Start the service first."})),
                    )
                })?
        }
    };

    // ── 3. Detect DB type from image name in compose content ──────────────────
    let db_type = detect_db_type_from_compose(&service.compose_content, &container_id);

    // ── 4. Determine output format ────────────────────────────────────────────
    let use_gz = query
        .format
        .as_deref()
        .map(|f| f == "gz")
        .unwrap_or(false);

    let db_name = query
        .database
        .as_deref()
        .unwrap_or("app")
        .to_string();

    // ── 5. Build dump command ─────────────────────────────────────────────────
    let dump_cmd = build_dump_command(&db_type, &db_name, use_gz, &service.compose_content)
        .map_err(|e| e)?;

    // ── 6. Run the dump command inside the container ──────────────────────────
    let result = state
        .runtime
        .run_command(&container_id, dump_cmd)
        .await
        .map_err(|e| {
            tracing::error!("Failed to run dump command: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    serde_json::json!({"error": format!("Failed to run dump command: {}", e)}),
                ),
            )
        })?;

    if result.exit_code != 0 {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Dump command failed",
                "stderr": result.stderr,
            })),
        ));
    }

    // ── 7. Build a filename for the download ──────────────────────────────────
    let date_str = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let safe_name = service
        .name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' })
        .collect::<String>();

    let safe_db = db_name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' })
        .collect::<String>();

    let ext = if use_gz { ".sql.gz" } else { ".sql" };
    let filename = format!("{}-{}-{}{}", safe_name, safe_db, date_str, ext);

    // ── 8. Stream the dump bytes as a file download ───────────────────────────
    tracing::info!(
        "Exported dump from service {} container {} ({} bytes)",
        service.name,
        container_id,
        result.stdout.len()
    );

    let body = Body::from(result.stdout.into_bytes());

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(body)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to build response: {}", e)})),
            )
        })?;

    Ok(response.into_response())
}

/// Detect the database type from the compose YAML image names.
/// Falls back to "unknown" if the type can't be determined.
fn detect_db_type_from_compose(compose_content: &str, container_hint: &str) -> String {
    let hint = container_hint.to_lowercase();

    for (keyword, db) in &[
        ("postgres", "postgres"),
        ("pg", "postgres"),
        ("mysql", "mysql"),
        ("mariadb", "mariadb"),
        ("mongo", "mongodb"),
        ("redis", "redis"),
    ] {
        if hint.contains(keyword) {
            return db.to_string();
        }
    }

    if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(compose_content) {
        if let Some(services) = yaml
            .as_mapping()
            .and_then(|m| m.get(serde_yaml::Value::String("services".to_string())))
            .and_then(|s| s.as_mapping())
        {
            for (_svc_name, svc_cfg) in services {
                let image = svc_cfg
                    .as_mapping()
                    .and_then(|m| m.get(serde_yaml::Value::String("image".to_string())))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_lowercase();

                if image.contains("postgres") {
                    return "postgres".to_string();
                }
                if image.contains("mysql") {
                    return "mysql".to_string();
                }
                if image.contains("mariadb") {
                    return "mariadb".to_string();
                }
                if image.contains("mongo") {
                    return "mongodb".to_string();
                }
            }
        }
    }

    "unknown".to_string()
}

/// Construct the dump command for the detected DB type.
fn build_dump_command(
    db_type: &str,
    db_name: &str,
    use_gz: bool,
    compose_content: &str,
) -> Result<Vec<String>, (StatusCode, Json<serde_json::Value>)> {
    let env_vars = extract_env_vars_from_compose(compose_content);

    let cmd: Vec<String> = match db_type {
        "postgres" => {
            let user = env_vars
                .get("POSTGRES_USER")
                .cloned()
                .unwrap_or_else(|| "postgres".to_string());
            let password = env_vars
                .get("POSTGRES_PASSWORD")
                .cloned()
                .unwrap_or_default();
            let effective_db = env_vars
                .get("POSTGRES_DB")
                .cloned()
                .unwrap_or_else(|| db_name.to_string());

            if use_gz {
                // pg_dump custom/archive format (binary, suitable as .gz)
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!(
                        "PGPASSWORD='{}' pg_dump -U {} -d {} -Fc",
                        password, user, effective_db
                    ),
                ]
            } else {
                // Plain SQL dump
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!(
                        "PGPASSWORD='{}' pg_dump -U {} -d {}",
                        password, user, effective_db
                    ),
                ]
            }
        }
        "mysql" => {
            let user = env_vars
                .get("MYSQL_USER")
                .or_else(|| env_vars.get("MYSQL_ROOT_USER"))
                .cloned()
                .unwrap_or_else(|| "root".to_string());
            let password = env_vars
                .get("MYSQL_PASSWORD")
                .or_else(|| env_vars.get("MYSQL_ROOT_PASSWORD"))
                .cloned()
                .unwrap_or_default();
            let effective_db = env_vars
                .get("MYSQL_DATABASE")
                .cloned()
                .unwrap_or_else(|| db_name.to_string());

            if use_gz {
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!(
                        "mysqldump -u{} -p'{}' {} | gzip",
                        user, password, effective_db
                    ),
                ]
            } else {
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!(
                        "mysqldump -u{} -p'{}' {}",
                        user, password, effective_db
                    ),
                ]
            }
        }
        "mariadb" => {
            let user = env_vars
                .get("MARIADB_USER")
                .or_else(|| env_vars.get("MYSQL_USER"))
                .cloned()
                .unwrap_or_else(|| "root".to_string());
            let password = env_vars
                .get("MARIADB_PASSWORD")
                .or_else(|| env_vars.get("MARIADB_ROOT_PASSWORD"))
                .or_else(|| env_vars.get("MYSQL_ROOT_PASSWORD"))
                .cloned()
                .unwrap_or_default();
            let effective_db = env_vars
                .get("MARIADB_DATABASE")
                .or_else(|| env_vars.get("MYSQL_DATABASE"))
                .cloned()
                .unwrap_or_else(|| db_name.to_string());

            if use_gz {
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!(
                        "mariadb-dump -u{} -p'{}' {} | gzip",
                        user, password, effective_db
                    ),
                ]
            } else {
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!(
                        "mariadb-dump -u{} -p'{}' {}",
                        user, password, effective_db
                    ),
                ]
            }
        }
        "mongodb" => {
            let user = env_vars
                .get("MONGO_INITDB_ROOT_USERNAME")
                .cloned()
                .unwrap_or_default();
            let password = env_vars
                .get("MONGO_INITDB_ROOT_PASSWORD")
                .cloned()
                .unwrap_or_default();

            let auth_part = if !user.is_empty() {
                format!(
                    "--username {} --password '{}' --authenticationDatabase admin",
                    user, password
                )
            } else {
                String::new()
            };

            // mongodump always outputs binary archive; --gzip compresses it
            if use_gz {
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!(
                        "mongodump {} --archive --db {} --gzip",
                        auth_part, db_name
                    ),
                ]
            } else {
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!(
                        "mongodump {} --archive --db {}",
                        auth_part, db_name
                    ),
                ]
            }
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Cannot determine database type from container image. Please ensure the container image name includes 'postgres', 'mysql', 'mariadb', or 'mongo'."
                })),
            ));
        }
    };

    Ok(cmd)
}

/// Parse environment variables from a compose YAML string.
fn extract_env_vars_from_compose(
    compose_content: &str,
) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();

    let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(compose_content) else {
        return map;
    };

    let Some(services) = yaml
        .as_mapping()
        .and_then(|m| m.get(serde_yaml::Value::String("services".to_string())))
        .and_then(|s| s.as_mapping())
    else {
        return map;
    };

    for (_svc_name, svc_cfg) in services {
        let Some(env_section) = svc_cfg
            .as_mapping()
            .and_then(|m| m.get(serde_yaml::Value::String("environment".to_string())))
        else {
            continue;
        };

        if let Some(env_map) = env_section.as_mapping() {
            for (k, v) in env_map {
                if let (Some(key), Some(val)) = (k.as_str(), v.as_str()) {
                    if !val.contains('$') {
                        map.insert(key.to_string(), val.to_string());
                    }
                }
            }
        } else if let Some(env_seq) = env_section.as_sequence() {
            for item in env_seq {
                if let Some(entry) = item.as_str() {
                    if let Some((k, v)) = entry.split_once('=') {
                        if !v.contains('$') {
                            map.insert(k.to_string(), v.to_string());
                        }
                    }
                }
            }
        }
    }

    map
}
