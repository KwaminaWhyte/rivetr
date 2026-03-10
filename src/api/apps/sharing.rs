use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{
    actions, resource_types, App, AppShareResponse, AppWithSharing, CreateAppShareRequest,
    TeamAuditAction, TeamAuditResourceType, User,
};
use crate::AppState;

use super::super::audit::{audit_log, extract_client_ip};
use super::super::error::ApiError;
use super::super::teams::log_team_audit;
use super::super::validation::validate_uuid;
use super::ListAppsWithSharingQuery;

/// List all teams an app is shared with
/// GET /api/apps/:id/shares
pub async fn list_app_shares(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    _user: User,
) -> Result<Json<Vec<AppShareResponse>>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Check if app exists
    let _app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Get all shares with team and user details
    let shares: Vec<AppShareResponse> = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            Option<String>,
        ),
    >(
        r#"
        SELECT
            s.id, s.app_id, s.shared_with_team_id, t.name as team_name,
            s.permission, s.created_at, s.created_by, u.name as created_by_name
        FROM app_shares s
        JOIN teams t ON t.id = s.shared_with_team_id
        LEFT JOIN users u ON u.id = s.created_by
        WHERE s.app_id = ?
        ORDER BY s.created_at DESC
        "#,
    )
    .bind(&app_id)
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(
        |(
            id,
            app_id,
            shared_with_team_id,
            shared_with_team_name,
            permission,
            created_at,
            created_by,
            created_by_name,
        )| {
            AppShareResponse {
                id,
                app_id,
                shared_with_team_id,
                shared_with_team_name,
                permission,
                created_at,
                created_by,
                created_by_name,
            }
        },
    )
    .collect();

    Ok(Json(shares))
}

/// Share an app with another team
/// POST /api/apps/:id/shares
pub async fn create_app_share(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Path(app_id): Path<String>,
    Json(req): Json<CreateAppShareRequest>,
) -> Result<(StatusCode, Json<AppShareResponse>), ApiError> {
    // Validate ID formats
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }
    if let Err(e) = validate_uuid(&req.team_id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }

    // Validate permission (currently only "view" is supported)
    if req.permission != "view" {
        return Err(ApiError::validation_field(
            "permission",
            "Only 'view' permission is currently supported".to_string(),
        ));
    }

    // Check if app exists and get the app
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Check if target team exists
    let target_team: Option<(String, String)> =
        sqlx::query_as("SELECT id, name FROM teams WHERE id = ?")
            .bind(&req.team_id)
            .fetch_optional(&state.db)
            .await?;

    let (target_team_id, target_team_name) =
        target_team.ok_or_else(|| ApiError::not_found("Target team not found"))?;

    // Cannot share with the owning team
    if let Some(ref owner_team_id) = app.team_id {
        if owner_team_id == &target_team_id {
            return Err(ApiError::bad_request(
                "Cannot share app with its owning team",
            ));
        }
    }

    // Check if share already exists
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT id FROM app_shares WHERE app_id = ? AND shared_with_team_id = ?")
            .bind(&app_id)
            .bind(&target_team_id)
            .fetch_optional(&state.db)
            .await?;

    if existing.is_some() {
        return Err(ApiError::conflict("App is already shared with this team"));
    }

    // Create the share
    let share_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO app_shares (id, app_id, shared_with_team_id, permission, created_at, created_by) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&share_id)
    .bind(&app_id)
    .bind(&target_team_id)
    .bind(&req.permission)
    .bind(&now)
    .bind(&user.id)
    .execute(&state.db)
    .await?;

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::APP_UPDATE,
        resource_types::APP,
        Some(&app.id),
        Some(&app.name),
        Some(&user.id),
        ip.as_deref(),
        Some(serde_json::json!({
            "action": "share",
            "shared_with_team_id": target_team_id,
            "shared_with_team_name": target_team_name,
            "permission": req.permission,
        })),
    )
    .await;

    // Log team audit event for the owning team
    if let Some(ref team_id) = app.team_id {
        if let Err(e) = log_team_audit(
            &state.db,
            team_id,
            Some(&user.id),
            TeamAuditAction::AppUpdated,
            TeamAuditResourceType::App,
            Some(&app.id),
            Some(serde_json::json!({
                "action": "shared",
                "app_name": app.name,
                "shared_with_team_id": target_team_id,
                "shared_with_team_name": target_team_name,
            })),
        )
        .await
        {
            tracing::warn!("Failed to log team audit event: {}", e);
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(AppShareResponse {
            id: share_id,
            app_id,
            shared_with_team_id: target_team_id,
            shared_with_team_name: target_team_name,
            permission: req.permission,
            created_at: now,
            created_by: Some(user.id.clone()),
            created_by_name: Some(user.name),
        }),
    ))
}

