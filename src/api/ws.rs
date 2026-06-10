use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
};
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use tokio::time::{interval, Duration};

use crate::db::{Deployment, DeploymentLog};
use crate::runtime::{ExecConfig, LogStream, TtySize};
use crate::AppState;

#[derive(Deserialize)]
pub struct WsAuthQuery {
    token: Option<String>,
}

/// Resolve the user behind a WS token (None/invalid → None). Used by the
/// app/server/deployment WS authorization helpers below.
async fn ws_resolve_user(state: &Arc<AppState>, query: &WsAuthQuery) -> Option<crate::db::User> {
    let token = match query.token.as_deref() {
        Some(t) if !t.is_empty() => t,
        _ => return None,
    };
    super::auth::get_current_user(&state.db, &state.config, token)
        .await
        .ok()
}

/// Resolve the user behind a WS token and verify access to `app_id` (SEC-C3).
/// Returns false on a missing/invalid token or when the user has no access — used
/// to gate app-scoped WebSocket upgrades (terminal, runtime logs).
async fn ws_user_can_access_app(state: &Arc<AppState>, query: &WsAuthQuery, app_id: &str) -> bool {
    match ws_resolve_user(state, query).await {
        Some(user) => super::authz::authorize_app(state, &user, app_id).await.is_ok(),
        None => false,
    }
}

/// SEC-C3: verify the WS caller has access to `server_id` (server terminal = RCE).
pub async fn ws_user_can_access_server(
    state: &Arc<AppState>,
    query: &WsAuthQuery,
    server_id: &str,
) -> bool {
    match ws_resolve_user(state, query).await {
        Some(user) => super::authz::authorize_server(state, &user, server_id)
            .await
            .is_ok(),
        None => false,
    }
}

/// SEC-C3: verify the WS caller has access to `deployment_id` (resolves the app).
pub async fn ws_user_can_access_deployment(
    state: &Arc<AppState>,
    query: &WsAuthQuery,
    deployment_id: &str,
) -> bool {
    match ws_resolve_user(state, query).await {
        Some(user) => super::authz::authorize_deployment(state, &user, deployment_id)
            .await
            .is_ok(),
        None => false,
    }
}

