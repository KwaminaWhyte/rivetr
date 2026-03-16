//! Database dump import handler for Docker Compose services.
//!
//! Accepts a multipart upload of a `.sql` or `.sql.gz` file and runs the
//! appropriate restore command inside the target container.

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::db::{Service, ServiceStatus};
use crate::AppState;

/// POST /api/services/:id/import-db
///
/// Multipart fields:
///   - `file`           — the dump file (.sql or .sql.gz / pg_custom)
///   - `container_name` — which container in the compose stack to target
///   - `database`       — the database name to restore into
pub async fn import_service_db(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    use base64::Engine as _;

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
            Json(serde_json::json!({"error": "Service must be running to import a database dump"})),
        ));
    }

    // ── 2. Parse multipart fields ─────────────────────────────────────────────
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut container_name: Option<String> = None;
    let mut database_name: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("Failed to read multipart field: {}", e)})),
        )
    })? {
        match field.name() {
            Some("file") => {
                file_name = field.file_name().map(|s| s.to_string());
                file_bytes = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| {
                            (
                                StatusCode::BAD_REQUEST,
                                Json(serde_json::json!({"error": format!("Failed to read file: {}", e)})),
                            )
                        })?
                        .to_vec(),
                );
            }
            Some("container_name") => {
                container_name = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| {
                            (
                                StatusCode::BAD_REQUEST,
                                Json(serde_json::json!({"error": format!("Failed to read container_name: {}", e)})),
                            )
                        })?,
                );
            }
            Some("database") => {
                database_name = Some(field.text().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(
                            serde_json::json!({"error": format!("Failed to read database: {}", e)}),
                        ),
                    )
                })?);
            }
            _ => {}
        }
    }

    let bytes = file_bytes.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "No 'file' field found in request"})),
        )
    })?;

    if bytes.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Uploaded file is empty"})),
        ));
    }

    // ── 3. Resolve the target container ──────────────────────────────────────
    // The caller passes the container name directly (as shown in the compose UI).
    // If none was provided, we fall back to the first running container for this
    // compose project.
    let project_name = service.compose_project_name();

    let resolved_container = if let Some(ref name) = container_name {
        if name.is_empty() {
            None
        } else {
            Some(name.clone())
        }
    } else {
        None
    };

    let container_id = match resolved_container {
        Some(name) => name,
        None => {
            // List containers for the compose project and pick the first one
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

    // ── 4. Detect DB type from image name in compose content ──────────────────
    let db_type = detect_db_type_from_compose(&service.compose_content, &container_id);

    // ── 5. Detect whether file is gzip-compressed ────────────────────────────
    let is_gz = file_name
        .as_deref()
        .map(|n| n.ends_with(".gz"))
        .unwrap_or_else(|| bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b);

    // ── 6. Write file into container via base64 ───────────────────────────────
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let dest_path = "/tmp/rivetr_import_dump";
    let write_arg = format!("echo '{}' | base64 -d > {}", encoded, dest_path);

    let write_result = state
        .runtime
        .run_command(
            &container_id,
            vec!["sh".to_string(), "-c".to_string(), write_arg],
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to write dump into container: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    serde_json::json!({"error": format!("Failed to write dump to container: {}", e)}),
                ),
            )
        })?;

    if write_result.exit_code != 0 {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to write dump file to container",
                "stderr": write_result.stderr
            })),
        ));
    }

    // ── 7. Build the restore command ──────────────────────────────────────────
    let db_name = database_name.as_deref().unwrap_or("app").to_string();

    let restore_cmd = build_restore_command(
        &db_type,
        &db_name,
        dest_path,
        is_gz,
        &container_id,
        &service.compose_content,
    )?;

    // ── 8. Run the restore ────────────────────────────────────────────────────
    let restore_result = state
        .runtime
        .run_command(&container_id, restore_cmd)
        .await
        .map_err(|e| {
            tracing::error!("Failed to run restore command: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to run restore command: {}", e)})),
            )
        })?;

    if restore_result.exit_code != 0 {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Restore command failed",
                "stderr": restore_result.stderr,
                "stdout": restore_result.stdout
            })),
        ));
    }

    // ── 9. Clean up temp file ─────────────────────────────────────────────────
    let _ = state
        .runtime
        .run_command(
            &container_id,
            vec!["rm".to_string(), "-f".to_string(), dest_path.to_string()],
        )
        .await;

    tracing::info!(
        "Successfully imported dump into service {} container {}",
        service.name,
        container_id
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Database dump imported successfully",
        "service_id": id,
        "container": container_id
    })))
}

/// Detect the database type from the compose YAML image names.
/// Falls back to "unknown" if the type can't be determined.
fn detect_db_type_from_compose(compose_content: &str, container_hint: &str) -> String {
    // Lower-case the container hint for matching
    let hint = container_hint.to_lowercase();

    // Quick heuristic on the container name itself
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

    // Parse the compose YAML and look at `image:` fields
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

/// Construct the restore command for the detected DB type.
fn build_restore_command(
    db_type: &str,
    db_name: &str,
    dump_path: &str,
    is_gz: bool,
    _container_id: &str,
    compose_content: &str,
) -> Result<Vec<String>, (StatusCode, Json<serde_json::Value>)> {
    // Try to extract env vars from the compose YAML to build credentials
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

            if is_gz {
                // pg_restore for custom/archive format
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!(
                        "PGPASSWORD='{}' pg_restore -U {} -d {} --clean --if-exists {}",
                        password, user, effective_db, dump_path
                    ),
                ]
            } else {
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!(
                        "PGPASSWORD='{}' psql -U {} -d {} -f {}",
                        password, user, effective_db, dump_path
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

            let import_cmd = if is_gz {
                format!(
                    "gunzip -c {} | mysql -u{} -p'{}' {}",
                    dump_path, user, password, effective_db
                )
            } else {
                format!(
                    "mysql -u{} -p'{}' {} < {}",
                    user, password, effective_db, dump_path
                )
            };

            vec!["sh".to_string(), "-c".to_string(), import_cmd]
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

            let import_cmd = if is_gz {
                format!(
                    "gunzip -c {} | mariadb -u{} -p'{}' {}",
                    dump_path, user, password, effective_db
                )
            } else {
                format!(
                    "mariadb -u{} -p'{}' {} < {}",
                    user, password, effective_db, dump_path
                )
            };

            vec!["sh".to_string(), "-c".to_string(), import_cmd]
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

            vec![
                "sh".to_string(),
                "-c".to_string(),
                format!("mongorestore {} --archive={} --gzip", auth_part, dump_path),
            ]
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
/// Returns a flat map of variable name → value (literals only; skips variable
/// references like `${VAR}`).
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

        // `environment` can be a mapping (key: value) or a sequence ("KEY=value")
        if let Some(env_map) = env_section.as_mapping() {
            for (k, v) in env_map {
                if let (Some(key), Some(val)) = (k.as_str(), v.as_str()) {
                    // Skip variable references
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