/// Remove app sharing with a team
/// DELETE /api/apps/:id/shares/:team_id
pub async fn delete_app_share(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Path((app_id, team_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    // Validate ID formats
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }
    if let Err(e) = validate_uuid(&team_id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }

    // Check if app exists
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Get the team name before deleting for audit log
    let team_name: Option<(String,)> = sqlx::query_as("SELECT name FROM teams WHERE id = ?")
        .bind(&team_id)
        .fetch_optional(&state.db)
        .await?;

    // Delete the share
    let result = sqlx::query("DELETE FROM app_shares WHERE app_id = ? AND shared_with_team_id = ?")
        .bind(&app_id)
        .bind(&team_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Share not found"));
    }

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::APP_UPDATE,
        resource_types::APP,
        Some(&app.id),
        Some(&app.name),
        Some(&user.id),
        ip.as_deref(),
        Some(serde_json::json!({
            "action": "unshare",
            "unshared_team_id": team_id,
            "unshared_team_name": team_name.map(|(n,)| n),
        })),
    )
    .await;

    // Log team audit event for the owning team
    if let Some(ref owner_team_id) = app.team_id {
        if let Err(e) = log_team_audit(
            &state.db,
            owner_team_id,
            Some(&user.id),
            TeamAuditAction::AppUpdated,
            TeamAuditResourceType::App,
            Some(&app.id),
            Some(serde_json::json!({
                "action": "unshared",
                "app_name": app.name,
                "unshared_team_id": team_id,
            })),
        )
        .await
        {
            tracing::warn!("Failed to log team audit event: {}", e);
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

/// List apps with sharing information for a team
/// This returns owned apps AND apps shared with the team, with a 'shared' badge indicator
/// GET /api/apps/with-sharing?team_id=xxx
pub async fn list_apps_with_sharing(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListAppsWithSharingQuery>,
) -> Result<Json<Vec<AppWithSharing>>, ApiError> {
    // Validate team_id
    if let Err(e) = validate_uuid(&query.team_id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }

    // Get owned apps
    let owned_apps: Vec<App> =
        sqlx::query_as::<_, App>("SELECT * FROM apps WHERE team_id = ? ORDER BY created_at DESC")
            .bind(&query.team_id)
            .fetch_all(&state.db)
            .await?;

    // Get shared app IDs and owner team names
    let shared_info: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT s.app_id, t.name as owner_team_name
        FROM app_shares s
        JOIN apps a ON a.id = s.app_id
        JOIN teams t ON t.id = a.team_id
        WHERE s.shared_with_team_id = ?
        "#,
    )
    .bind(&query.team_id)
    .fetch_all(&state.db)
    .await?;

    // Fetch shared apps using their IDs
    let mut shared_apps: Vec<(App, String)> = Vec::new();
    for (app_id, owner_team_name) in shared_info {
        if let Some(app) = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
            .bind(&app_id)
            .fetch_optional(&state.db)
            .await?
        {
            shared_apps.push((app, owner_team_name));
        }
    }

    // Combine into response
    let mut result: Vec<AppWithSharing> = owned_apps
        .into_iter()
        .map(|app| AppWithSharing {
            app,
            is_shared: false,
            owner_team_name: None,
        })
        .collect();

    // Add shared apps
    for (app, owner_team_name) in shared_apps {
        result.push(AppWithSharing {
            app,
            is_shared: true,
            owner_team_name: Some(owner_team_name),
        });
    }

    Ok(Json(result))
}