/// WebSocket endpoint for streaming deployment logs
pub async fn deployment_logs_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(deployment_id): Path<String>,
    Query(query): Query<WsAuthQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    // SEC-C3: caller must have access to the deployment's app.
    if !ws_user_can_access_deployment(&state, &query, &deployment_id).await {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(ws.on_upgrade(move |socket| handle_log_stream(socket, state, deployment_id)))
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
            if sender
                .send(Message::Text(log_json.to_string()))
                .await
                .is_err()
            {
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
                        if sender.send(Message::Text(log_json.to_string())).await.is_err() {
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
    let result = sqlx::query_scalar::<_, String>("SELECT status FROM deployments WHERE id = ?")
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
#[allow(dead_code)]
pub async fn runtime_logs_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Query(query): Query<WsAuthQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    // SEC-C3: caller must have access to this app.
    if !ws_user_can_access_app(&state, &query, &app_id).await {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(ws.on_upgrade(move |socket| handle_runtime_log_stream(socket, state, app_id)))
}

#[allow(dead_code)]
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
            let _ = sender.send(Message::Text(error_msg.to_string())).await;
            return;
        }
        Err(e) => {
            let error_msg = serde_json::json!({
                "type": "error",
                "message": format!("Database error: {}", e)
            });
            let _ = sender.send(Message::Text(error_msg.to_string())).await;
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
            let _ = sender.send(Message::Text(error_msg.to_string())).await;
            return;
        }
    };

    // Send connection established message
    let connected_msg = serde_json::json!({
        "type": "connected",
        "container_id": container_id,
        "app_id": app_id,
    });
    if sender
        .send(Message::Text(connected_msg.to_string()))
        .await
        .is_err()
    {
        return;
    }

    // Get log stream from runtime (using logs_stream for real-time following)
    let mut log_stream = match state.runtime.logs_stream(&container_id).await {
        Ok(stream) => stream,
        Err(e) => {
            let error_msg = serde_json::json!({
                "type": "error",
                "message": format!("Failed to start log stream: {}", e)
            });
            let _ = sender.send(Message::Text(error_msg.to_string())).await;
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
                        if sender.send(Message::Text(log_json.to_string())).await.is_err() {
                            return;
                        }
                    }
                    None => {
                        // Stream ended (container stopped or closed)
                        let end_msg = serde_json::json!({
                            "type": "end",
                            "message": "Log stream ended"
                        });
                        let _ = sender.send(Message::Text(end_msg.to_string())).await;
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

/// Terminal message types for WebSocket protocol
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum TerminalMessage {
    /// Input data from the client
    Data { data: String },
    /// Resize event from the client
    Resize { cols: u16, rows: u16 },
}

/// WebSocket endpoint for interactive terminal access
/// GET /api/apps/:id/terminal
pub async fn terminal_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Query(query): Query<WsAuthQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    // SEC-C3: a container shell is RCE-equivalent — caller must have access to this app.
    if !ws_user_can_access_app(&state, &query, &app_id).await {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(ws.on_upgrade(move |socket| handle_terminal_session(socket, state, app_id)))
}

async fn handle_terminal_session(socket: WebSocket, state: Arc<AppState>, app_id: String) {
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
            let error_msg = serde_json::json!({
                "type": "error",
                "message": "No running container found for this app"
            });
            let _ = sender.send(Message::Text(error_msg.to_string())).await;
            return;
        }
        Err(e) => {
            let error_msg = serde_json::json!({
                "type": "error",
                "message": format!("Database error: {}", e)
            });
            let _ = sender.send(Message::Text(error_msg.to_string())).await;
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
            let _ = sender.send(Message::Text(error_msg.to_string())).await;
            return;
        }
    };

    // Start exec session with shell
    let exec_config = ExecConfig {
        container_id: container_id.clone(),
        cmd: vec!["/bin/sh".to_string()],
        tty: true,
    };

    let mut exec_handle = match state.runtime.exec(&exec_config).await {
        Ok(handle) => handle,
        Err(e) => {
            let error_msg = serde_json::json!({
                "type": "error",
                "message": format!("Failed to start terminal: {}", e)
            });
            let _ = sender.send(Message::Text(error_msg.to_string())).await;
            return;
        }
    };

    // Send connection established message
    let connected_msg = serde_json::json!({
        "type": "connected",
        "container_id": container_id,
        "app_id": app_id,
    });
    if sender
        .send(Message::Text(connected_msg.to_string()))
        .await
        .is_err()
    {
        return;
    }

    // Bidirectional streaming loop
    loop {
        tokio::select! {
            // Receive data from container and send to WebSocket
            Some(data) = exec_handle.stdout_rx.recv() => {
                let msg = serde_json::json!({
                    "type": "data",
                    "data": String::from_utf8_lossy(&data),
                });
                if sender.send(Message::Text(msg.to_string())).await.is_err() {
                    break;
                }
            }

            // Receive messages from WebSocket and send to container
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // Parse the terminal message
                        match serde_json::from_str::<TerminalMessage>(&text) {
                            Ok(TerminalMessage::Data { data }) => {
                                // Send input to container
                                if exec_handle.stdin_tx.send(Bytes::from(data)).await.is_err() {
                                    break;
                                }
                            }
                            Ok(TerminalMessage::Resize { cols, rows }) => {
                                // Send resize event
                                if exec_handle.resize_tx.send(TtySize { cols, rows }).await.is_err() {
                                    tracing::warn!("Failed to send resize event");
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse terminal message: {}", e);
                            }
                        }
                    }
                    Some(Ok(Message::Binary(data))) => {
                        // Handle raw binary data as input
                        if exec_handle.stdin_tx.send(Bytes::from(data.to_vec())).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        break;
                    }
                    _ => {}
                }
            }

            else => {
                // Both channels closed
                break;
            }
        }
    }

    // Send end message
    let end_msg = serde_json::json!({
        "type": "end",
        "message": "Terminal session ended"
    });
    let _ = sender.send(Message::Text(end_msg.to_string())).await;
}
