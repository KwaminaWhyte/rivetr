//! Per-container management for a compose service: status, health, resource
//! usage, restart one container, its logs, and a terminal.

use axum::{
    extract::{ws::WebSocketUpgrade, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::audit::{audit_log, ClientIp};
use crate::api::error::ApiError;
use crate::api::start_logs::{resolve_ws_user, AuthQuery};
use crate::db::{actions, resource_types, Service, User};
use crate::engine::service_deploy::images::{compose_ps, ComposePsEntry, ComposePublisher};
use crate::engine::service_deploy::runner::{CliRunner, CommandRunner, CommandSpec};
use crate::AppState;

use super::compose::get_service_compose_dir;

#[derive(Debug, Serialize)]
pub struct ServiceContainer {
    pub id: String,
    pub name: String,
    /// Compose service name
    pub service: String,
    pub image: String,
    /// running | exited | restarting | paused | created | dead
    pub state: String,
    /// healthy | unhealthy | starting | "" (no healthcheck)
    pub health: String,
    /// Docker's status text, e.g. "Up 2 hours (healthy)"
    pub status: String,
    pub created_at: String,
    pub exit_code: i64,
    pub ports: Vec<ComposePublisher>,
    pub cpu_percent: Option<f64>,
    pub memory_usage: Option<u64>,
    pub memory_limit: Option<u64>,
}

async fn load_service(state: &AppState, id: &str) -> Result<Service, ApiError> {
    sqlx::query_as("SELECT * FROM services WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("Service not found"))
}

async fn list_entries(
    state: &AppState,
    service: &Service,
) -> Result<Vec<ComposePsEntry>, ApiError> {
    let dir = get_service_compose_dir(&state.config.server.data_dir, &service.name);
    let dir = if dir.exists() {
        dir
    } else {
        std::env::temp_dir()
    };
    compose_ps(&CliRunner, &dir, &service.compose_project_name())
        .await
        .map_err(|e| ApiError::internal(format!("{:#}", e)))
}

/// Find a container of this service by (full or short) ID or name, so
/// handlers never act on containers outside the compose project.
pub(crate) fn find_container<'a>(
    entries: &'a [ComposePsEntry],
    key: &str,
) -> Option<&'a ComposePsEntry> {
    if key.len() < 6 {
        return entries.iter().find(|e| e.name == key);
    }
    entries
        .iter()
        .find(|e| e.name == key || e.id.starts_with(key) || key.starts_with(&e.id))
}

/// ID of a running container of `service` matched by name or ID.
pub(crate) async fn resolve_running_container(
    state: &AppState,
    service: &Service,
    key: &str,
) -> Result<String, ApiError> {
    let entries = list_entries(state, service).await?;
    find_container(&entries, key)
        .filter(|e| e.state == "running")
        .map(|e| e.id.clone())
        .ok_or_else(|| ApiError::not_found("No running container with that name"))
}

/// GET /api/services/:id/containers
pub async fn list_service_containers(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<ServiceContainer>>, ApiError> {
    let service = load_service(&state, &id).await?;
    let entries = list_entries(&state, &service).await?;

    let stats = futures::future::join_all(entries.iter().map(|e| {
        let runtime = state.runtime.clone();
        let running = e.state == "running";
        let cid = e.id.clone();
        async move {
            if running {
                runtime.stats(&cid).await.ok()
            } else {
                None
            }
        }
    }))
    .await;

    let mut out: Vec<ServiceContainer> = entries
        .into_iter()
        .zip(stats)
        .map(|(e, s)| ServiceContainer {
            id: e.id,
            name: e.name,
            service: e.service,
            image: e.image,
            state: e.state,
            health: e.health,
            status: e.status,
            created_at: e.created_at,
            exit_code: e.exit_code,
            ports: e.publishers.unwrap_or_default(),
            cpu_percent: s.as_ref().map(|s| s.cpu_percent),
            memory_usage: s.as_ref().map(|s| s.memory_usage),
            memory_limit: s.as_ref().map(|s| s.memory_limit),
        })
        .collect();
    out.sort_by(|a, b| a.service.cmp(&b.service).then(a.name.cmp(&b.name)));
    Ok(Json(out))
}

/// POST /api/services/:id/containers/:container/restart
pub async fn restart_service_container(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path((id, container)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let service = load_service(&state, &id).await?;
    let entries = list_entries(&state, &service).await?;
    let entry = find_container(&entries, &container)
        .ok_or_else(|| ApiError::not_found("Container not found in this service"))?;

    let out = CliRunner
        .run(
            &CommandSpec::new("docker", &["restart", &entry.id])
                .timeout(std::time::Duration::from_secs(120)),
            None,
        )
        .await
        .map_err(|e| ApiError::internal(format!("{:#}", e)))?;
    if !out.success {
        return Err(ApiError::internal(format!(
            "docker restart failed: {}",
            out.error_tail()
        )));
    }

    audit_log(
        &state,
        actions::SERVICE_CONTAINER_RESTART,
        resource_types::SERVICE,
        Some(&service.id),
        Some(&service.name),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "container": entry.name })),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct ContainerLogsQuery {
    #[serde(default = "default_lines")]
    pub lines: u32,
}

fn default_lines() -> u32 {
    200
}

#[derive(Debug, Serialize)]
pub struct ContainerLogLine {
    pub timestamp: String,
    pub stream: &'static str,
    pub message: String,
}

fn split_timestamp(line: &str) -> (String, String) {
    match line.split_once(' ') {
        Some((ts, rest)) if ts.len() >= 20 && ts.as_bytes()[4] == b'-' => {
            (ts.to_string(), rest.to_string())
        }
        _ => (String::new(), line.to_string()),
    }
}

/// GET /api/services/:id/containers/:container/logs?lines=200
pub async fn get_service_container_logs(
    State(state): State<Arc<AppState>>,
    Path((id, container)): Path<(String, String)>,
    Query(q): Query<ContainerLogsQuery>,
) -> Result<Json<Vec<ContainerLogLine>>, ApiError> {
    let service = load_service(&state, &id).await?;
    let entries = list_entries(&state, &service).await?;
    let entry = find_container(&entries, &container)
        .ok_or_else(|| ApiError::not_found("Container not found in this service"))?;
    let tail = q.lines.clamp(1, 5000).to_string();
    let out = CliRunner
        .run(
            &CommandSpec::new(
                "docker",
                &["logs", "--timestamps", "--tail", &tail, &entry.id],
            )
            .timeout(std::time::Duration::from_secs(30)),
            None,
        )
        .await
        .map_err(|e| ApiError::internal(format!("{:#}", e)))?;
    if !out.success {
        return Err(ApiError::internal(format!(
            "docker logs failed: {}",
            out.error_tail()
        )));
    }
    let mut lines: Vec<ContainerLogLine> = out
        .stdout
        .lines()
        .map(|l| (l, "stdout"))
        .chain(out.stderr.lines().map(|l| (l, "stderr")))
        .map(|(l, stream)| {
            let (timestamp, message) = split_timestamp(l);
            ContainerLogLine {
                timestamp,
                stream,
                message,
            }
        })
        .collect();
    lines.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    Ok(Json(lines))
}

/// Interactive shell in one container of the service.
///
/// GET /api/services/:id/containers/:container/terminal?token=
pub async fn service_container_terminal_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path((id, container)): Path<(String, String)>,
    Query(query): Query<AuthQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    // A container shell is RCE-equivalent: the caller must have access to the service.
    let Some(user) = resolve_ws_user(&state, query.token.as_deref()).await else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let service = crate::api::authz::authorize_service(&state, &user, &id)
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;
    let entries = list_entries(&state, &service)
        .await
        .map_err(|e| e.status())?;
    let entry = find_container(&entries, &container)
        .filter(|e| e.state == "running")
        .ok_or(StatusCode::NOT_FOUND)?
        .clone();

    Ok(ws.on_upgrade(move |socket| async move {
        let (sender, receiver) = socket.split();
        crate::api::ws::bridge_terminal(
            sender,
            receiver,
            state,
            entry.id.clone(),
            serde_json::json!({ "service_id": service.id, "container": entry.name }),
        )
        .await;
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, name: &str) -> ComposePsEntry {
        ComposePsEntry {
            id: id.into(),
            name: name.into(),
            ..Default::default()
        }
    }

    #[test]
    fn only_containers_of_the_project_are_found() {
        let entries = vec![
            entry("0a3f81448176", "rivetr-svc-shop-web-1"),
            entry("9b1c2d3e4f50", "rivetr-svc-shop-db-1"),
        ];
        assert_eq!(
            find_container(&entries, "rivetr-svc-shop-db-1").unwrap().id,
            "9b1c2d3e4f50"
        );
        assert_eq!(
            find_container(&entries, "0a3f81448176098f854cc27e4ae942c1")
                .unwrap()
                .name,
            "rivetr-svc-shop-web-1"
        );
        assert!(find_container(&entries, "deadbeef0000").is_none());
        // Too-short prefixes never match by id.
        assert!(find_container(&entries, "0a").is_none());
    }

    #[test]
    fn docker_log_timestamps_are_split() {
        let (ts, msg) = split_timestamp("2026-11-04T08:15:35.123456789Z listening on :3000");
        assert_eq!(ts, "2026-11-04T08:15:35.123456789Z");
        assert_eq!(msg, "listening on :3000");
        assert_eq!(split_timestamp("no timestamp").1, "no timestamp");
    }
}
