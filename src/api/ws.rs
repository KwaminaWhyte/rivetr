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

use crate::db::DeploymentLog;
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
