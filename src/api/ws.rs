use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::time::{interval, Duration};

use crate::db::{Deployment, DeploymentLog};
use crate::runtime::LogStream;
use crate::AppState;

/// WebSocket endpoint for streaming deployment logs
pub async fn deployment_logs_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(deployment_id): Path<String>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_log_stream(socket, state, deployment_id))
}

async fn handle_log_stream(socket: WebSocket, state: Arc<AppState>, deployment_id: String) {
    let (mut sender, mut receiver) = socket.split();

    // Track the last log ID we've sent
    let mut last_log_id: i64 = 0;

    // Create an interval for polling new logs
    let mut poll_interval = interval(Duration::from_millis(500));

    // First, send all existing logs
    if let Ok(logs) = sqlx::query_as::<_, DeploymentLog>(
        "SELECT * FROM deployment_logs WHERE deployment_id = ? ORDER BY id ASC",
    )
    .bind(&deployment_id)
    .fetch_all(&state.db)
    .await
    {
        for log in logs {
            last_log_id = log.id;
            let log_json = serde_json::json!({
                "id": log.id,
                "deployment_id": log.deployment_id,
                "level": log.level,
                "message": log.message,
                "timestamp": log.timestamp,
            });
            if sender.send(Message::Text(log_json.to_string().into())).await.is_err() {
                return;
            }
        }
    }

    // Check if deployment is still in progress
    let is_active = check_deployment_active(&state, &deployment_id).await;
    if !is_active {
        // Send end message and close
        let _ = sender.send(Message::Text(r#"{"type":"end"}"#.into())).await;
        return;
    }

    // Poll for new logs while deployment is in progress
    loop {
        tokio::select! {
            // Check for new logs on interval
            _ = poll_interval.tick() => {
                // Fetch new logs
                if let Ok(new_logs) = sqlx::query_as::<_, DeploymentLog>(
                    "SELECT * FROM deployment_logs WHERE deployment_id = ? AND id > ? ORDER BY id ASC",
                )
                .bind(&deployment_id)
                .bind(last_log_id)
                .fetch_all(&state.db)
                .await
                {
                    for log in new_logs {
                        last_log_id = log.id;
                        let log_json = serde_json::json!({
                            "id": log.id,
                            "deployment_id": log.deployment_id,
                            "level": log.level,
                            "message": log.message,
                            "timestamp": log.timestamp,
                        });
                        if sender.send(Message::Text(log_json.to_string().into())).await.is_err() {
                            return;
                        }
                    }
                }

                // Check if deployment is still active
                if !check_deployment_active(&state, &deployment_id).await {
                    let _ = sender.send(Message::Text(r#"{"type":"end"}"#.into())).await;
                    return;
                }
            }

            // Handle incoming messages (for ping/pong or close)
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Ping(data))) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            return;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        return;
                    }
                    _ => {}
                }
            }
        }
    }
}

async fn check_deployment_active(state: &AppState, deployment_id: &str) -> bool {
    let result = sqlx::query_scalar::<_, String>(
        "SELECT status FROM deployments WHERE id = ?",
    )
    .bind(deployment_id)
    .fetch_optional(&state.db)
    .await;

    match result {
        Ok(Some(status)) => {
            matches!(
                status.as_str(),
                "pending" | "cloning" | "building" | "starting" | "checking"
            )
        }
        _ => false,
    }
}

/// WebSocket endpoint for streaming runtime container logs
/// GET /api/apps/:id/logs/stream
pub async fn runtime_logs_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_runtime_log_stream(socket, state, app_id))
}

async fn handle_runtime_log_stream(socket: WebSocket, state: Arc<AppState>, app_id: String) {
    let (mut sender, mut receiver) = socket.split();

    // Find the latest running deployment for this app
    let deployment = match sqlx::query_as::<_, Deployment>(
        "SELECT * FROM deployments WHERE app_id = ? AND status = 'running' ORDER BY started_at DESC LIMIT 1",
    )
    .bind(&app_id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(d)) => d,
        Ok(None) => {
            // No running deployment found
            let error_msg = serde_json::json!({
                "type": "error",
                "message": "No running container found for this app"
            });
            let _ = sender.send(Message::Text(error_msg.to_string().into())).await;
            return;
        }
        Err(e) => {
            let error_msg = serde_json::json!({
                "type": "error",
                "message": format!("Database error: {}", e)
            });
            let _ = sender.send(Message::Text(error_msg.to_string().into())).await;
            return;
        }
    };

    // Check if we have a container ID
    let container_id = match deployment.container_id {
        Some(id) => id,
        None => {
            let error_msg = serde_json::json!({
                "type": "error",
                "message": "No container ID found for this deployment"
            });
            let _ = sender.send(Message::Text(error_msg.to_string().into())).await;
            return;
        }
    };

    // Send connection established message
    let connected_msg = serde_json::json!({
        "type": "connected",
        "container_id": container_id,
        "app_id": app_id,
    });
    if sender.send(Message::Text(connected_msg.to_string().into())).await.is_err() {
        return;
    }

    // Get log stream from runtime
    let mut log_stream = match state.runtime.logs(&container_id).await {
        Ok(stream) => stream,
        Err(e) => {
            let error_msg = serde_json::json!({
                "type": "error",
                "message": format!("Failed to start log stream: {}", e)
            });
            let _ = sender.send(Message::Text(error_msg.to_string().into())).await;
            return;
        }
    };

    // Stream logs to WebSocket client
    loop {
        tokio::select! {
            // Stream logs from container
            log_line = log_stream.next() => {
                match log_line {
                    Some(line) => {
                        let stream_type = match line.stream {
                            LogStream::Stdout => "stdout",
                            LogStream::Stderr => "stderr",
                        };
                        let log_json = serde_json::json!({
                            "type": "log",
                            "timestamp": line.timestamp,
                            "message": line.message,
                            "stream": stream_type,
                        });
                        if sender.send(Message::Text(log_json.to_string().into())).await.is_err() {
                            return;
                        }
                    }
                    None => {
                        // Stream ended (container stopped or closed)
                        let end_msg = serde_json::json!({
                            "type": "end",
                            "message": "Log stream ended"
                        });
                        let _ = sender.send(Message::Text(end_msg.to_string().into())).await;
                        return;
                    }
                }
            }

            // Handle incoming messages (for ping/pong or close)
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Ping(data))) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            return;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        return;
                    }
                    _ => {}
                }
            }
        }
    }
}
