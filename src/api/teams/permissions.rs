//! Fine-grained per-resource permission handlers for team members.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{ResourcePermission, SetResourcePermissionsRequest, TeamRole, User};
use crate::AppState;

use super::super::error::ApiError;
use super::super::validation::validate_uuid;
use super::require_team_role;

/// GET /teams/:id/members/:user_id/permissions
/// List all resource permission overrides for a specific team member.
pub async fn list_member_permissions(
    State(state): State<Arc<AppState>>,
    Path((team_id, user_id)): Path<(String, String)>,
    caller: User,
) -> Result<Json<Vec<ResourcePermission>>, ApiError> {
    if let Err(e) = validate_uuid(&team_id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }
    if let Err(e) = validate_uuid(&user_id, "user_id") {
        return Err(ApiError::validation_field("user_id", e));
    }

    // Require at least admin to view another member's permissions;
    // the member themselves can view their own.
    if caller.id != user_id {
        require_team_role(&state.db, &team_id, &caller.id, TeamRole::Admin).await?;
    } else {
        require_team_role(&state.db, &team_id, &caller.id, TeamRole::Viewer).await?;
    }

    let perms = sqlx::query_as::<_, ResourcePermission>(
        "SELECT * FROM team_resource_permissions WHERE team_id = ? AND user_id = ? ORDER BY resource_type, created_at",
    )
    .bind(&team_id)
    .bind(&user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(perms))
}

/// PUT /teams/:id/members/:user_id/permissions
/// Bulk upsert resource permissions for a team member (replaces existing ones entirely).
pub async fn set_member_permissions(
    State(state): State<Arc<AppState>>,
    Path((team_id, user_id)): Path<(String, String)>,
    caller: User,
    Json(req): Json<SetResourcePermissionsRequest>,
) -> Result<Json<Vec<ResourcePermission>>, ApiError> {
    if let Err(e) = validate_uuid(&team_id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }
    if let Err(e) = validate_uuid(&user_id, "user_id") {
        return Err(ApiError::validation_field("user_id", e));
    }

    // Only admins and owners can set permissions
    require_team_role(&state.db, &team_id, &caller.id, TeamRole::Admin).await?;

    // Validate permission values
    for p in &req.permissions {
        if p.permission != "allow" && p.permission != "deny" {
            return Err(ApiError::validation_field(
                "permission",
                "Permission must be 'allow' or 'deny'",
            ));
        }
        if p.resource_type.is_empty() {
            return Err(ApiError::validation_field(
                "resource_type",
                "resource_type is required",
            ));
        }
        if p.resource_id.is_empty() {
            return Err(ApiError::validation_field(
                "resource_id",
                "resource_id is required",
            ));
        }
    }

    // Delete existing permissions for this member in this team, then insert fresh
    sqlx::query("DELETE FROM team_resource_permissions WHERE team_id = ? AND user_id = ?")
        .bind(&team_id)
        .bind(&user_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to clear member permissions: {}", e);
            ApiError::database("Failed to update permissions")
        })?;

    let now = chrono::Utc::now().to_rfc3339();

    for p in &req.permissions {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO team_resource_permissions (id, team_id, user_id, resource_type, resource_id, permission, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&team_id)
        .bind(&user_id)
        .bind(&p.resource_type)
        .bind(&p.resource_id)
        .bind(&p.permission)
        .bind(&now)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to insert resource permission: {}", e);
            ApiError::database("Failed to insert permission")
        })?;
    }

    // Return the newly stored permissions
    let perms = sqlx::query_as::<_, ResourcePermission>(
        "SELECT * FROM team_resource_permissions WHERE team_id = ? AND user_id = ? ORDER BY resource_type, created_at",
    )
    .bind(&team_id)
    .bind(&user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(perms))
}

/// DELETE /teams/:id/members/:user_id/permissions/:perm_id
/// Remove a single resource permission override.
pub async fn delete_member_permission(
    State(state): State<Arc<AppState>>,
    Path((team_id, user_id, perm_id)): Path<(String, String, String)>,
    caller: User,
) -> Result<StatusCode, ApiError> {
    if let Err(e) = validate_uuid(&team_id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }
    if let Err(e) = validate_uuid(&user_id, "user_id") {
        return Err(ApiError::validation_field("user_id", e));
    }

    // Only admins and owners can delete permissions
    require_team_role(&state.db, &team_id, &caller.id, TeamRole::Admin).await?;

    let result = sqlx::query(
        "DELETE FROM team_resource_permissions WHERE id = ? AND team_id = ? AND user_id = ?",
    )
    .bind(&perm_id)
    .bind(&team_id)
    .bind(&user_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to delete resource permission: {}", e);
        ApiError::database("Failed to delete permission")
    })?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Permission not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}
