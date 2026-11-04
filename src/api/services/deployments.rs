//! Deploy, deployment history, logs and rollback for Docker Compose services.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::Arc;

use crate::api::audit::{audit_log, ClientIp};
use crate::api::error::ApiError;
use crate::api::start_logs::{resolve_ws_user, AuthQuery};
use crate::db::{
    actions, resource_types, service_deployment_status, service_deployment_trigger,
    ServiceDeployment, ServiceDeploymentLog, ServiceDeploymentResponse, User,
};
use crate::engine::service_deploy::{queue_deployment, DeployEnv, DeployRequest, QueueError};
use crate::AppState;

#[derive(Debug, Deserialize, Default)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

fn queue_error(e: QueueError) -> ApiError {
    match e {
        QueueError::NotFound => ApiError::not_found("Service not found"),
        QueueError::Busy => ApiError::conflict(e.to_string()),
        QueueError::Invalid(m) => ApiError::bad_request(m),
        QueueError::Db(m) => ApiError::database(m),
    }
}

fn user_label(user: &User) -> String {
    if user.name.trim().is_empty() {
        user.email.clone()
    } else {
        user.name.clone()
    }
}

/// Start a deployment of a service.
///
/// POST /api/services/:id/deploy
pub async fn deploy_service(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<ServiceDeploymentResponse>), ApiError> {
    let env = Arc::new(DeployEnv::from_state(&state));
    let dep = queue_deployment(
        env,
        &id,
        DeployRequest {
            trigger: service_deployment_trigger::MANUAL.to_string(),
            user_id: Some(user.id.clone()),
            user_name: Some(user_label(&user)),
            ..Default::default()
        },
    )
    .await
    .map_err(queue_error)?;

    audit_log(
        &state,
        actions::SERVICE_DEPLOY,
        resource_types::SERVICE,
        Some(&id),
        None,
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "deployment_id": dep.id })),
    )
    .await;

    Ok((StatusCode::ACCEPTED, Json(dep.to_response())))
}

/// GET /api/services/:id/deployments
pub async fn list_service_deployments(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<ServiceDeploymentResponse>>, ApiError> {
    let rows: Vec<ServiceDeployment> = sqlx::query_as(
        "SELECT * FROM service_deployments WHERE service_id = ? ORDER BY started_at DESC LIMIT ?",
    )
    .bind(&id)
    .bind(q.limit.clamp(1, 500))
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::database(e.to_string()))?;
    Ok(Json(rows.iter().map(|d| d.to_response()).collect()))
}

async fn load_deployment(
    state: &AppState,
    service_id: &str,
    deployment_id: &str,
) -> Result<ServiceDeployment, ApiError> {
    sqlx::query_as("SELECT * FROM service_deployments WHERE id = ? AND service_id = ?")
        .bind(deployment_id)
        .bind(service_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("Deployment not found"))
}

/// GET /api/services/:id/deployments/:deployment_id
pub async fn get_service_deployment(
    State(state): State<Arc<AppState>>,
    Path((id, deployment_id)): Path<(String, String)>,
) -> Result<Json<ServiceDeploymentResponse>, ApiError> {
    let dep = load_deployment(&state, &id, &deployment_id).await?;
    Ok(Json(dep.to_response()))
}

/// GET /api/services/:id/deployments/:deployment_id/logs
pub async fn get_service_deployment_logs(
    State(state): State<Arc<AppState>>,
    Path((id, deployment_id)): Path<(String, String)>,
) -> Result<Json<Vec<ServiceDeploymentLog>>, ApiError> {
    load_deployment(&state, &id, &deployment_id).await?;
    let logs: Vec<ServiceDeploymentLog> = sqlx::query_as(
        "SELECT * FROM service_deployment_logs WHERE deployment_id = ? ORDER BY id ASC",
    )
    .bind(&deployment_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::database(e.to_string()))?;
    Ok(Json(logs))
}

/// Live deployment log stream.
///
/// GET /api/services/:id/deployments/:deployment_id/logs/stream?token=
pub async fn service_deployment_logs_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path((id, deployment_id)): Path<(String, String)>,
    Query(query): Query<AuthQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let Some(user) = resolve_ws_user(&state, query.token.as_deref()).await else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    if crate::api::authz::authorize_service(&state, &user, &id)
        .await
        .is_err()
    {
        return Err(StatusCode::FORBIDDEN);
    }
    load_deployment(&state, &id, &deployment_id)
        .await
        .map_err(|e| e.status())?;
    Ok(ws.on_upgrade(move |socket| stream_logs(socket, state, deployment_id)))
}

async fn deployment_active(state: &AppState, deployment_id: &str) -> bool {
    let s: Option<String> =
        sqlx::query_scalar("SELECT status FROM service_deployments WHERE id = ?")
            .bind(deployment_id)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten();
    s.map(|s| service_deployment_status::is_active(&s))
        .unwrap_or(false)
}

async fn stream_logs(socket: WebSocket, state: Arc<AppState>, deployment_id: String) {
    let (mut sender, mut receiver) = socket.split();
    let mut last_id: i64 = 0;
    let mut tick = tokio::time::interval(std::time::Duration::from_millis(500));

    loop {
        tokio::select! {
            _ = tick.tick() => {
                // Read the status before the logs so the final lines are never missed.
                let active = deployment_active(&state, &deployment_id).await;
                let rows: Vec<ServiceDeploymentLog> = sqlx::query_as(
                    "SELECT * FROM service_deployment_logs WHERE deployment_id = ? AND id > ? ORDER BY id ASC",
                )
                .bind(&deployment_id)
                .bind(last_id)
                .fetch_all(&state.db)
                .await
                .unwrap_or_default();
                for row in rows {
                    last_id = row.id;
                    let msg = serde_json::json!({
                        "id": row.id,
                        "deployment_id": row.deployment_id,
                        "level": row.level,
                        "message": row.message,
                        "timestamp": row.timestamp,
                    });
                    if sender.send(Message::Text(msg.to_string().into())).await.is_err() {
                        return;
                    }
                }
                if !active {
                    let _ = sender.send(Message::Text(r#"{"type":"end"}"#.into())).await;
                    return;
                }
            }
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Ping(data))) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            return;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => return,
                    _ => {}
                }
            }
        }
    }
}
