use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::sse::{Event, Sse},
    Json,
};
use futures::stream::Stream;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::db::{Deployment, User};
use crate::AppState;

use super::super::validation::validate_uuid;

/// Stream runtime logs for an app via SSE
/// GET /api/apps/:id/logs/stream
pub async fn stream_app_logs(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    _user: User, // Require authentication
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, (StatusCode, Json<serde_json::Value>)>
{
    // Validate app_id
    validate_uuid(&app_id, "app_id").map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e})),
        )
    })?;

    // Find the latest running deployment for this app
    let deployment = sqlx::query_as::<_, Deployment>(
        "SELECT * FROM deployments WHERE app_id = ? AND status = 'running' ORDER BY started_at DESC LIMIT 1",
    )
    .bind(&app_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Database error fetching deployment: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Database error"})),
        )
    })?;

    let deployment = deployment.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "No running container found for this app"})),
        )
    })?;

    let container_id = deployment.container_id.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "No container ID found for this deployment"})),
        )
    })?;

    tracing::info!(app_id = %app_id, container_id = %container_id, "Starting log stream for app");

    // Start docker logs with --follow
    let mut cmd = Command::new("docker");
    cmd.arg("logs")
        .arg("--follow")
        .arg("--timestamps")
        .arg("--tail")
        .arg("100") // Start with last 100 lines
        .arg(&container_id)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        tracing::error!("Failed to spawn docker logs: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to start log stream: {}", e)})),
        )
    })?;

    let stdout = child.stdout.take().ok_or_else(|| {
        tracing::error!("Failed to get stdout from docker logs");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to get log output stream"})),
        )
    })?;

    let stderr = child.stderr.take();

    let container_id_clone = container_id.clone();

    // Create the SSE stream using async_stream
    let stream = async_stream::stream! {
        // Send connected message first
        let connected_msg = serde_json::json!({
            "type": "connected",
            "container_id": container_id_clone,
        });
        yield Ok(Event::default().data(connected_msg.to_string()));

        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        // Also read stderr in parallel
        let stderr_task = if let Some(stderr) = stderr {
            let stderr_reader = BufReader::new(stderr);
            Some(tokio::spawn(async move {
                let mut stderr_lines = stderr_reader.lines();
                let mut stderr_msgs = Vec::new();
                while let Ok(Some(line)) = stderr_lines.next_line().await {
                    stderr_msgs.push(line);
                }
                stderr_msgs
            }))
        } else {
            None
        };

        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    // Parse docker log line format: 2024-01-01T00:00:00.000000000Z message
                    let (timestamp, message) = if let Some(idx) = line.find(' ') {
                        let ts = &line[..idx];
                        let msg = &line[idx + 1..];
                        (Some(ts.to_string()), msg.to_string())
                    } else {
                        (None, line)
                    };

                    let log_entry = serde_json::json!({
                        "type": "log",
                        "timestamp": timestamp,
                        "message": message,
                        "stream": "stdout",
                    });
                    yield Ok(Event::default().data(log_entry.to_string()));
                }
                Ok(None) => {
                    // Stream ended - container stopped or exited
                    let end_msg = serde_json::json!({
                        "type": "end",
                        "message": "Log stream ended"
                    });
                    yield Ok(Event::default().data(end_msg.to_string()));
                    break;
                }
                Err(e) => {
                    tracing::warn!("Error reading log line: {}", e);
                    let error_msg = serde_json::json!({
                        "type": "error",
                        "message": format!("{}", e)
                    });
                    yield Ok(Event::default().data(error_msg.to_string()));
                    break;
                }
            }
        }

        // Cleanup the stderr task
        if let Some(task) = stderr_task {
            let _ = task.await;
        }
    };

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}
