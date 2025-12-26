use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{App, CreateAppRequest, UpdateAppRequest};
use crate::AppState;

pub async fn list_apps(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<App>>, StatusCode> {
    let apps = sqlx::query_as::<_, App>("SELECT * FROM apps ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(apps))
}

pub async fn get_app(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<App>, StatusCode> {
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(app))
}

pub async fn create_app(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAppRequest>,
) -> Result<(StatusCode, Json<App>), StatusCode> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO apps (id, name, git_url, branch, dockerfile, domain, port, healthcheck, memory_limit, cpu_limit, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.git_url)
    .bind(&req.branch)
    .bind(&req.dockerfile)
    .bind(&req.domain)
    .bind(req.port)
    .bind(&req.healthcheck)
    .bind(&req.memory_limit)
    .bind(&req.cpu_limit)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create app: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(app)))
}

pub async fn update_app(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateAppRequest>,
) -> Result<Json<App>, StatusCode> {
    // Check if app exists
    let _existing = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        UPDATE apps SET
            name = COALESCE(?, name),
            git_url = COALESCE(?, git_url),
            branch = COALESCE(?, branch),
            dockerfile = COALESCE(?, dockerfile),
            domain = COALESCE(?, domain),
            port = COALESCE(?, port),
            healthcheck = COALESCE(?, healthcheck),
            memory_limit = COALESCE(?, memory_limit),
            cpu_limit = COALESCE(?, cpu_limit),
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&req.name)
    .bind(&req.git_url)
    .bind(&req.branch)
    .bind(&req.dockerfile)
    .bind(&req.domain)
    .bind(req.port)
    .bind(&req.healthcheck)
    .bind(&req.memory_limit)
    .bind(&req.cpu_limit)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(app))
}

pub async fn delete_app(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM apps WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}
