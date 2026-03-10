//! MCP HTTP server — exposes tools at /mcp endpoint.
//! Implements the MCP protocol for tool discovery and invocation.

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::AppState;

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
                    let apps =
                        sqlx::query_as::<_, crate::db::App>("SELECT * FROM apps LIMIT 50")
                            .fetch_all(&state.db)
                            .await
                            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    Ok(Json(McpResponse {
                        result: Some(serde_json::to_value(apps).unwrap_or_default()),
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
