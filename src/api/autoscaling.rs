//! API handlers for autoscaling rules.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;

/// An autoscaling rule stored in the database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AutoscalingRule {
    pub id: String,
    pub app_id: String,
    pub metric: String,
    pub scale_up_threshold: f64,
    pub scale_down_threshold: f64,
    pub min_replicas: i64,
    pub max_replicas: i64,
    pub cooldown_seconds: i64,
    pub enabled: i64,
    pub last_scaled_at: Option<String>,
    pub created_at: String,
}

/// Request body for creating / updating an autoscaling rule
#[derive(Debug, Deserialize)]
pub struct AutoscalingRuleRequest {
    pub metric: String,
    pub scale_up_threshold: f64,
    pub scale_down_threshold: f64,
    pub min_replicas: Option<i64>,
    pub max_replicas: Option<i64>,
    pub cooldown_seconds: Option<i64>,
    pub enabled: Option<bool>,
}

/// List all autoscaling rules for an app
pub async fn list_rules(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<AutoscalingRule>>, StatusCode> {
    let rules = sqlx::query_as::<_, AutoscalingRule>(
        "SELECT * FROM autoscaling_rules WHERE app_id = ? ORDER BY created_at ASC",
    )
    .bind(&app_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list autoscaling rules: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(rules))
}

/// Create a new autoscaling rule for an app
pub async fn create_rule(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(req): Json<AutoscalingRuleRequest>,
) -> Result<(StatusCode, Json<AutoscalingRule>), StatusCode> {
    // Validate metric value
    if !["cpu", "memory", "request_rate"].contains(&req.metric.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let min_replicas = req.min_replicas.unwrap_or(1).clamp(1, 100);
    let max_replicas = req.max_replicas.unwrap_or(10).clamp(1, 100);
    let cooldown = req.cooldown_seconds.unwrap_or(300);
    let enabled = if req.enabled.unwrap_or(true) {
        1_i64
    } else {
        0_i64
    };

    sqlx::query(
        r#"
        INSERT INTO autoscaling_rules
            (id, app_id, metric, scale_up_threshold, scale_down_threshold,
             min_replicas, max_replicas, cooldown_seconds, enabled, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&app_id)
    .bind(&req.metric)
    .bind(req.scale_up_threshold)
    .bind(req.scale_down_threshold)
    .bind(min_replicas)
    .bind(max_replicas)
    .bind(cooldown)
    .bind(enabled)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create autoscaling rule: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let rule = sqlx::query_as::<_, AutoscalingRule>("SELECT * FROM autoscaling_rules WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(rule)))
}

/// Update an existing autoscaling rule
pub async fn update_rule(
    State(state): State<Arc<AppState>>,
    Path((app_id, rule_id)): Path<(String, String)>,
    Json(req): Json<AutoscalingRuleRequest>,
) -> Result<Json<AutoscalingRule>, StatusCode> {
    // Validate metric value
    if !["cpu", "memory", "request_rate"].contains(&req.metric.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let min_replicas = req.min_replicas.unwrap_or(1).clamp(1, 100);
    let max_replicas = req.max_replicas.unwrap_or(10).clamp(1, 100);
    let cooldown = req.cooldown_seconds.unwrap_or(300);
    let enabled = if req.enabled.unwrap_or(true) {
        1_i64
    } else {
        0_i64
    };

    let result = sqlx::query(
        r#"
        UPDATE autoscaling_rules SET
            metric = ?,
            scale_up_threshold = ?,
            scale_down_threshold = ?,
            min_replicas = ?,
            max_replicas = ?,
            cooldown_seconds = ?,
            enabled = ?
        WHERE id = ? AND app_id = ?
        "#,
    )
    .bind(&req.metric)
    .bind(req.scale_up_threshold)
    .bind(req.scale_down_threshold)
    .bind(min_replicas)
    .bind(max_replicas)
    .bind(cooldown)
    .bind(enabled)
    .bind(&rule_id)
    .bind(&app_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update autoscaling rule: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let rule = sqlx::query_as::<_, AutoscalingRule>("SELECT * FROM autoscaling_rules WHERE id = ?")
        .bind(&rule_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(rule))
}

/// Delete an autoscaling rule
pub async fn delete_rule(
    State(state): State<Arc<AppState>>,
    Path((app_id, rule_id)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM autoscaling_rules WHERE id = ? AND app_id = ?")
        .bind(&rule_id)
        .bind(&app_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete autoscaling rule: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}
