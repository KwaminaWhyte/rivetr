use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{App, DeploymentFreezeWindow, User};
use crate::engine::freeze;
use crate::AppState;

use crate::api::error::ApiError;
use crate::api::validation::validate_uuid;

/// Request body for creating a freeze window
#[derive(Debug, Deserialize)]
pub struct CreateFreezeWindowRequest {
    pub name: String,
    /// Start time in HH:MM UTC format
    pub start_time: String,
    /// End time in HH:MM UTC format
    pub end_time: String,
    /// Comma-separated days of week (0=Sun, ..., 6=Sat). Default: all days
    pub days_of_week: Option<String>,
    #[serde(default = "default_is_active")]
    pub is_active: bool,
    /// Also block preview deployments (exempt by default).
    #[serde(default)]
    pub include_previews: bool,
    /// Apply to every app and compose service of the app's team instead of
    /// this app only.
    #[serde(default)]
    pub team_wide: bool,
}

fn default_is_active() -> bool {
    true
}

/// Check the app's freeze windows for a dashboard/API deploy.
/// Returns 409 Conflict if deployment is frozen. Thin wrapper over the
/// central check in `crate::engine::freeze`.
pub async fn check_freeze_windows(
    state: &Arc<AppState>,
    app: &App,
    now: &str,
) -> Result<(), ApiError> {
    check_freeze_for(state, app, now, freeze::DeployTrigger::Manual).await
}

/// Same as [`check_freeze_windows`] for any trigger.
pub async fn check_freeze_for(
    state: &Arc<AppState>,
    app: &App,
    now: &str,
    trigger: freeze::DeployTrigger,
) -> Result<(), ApiError> {
    let now_dt = chrono::DateTime::parse_from_rfc3339(now)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());
    let hit = freeze::check_app_at(&state.db, &app.id, app.team_id.as_deref(), trigger, now_dt)
        .await
        .map_err(|e| ApiError::internal(format!("Freeze window check failed: {e}")))?;
    match hit {
        Some(hit) => Err(ApiError::conflict(hit.reason())),
        None => Ok(()),
    }
}

/// List freeze windows for an app
/// GET /api/apps/:id/freeze-windows
pub async fn list_freeze_windows(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<DeploymentFreezeWindow>>, ApiError> {
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // The app's own windows plus its team's team-wide windows.
    let windows = sqlx::query_as::<_, DeploymentFreezeWindow>(
        "SELECT * FROM deployment_freeze_windows \
         WHERE app_id = ? OR (app_id IS NULL AND ? IS NOT NULL AND team_id = ?) \
         ORDER BY created_at DESC",
    )
    .bind(&app_id)
    .bind(&app.team_id)
    .bind(&app.team_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(windows))
}

/// Create a freeze window for an app
/// POST /api/apps/:id/freeze-windows
pub async fn create_freeze_window(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(app_id): Path<String>,
    Json(req): Json<CreateFreezeWindowRequest>,
) -> Result<(StatusCode, Json<DeploymentFreezeWindow>), ApiError> {
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Only admins can create freeze windows
    if user.role != "admin" {
        return Err(ApiError::forbidden("Only admins can create freeze windows"));
    }

    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Validate time format (HH:MM)
    let time_re = regex::Regex::new(r"^\d{2}:\d{2}$").unwrap();
    if !time_re.is_match(&req.start_time) || !time_re.is_match(&req.end_time) {
        return Err(ApiError::bad_request(
            "start_time and end_time must be in HH:MM format (UTC)",
        ));
    }

    if req.team_wide && app.team_id.is_none() {
        return Err(ApiError::bad_request(
            "This app has no team, so a team-wide window cannot be created",
        ));
    }
    let window_app_id: Option<&str> = if req.team_wide {
        None
    } else {
        Some(app_id.as_str())
    };

    let window_id = Uuid::new_v4().to_string();
    let days_of_week = req
        .days_of_week
        .unwrap_or_else(|| "0,1,2,3,4,5,6".to_string());
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO deployment_freeze_windows
          (id, app_id, team_id, name, start_time, end_time, days_of_week, is_active, created_at, include_previews)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&window_id)
    .bind(window_app_id)
    .bind(&app.team_id)
    .bind(&req.name)
    .bind(&req.start_time)
    .bind(&req.end_time)
    .bind(&days_of_week)
    .bind(req.is_active as i32)
    .bind(&now)
    .bind(req.include_previews as i32)
    .execute(&state.db)
    .await?;

    let window = sqlx::query_as::<_, DeploymentFreezeWindow>(
        "SELECT * FROM deployment_freeze_windows WHERE id = ?",
    )
    .bind(&window_id)
    .fetch_one(&state.db)
    .await?;

    Ok((StatusCode::CREATED, Json(window)))
}

/// Delete a freeze window
/// DELETE /api/apps/:id/freeze-windows/:window_id
pub async fn delete_freeze_window(
    State(state): State<Arc<AppState>>,
    user: User,
    Path((app_id, window_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }
    if let Err(e) = validate_uuid(&window_id, "window_id") {
        return Err(ApiError::validation_field("window_id", e));
    }

    // Only admins can delete freeze windows
    if user.role != "admin" {
        return Err(ApiError::forbidden("Only admins can delete freeze windows"));
    }

    // The app's own window, or a team-wide window of the app's team.
    let result = sqlx::query(
        "DELETE FROM deployment_freeze_windows WHERE id = ? AND (app_id = ? \
         OR (app_id IS NULL AND team_id = (SELECT team_id FROM apps WHERE id = ?)))",
    )
    .bind(&window_id)
    .bind(&app_id)
    .bind(&app_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Freeze window not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}
