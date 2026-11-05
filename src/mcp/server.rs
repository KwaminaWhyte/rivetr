//! MCP HTTP server — exposes tools at /mcp endpoint.
//! Implements the MCP protocol for tool discovery and invocation.

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::tags::{tag_names_by_resource, TaggableKind};
use crate::AppState;

/// Add a `tags` array (tag names) to each listed resource and, when the tool
/// input has a `tag`, keep only resources carrying that tag (case-insensitive).
async fn with_tags(
    db: &sqlx::SqlitePool,
    kind: TaggableKind,
    rows: Vec<(String, serde_json::Value)>,
    input: &serde_json::Value,
) -> Result<Vec<serde_json::Value>, StatusCode> {
    let tags = tag_names_by_resource(db, kind)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let wanted = input["tag"].as_str().map(str::to_lowercase);
    Ok(rows
        .into_iter()
        .filter_map(|(id, mut value)| {
            let names = tags.get(&id).cloned().unwrap_or_default();
            if let Some(w) = &wanted {
                if !names.iter().any(|n| n.to_lowercase() == *w) {
                    return None;
                }
            }
            if let Some(obj) = value.as_object_mut() {
                obj.insert("tags".into(), serde_json::json!(names));
            }
            Some(value)
        })
        .collect())
}

#[derive(Deserialize)]
pub struct McpRequest {
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct McpResponse {
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

pub async fn mcp_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<McpRequest>,
) -> Result<Json<McpResponse>, StatusCode> {
    match req.method.as_str() {
        "tools/list" => {
            let tools = super::tools::get_tools();
            Ok(Json(McpResponse {
                result: Some(serde_json::json!({"tools": tools})),
                error: None,
            }))
        }
        "tools/call" => {
            let params = req.params.unwrap_or_default();
            let tool_name = params["name"].as_str().unwrap_or("").to_string();
            let input = &params["input"];

            match tool_name.as_str() {
                "list_apps" => {
                    let apps = match input["project_id"].as_str() {
                        Some(pid) => {
                            sqlx::query_as::<_, crate::db::App>(
                                "SELECT * FROM apps WHERE project_id = ? LIMIT 50",
                            )
                            .bind(pid)
                            .fetch_all(&state.db)
                            .await
                        }
                        None => {
                            sqlx::query_as::<_, crate::db::App>("SELECT * FROM apps LIMIT 50")
                                .fetch_all(&state.db)
                                .await
                        }
                    }
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    let rows = apps
                        .into_iter()
                        .map(|a| (a.id.clone(), serde_json::to_value(a).unwrap_or_default()))
                        .collect();
                    let rows = with_tags(&state.db, TaggableKind::App, rows, input).await?;
                    Ok(Json(McpResponse {
                        result: Some(serde_json::Value::Array(rows)),
                        error: None,
                    }))
                }
                "list_tags" => {
                    let rows: Vec<(String, String, String, Option<String>, i64)> = sqlx::query_as(
                        "SELECT t.id, t.name, t.color, t.team_id, \
                         (SELECT COUNT(*) FROM resource_tags rt WHERE rt.tag_id = t.id) \
                         FROM tags t ORDER BY t.name",
                    )
                    .fetch_all(&state.db)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    let tags: Vec<serde_json::Value> = rows
                        .into_iter()
                        .map(|(id, name, color, team_id, count)| {
                            serde_json::json!({
                                "id": id,
                                "name": name,
                                "color": color,
                                "team_id": team_id,
                                "resource_count": count,
                            })
                        })
                        .collect();
                    Ok(Json(McpResponse {
                        result: Some(serde_json::json!({ "tags": tags })),
                        error: None,
                    }))
                }
                "get_app_status" => {
                    let app_id = input["app_id"].as_str().unwrap_or("");
                    let app: Option<crate::db::App> =
                        sqlx::query_as("SELECT * FROM apps WHERE id = ?")
                            .bind(app_id)
                            .fetch_optional(&state.db)
                            .await
                            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                    match app {
                        Some(a) => Ok(Json(McpResponse {
                            result: Some(serde_json::to_value(a).unwrap_or_default()),
                            error: None,
                        })),
                        None => Ok(Json(McpResponse {
                            result: None,
                            error: Some(format!("App not found: {}", app_id)),
                        })),
                    }
                }
                "deploy_app" => {
                    let app_id = input["app_id"].as_str().unwrap_or("");
                    let app: Option<crate::db::App> =
                        sqlx::query_as("SELECT * FROM apps WHERE id = ?")
                            .bind(app_id)
                            .fetch_optional(&state.db)
                            .await
                            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                    match app {
                        Some(a) => {
                            let deployment_id = uuid::Uuid::new_v4().to_string();
                            let now = chrono::Utc::now().to_rfc3339();
                            sqlx::query(
                                "INSERT INTO deployments (id, app_id, status, started_at) \
                                 VALUES (?, ?, 'pending', ?)",
                            )
                            .bind(&deployment_id)
                            .bind(&a.id)
                            .bind(&now)
                            .execute(&state.db)
                            .await
                            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                            let _ = state.deploy_tx.send((deployment_id.clone(), a)).await;

                            Ok(Json(McpResponse {
                                result: Some(serde_json::json!({
                                    "deployment_id": deployment_id,
                                    "status": "queued"
                                })),
                                error: None,
                            }))
                        }
                        None => Ok(Json(McpResponse {
                            result: None,
                            error: Some(format!("App not found: {}", app_id)),
                        })),
                    }
                }
                "get_deployment_logs" => {
                    let deployment_id = input["deployment_id"].as_str().unwrap_or("");
                    let logs: Vec<(String, String, String)> = sqlx::query_as(
                        "SELECT level, message, created_at FROM deployment_logs \
                         WHERE deployment_id = ? ORDER BY id ASC LIMIT 500",
                    )
                    .bind(deployment_id)
                    .fetch_all(&state.db)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                    let log_entries: Vec<serde_json::Value> = logs
                        .into_iter()
                        .map(|(level, message, created_at)| {
                            serde_json::json!({
                                "level": level,
                                "message": message,
                                "timestamp": created_at,
                            })
                        })
                        .collect();

                    Ok(Json(McpResponse {
                        result: Some(serde_json::json!({"logs": log_entries})),
                        error: None,
                    }))
                }
                "restart_app" => {
                    // Restart == enqueue a fresh deployment of the current config.
                    let app_id = input["app_id"].as_str().unwrap_or("");
                    let app: Option<crate::db::App> =
                        sqlx::query_as("SELECT * FROM apps WHERE id = ?")
                            .bind(app_id)
                            .fetch_optional(&state.db)
                            .await
                            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                    match app {
                        Some(a) => {
                            let deployment_id = uuid::Uuid::new_v4().to_string();
                            let now = chrono::Utc::now().to_rfc3339();
                            sqlx::query(
                                "INSERT INTO deployments (id, app_id, status, started_at) \
                                 VALUES (?, ?, 'pending', ?)",
                            )
                            .bind(&deployment_id)
                            .bind(&a.id)
                            .bind(&now)
                            .execute(&state.db)
                            .await
                            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                            let _ = state.deploy_tx.send((deployment_id.clone(), a)).await;

                            Ok(Json(McpResponse {
                                result: Some(serde_json::json!({
                                    "deployment_id": deployment_id,
                                    "status": "queued"
                                })),
                                error: None,
                            }))
                        }
                        None => Ok(Json(McpResponse {
                            result: None,
                            error: Some(format!("App not found: {}", app_id)),
                        })),
                    }
                }
                "list_deployments" => {
                    let app_id = input["app_id"].as_str();
                    let limit = input["limit"].as_i64().unwrap_or(20).clamp(1, 100);
                    let rows = match app_id {
                        Some(aid) => {
                            sqlx::query_as::<_, crate::db::Deployment>(
                                "SELECT * FROM deployments WHERE app_id = ? \
                                 ORDER BY started_at DESC LIMIT ?",
                            )
                            .bind(aid)
                            .bind(limit)
                            .fetch_all(&state.db)
                            .await
                        }
                        None => {
                            sqlx::query_as::<_, crate::db::Deployment>(
                                "SELECT * FROM deployments ORDER BY started_at DESC LIMIT ?",
                            )
                            .bind(limit)
                            .fetch_all(&state.db)
                            .await
                        }
                    }
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    Ok(Json(McpResponse {
                        result: Some(serde_json::to_value(rows).unwrap_or_default()),
                        error: None,
                    }))
                }
                "get_deployment_status" => {
                    let deployment_id = input["deployment_id"].as_str().unwrap_or("");
                    let dep: Option<crate::db::Deployment> =
                        sqlx::query_as("SELECT * FROM deployments WHERE id = ?")
                            .bind(deployment_id)
                            .fetch_optional(&state.db)
                            .await
                            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    match dep {
                        Some(d) => Ok(Json(McpResponse {
                            result: Some(serde_json::to_value(d).unwrap_or_default()),
                            error: None,
                        })),
                        None => Ok(Json(McpResponse {
                            result: None,
                            error: Some(format!("Deployment not found: {}", deployment_id)),
                        })),
                    }
                }
                "list_services" => {
                    let rows = match input["project_id"].as_str() {
                        Some(pid) => {
                            sqlx::query_as::<_, crate::db::Service>(
                                "SELECT * FROM services WHERE project_id = ? LIMIT 100",
                            )
                            .bind(pid)
                            .fetch_all(&state.db)
                            .await
                        }
                        None => {
                            sqlx::query_as::<_, crate::db::Service>(
                                "SELECT * FROM services LIMIT 100",
                            )
                            .fetch_all(&state.db)
                            .await
                        }
                    }
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    let rows = rows
                        .into_iter()
                        .map(|svc| {
                            (
                                svc.id.clone(),
                                serde_json::to_value(svc).unwrap_or_default(),
                            )
                        })
                        .collect();
                    let rows = with_tags(&state.db, TaggableKind::Service, rows, input).await?;
                    Ok(Json(McpResponse {
                        result: Some(serde_json::Value::Array(rows)),
                        error: None,
                    }))
                }
                "list_databases" => {
                    let rows = match input["project_id"].as_str() {
                        Some(pid) => {
                            sqlx::query_as::<_, crate::db::ManagedDatabase>(
                                "SELECT * FROM databases WHERE project_id = ? LIMIT 100",
                            )
                            .bind(pid)
                            .fetch_all(&state.db)
                            .await
                        }
                        None => {
                            sqlx::query_as::<_, crate::db::ManagedDatabase>(
                                "SELECT * FROM databases LIMIT 100",
                            )
                            .fetch_all(&state.db)
                            .await
                        }
                    }
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    // Mask secrets — never expose DB credentials over MCP.
                    let masked = rows
                        .into_iter()
                        .map(|db| {
                            (
                                db.id.clone(),
                                serde_json::to_value(db.to_response(false, None))
                                    .unwrap_or_default(),
                            )
                        })
                        .collect();
                    let masked =
                        with_tags(&state.db, TaggableKind::Database, masked, input).await?;
                    Ok(Json(McpResponse {
                        result: Some(serde_json::json!({ "databases": masked })),
                        error: None,
                    }))
                }
                "list_projects" => {
                    let rows =
                        sqlx::query_as::<_, crate::db::Project>("SELECT * FROM projects LIMIT 100")
                            .fetch_all(&state.db)
                            .await
                            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    Ok(Json(McpResponse {
                        result: Some(serde_json::to_value(rows).unwrap_or_default()),
                        error: None,
                    }))
                }
                _ => Ok(Json(McpResponse {
                    result: None,
                    error: Some(format!("Unknown tool: {}", tool_name)),
                })),
            }
        }
        _ => Ok(Json(McpResponse {
            result: None,
            error: Some(format!("Unknown method: {}", req.method)),
        })),
    }
}
