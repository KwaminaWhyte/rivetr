//! Team audit log handlers and helpers.

use axum::extract::{Path, Query, State};
use axum::Json;
use std::sync::Arc;

use crate::db::{
    TeamAuditAction, TeamAuditLog, TeamAuditLogPage, TeamAuditLogResponse, TeamAuditResourceType,
    TeamRole, User,
};
use crate::AppState;

use super::super::error::ApiError;
use super::super::validation::validate_uuid;
use super::{require_team_role, ListAuditLogsQuery};

/// Helper function to log audit events
pub async fn log_team_audit(
    pool: &sqlx::SqlitePool,
    team_id: &str,
    user_id: Option<&str>,
    action: TeamAuditAction,
    resource_type: TeamAuditResourceType,
    resource_id: Option<&str>,
    details: Option<serde_json::Value>,
) -> Result<(), sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let details_str = details.map(|d| d.to_string());

    sqlx::query(
        r#"
        INSERT INTO team_audit_logs (id, team_id, user_id, action, resource_type, resource_id, details, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(team_id)
    .bind(user_id)
    .bind(action.to_string())
    .bind(resource_type.to_string())
    .bind(resource_id)
    .bind(details_str)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(())
}

/// List audit logs for a team with pagination and filtering
pub async fn list_audit_logs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<ListAuditLogsQuery>,
    user: User,
) -> Result<Json<TeamAuditLogPage>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }

    // Check user has admin+ role to view audit logs
    require_team_role(&state.db, &id, &user.id, TeamRole::Admin).await?;

    // Pagination defaults
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    // Build the count query
    let mut count_sql = String::from("SELECT COUNT(*) FROM team_audit_logs WHERE team_id = ?");
    let mut params: Vec<String> = vec![id.clone()];

    if let Some(ref action) = query.action {
        count_sql.push_str(" AND action = ?");
        params.push(action.clone());
    }
    if let Some(ref resource_type) = query.resource_type {
        count_sql.push_str(" AND resource_type = ?");
        params.push(resource_type.clone());
    }
    if let Some(ref start_date) = query.start_date {
        count_sql.push_str(" AND created_at >= ?");
        params.push(start_date.clone());
    }
    if let Some(ref end_date) = query.end_date {
        count_sql.push_str(" AND created_at <= ?");
        params.push(end_date.clone());
    }

    // Execute count query with dynamic binding
    let total: i64 = match params.len() {
        1 => {
            let (count,): (i64,) = sqlx::query_as(&count_sql)
                .bind(&params[0])
                .fetch_one(&state.db)
                .await?;
            count
        }
        2 => {
            let (count,): (i64,) = sqlx::query_as(&count_sql)
                .bind(&params[0])
                .bind(&params[1])
                .fetch_one(&state.db)
                .await?;
            count
        }
        3 => {
            let (count,): (i64,) = sqlx::query_as(&count_sql)
                .bind(&params[0])
                .bind(&params[1])
                .bind(&params[2])
                .fetch_one(&state.db)
                .await?;
            count
        }
        4 => {
            let (count,): (i64,) = sqlx::query_as(&count_sql)
                .bind(&params[0])
                .bind(&params[1])
                .bind(&params[2])
                .bind(&params[3])
                .fetch_one(&state.db)
                .await?;
            count
        }
        5 => {
            let (count,): (i64,) = sqlx::query_as(&count_sql)
                .bind(&params[0])
                .bind(&params[1])
                .bind(&params[2])
                .bind(&params[3])
                .bind(&params[4])
                .fetch_one(&state.db)
                .await?;
            count
        }
        _ => 0,
    };

    let total_pages = ((total as f64) / (per_page as f64)).ceil() as i32;

    // Build the select query
    let mut select_sql = String::from("SELECT * FROM team_audit_logs WHERE team_id = ?");
    if query.action.is_some() {
        select_sql.push_str(" AND action = ?");
    }
    if query.resource_type.is_some() {
        select_sql.push_str(" AND resource_type = ?");
    }
    if query.start_date.is_some() {
        select_sql.push_str(" AND created_at >= ?");
    }
    if query.end_date.is_some() {
        select_sql.push_str(" AND created_at <= ?");
    }
    select_sql.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");

    // Execute select query with dynamic binding
    let logs: Vec<TeamAuditLog> = match params.len() {
        1 => {
            sqlx::query_as(&select_sql)
                .bind(&params[0])
                .bind(per_page)
                .bind(offset)
                .fetch_all(&state.db)
                .await?
        }
        2 => {
            sqlx::query_as(&select_sql)
                .bind(&params[0])
                .bind(&params[1])
                .bind(per_page)
                .bind(offset)
                .fetch_all(&state.db)
                .await?
        }
        3 => {
            sqlx::query_as(&select_sql)
                .bind(&params[0])
                .bind(&params[1])
                .bind(&params[2])
                .bind(per_page)
                .bind(offset)
                .fetch_all(&state.db)
                .await?
        }
        4 => {
            sqlx::query_as(&select_sql)
                .bind(&params[0])
                .bind(&params[1])
                .bind(&params[2])
                .bind(&params[3])
                .bind(per_page)
                .bind(offset)
                .fetch_all(&state.db)
                .await?
        }
        5 => {
            sqlx::query_as(&select_sql)
                .bind(&params[0])
                .bind(&params[1])
                .bind(&params[2])
                .bind(&params[3])
                .bind(&params[4])
                .bind(per_page)
                .bind(offset)
                .fetch_all(&state.db)
                .await?
        }
        _ => vec![],
    };

    // Fetch user details for each log entry
    let mut items = Vec::with_capacity(logs.len());
    for log in logs {
        let (user_name, user_email) = if let Some(ref uid) = log.user_id {
            let user_info: Option<(String, String)> =
                sqlx::query_as("SELECT name, email FROM users WHERE id = ?")
                    .bind(uid)
                    .fetch_optional(&state.db)
                    .await?;
            user_info
                .map(|(n, e)| (Some(n), Some(e)))
                .unwrap_or((None, None))
        } else {
            (None, None)
        };

        let details = log
            .details
            .as_ref()
            .and_then(|d| serde_json::from_str(d).ok());

        items.push(TeamAuditLogResponse {
            id: log.id,
            team_id: log.team_id,
            user_id: log.user_id,
            action: log.action,
            resource_type: log.resource_type,
            resource_id: log.resource_id,
            details,
            created_at: log.created_at,
            user_name,
            user_email,
        });
    }

    Ok(Json(TeamAuditLogPage {
        items,
        total,
        page,
        per_page,
        total_pages,
    }))
}
