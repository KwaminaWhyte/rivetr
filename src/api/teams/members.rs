//! Team member management handlers.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{
    TeamAuditAction, TeamAuditResourceType, TeamMemberWithUser, TeamRole, UpdateMemberRoleRequest,
    User,
};
use crate::AppState;

use super::super::error::ApiError;
use super::super::validation::validate_uuid;
use super::audit::log_team_audit;
use super::{get_user_team_membership, require_team_role, validate_team_role};

/// List team members
pub async fn list_members(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
) -> Result<Json<Vec<TeamMemberWithUser>>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }

    // Check user is a member of the team
    require_team_role(&state.db, &id, &user.id, TeamRole::Viewer).await?;

    let members = sqlx::query_as::<_, TeamMemberWithUser>(
        r#"
        SELECT tm.id, tm.team_id, tm.user_id, tm.role, tm.created_at,
               u.name as user_name, u.email as user_email
        FROM team_members tm
        INNER JOIN users u ON tm.user_id = u.id
        WHERE tm.team_id = ?
        ORDER BY
            CASE tm.role
                WHEN 'owner' THEN 1
                WHEN 'admin' THEN 2
                WHEN 'developer' THEN 3
                WHEN 'viewer' THEN 4
            END,
            tm.created_at ASC
        "#,
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(members))
}

/// Invite/add a member to a team
pub async fn invite_member(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
    Json(req): Json<crate::db::InviteMemberRequest>,
) -> Result<(StatusCode, Json<TeamMemberWithUser>), ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }

    // Validate role
    let target_role =
        validate_team_role(&req.role).map_err(|e| ApiError::validation_field("role", e))?;

    // Check user has permission to manage members
    let membership = require_team_role(&state.db, &id, &user.id, TeamRole::Admin).await?;
    let user_role = membership.role_enum();

    // Check user can assign the target role
    if !user_role.can_manage_member_role(target_role) {
        return Err(ApiError::forbidden(
            "You don't have permission to assign this role",
        ));
    }

    // Find the user to invite (by email or ID)
    let target_user: Option<User> = if req.user_identifier.contains('@') {
        sqlx::query_as("SELECT * FROM users WHERE email = ?")
            .bind(&req.user_identifier)
            .fetch_optional(&state.db)
            .await?
    } else {
        sqlx::query_as("SELECT * FROM users WHERE id = ?")
            .bind(&req.user_identifier)
            .fetch_optional(&state.db)
            .await?
    };

    let target_user = target_user.ok_or_else(|| ApiError::not_found("User not found"))?;

    // Check if user is already a member
    let existing = get_user_team_membership(&state.db, &id, &target_user.id).await?;
    if existing.is_some() {
        return Err(ApiError::conflict("User is already a member of this team"));
    }

    // Add the member
    let member_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO team_members (id, team_id, user_id, role, created_at)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(&member_id)
    .bind(&id)
    .bind(&target_user.id)
    .bind(&req.role)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to add team member: {}", e);
        ApiError::database("Failed to add team member")
    })?;

    let member = TeamMemberWithUser {
        id: member_id.clone(),
        team_id: id.clone(),
        user_id: target_user.id.clone(),
        role: req.role.clone(),
        created_at: now,
        user_name: target_user.name.clone(),
        user_email: target_user.email.clone(),
    };

    // Log audit event for member addition
    if let Err(e) = log_team_audit(
        &state.db,
        &id,
        Some(&user.id),
        TeamAuditAction::MemberJoined,
        TeamAuditResourceType::Member,
        Some(&target_user.id),
        Some(serde_json::json!({
            "email": &target_user.email,
            "name": &target_user.name,
            "role": &req.role,
            "added_by": &user.email
        })),
    )
    .await
    {
        tracing::error!("Failed to log member addition audit: {}", e);
    }

    tracing::info!("Added {} to team as {}", member.user_email, member.role);

    Ok((StatusCode::CREATED, Json(member)))
}

/// Update a member's role
pub async fn update_member_role(
    State(state): State<Arc<AppState>>,
    Path((team_id, user_id)): Path<(String, String)>,
    user: User,
    Json(req): Json<UpdateMemberRoleRequest>,
) -> Result<Json<TeamMemberWithUser>, ApiError> {
    // Validate IDs
    if let Err(e) = validate_uuid(&team_id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }
    if let Err(e) = validate_uuid(&user_id, "user_id") {
        return Err(ApiError::validation_field("user_id", e));
    }

    // Validate role
    let new_role =
        validate_team_role(&req.role).map_err(|e| ApiError::validation_field("role", e))?;

    // Check user has permission to manage members
    let membership = require_team_role(&state.db, &team_id, &user.id, TeamRole::Admin).await?;
    let user_role = membership.role_enum();

    // Get the target member
    let target_membership = get_user_team_membership(&state.db, &team_id, &user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Team member not found"))?;
    let target_current_role = target_membership.role_enum();

    // Check user can manage the target's current role
    if !user_role.can_manage_member_role(target_current_role) {
        return Err(ApiError::forbidden(
            "You don't have permission to modify this member",
        ));
    }

    // Check user can assign the new role
    if !user_role.can_manage_member_role(new_role) {
        return Err(ApiError::forbidden(
            "You don't have permission to assign this role",
        ));
    }

    // Cannot change the last owner's role
    if target_current_role == TeamRole::Owner {
        let owner_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM team_members WHERE team_id = ? AND role = 'owner'",
        )
        .bind(&team_id)
        .fetch_one(&state.db)
        .await?;

        if owner_count.0 <= 1 && new_role != TeamRole::Owner {
            return Err(ApiError::bad_request(
                "Cannot change the role of the last owner. Assign another owner first.",
            ));
        }
    }

    // Update the role
    sqlx::query("UPDATE team_members SET role = ? WHERE team_id = ? AND user_id = ?")
        .bind(&req.role)
        .bind(&team_id)
        .bind(&user_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update member role: {}", e);
            ApiError::database("Failed to update member role")
        })?;

    // Get the updated member with user details
    let member = sqlx::query_as::<_, TeamMemberWithUser>(
        r#"
        SELECT tm.id, tm.team_id, tm.user_id, tm.role, tm.created_at,
               u.name as user_name, u.email as user_email
        FROM team_members tm
        INNER JOIN users u ON tm.user_id = u.id
        WHERE tm.team_id = ? AND tm.user_id = ?
        "#,
    )
    .bind(&team_id)
    .bind(&user_id)
    .fetch_one(&state.db)
    .await?;

    // Log audit event for role change
    if let Err(e) = log_team_audit(
        &state.db,
        &team_id,
        Some(&user.id),
        TeamAuditAction::RoleChanged,
        TeamAuditResourceType::Member,
        Some(&user_id),
        Some(serde_json::json!({
            "email": &member.user_email,
            "old_role": target_membership.role,
            "new_role": &req.role,
            "changed_by": &user.email
        })),
    )
    .await
    {
        tracing::error!("Failed to log role change audit: {}", e);
    }

    tracing::info!(
        "Updated {}'s role to {} in team {}",
        member.user_email,
        member.role,
        team_id
    );

    Ok(Json(member))
}

/// Remove a member from a team
pub async fn remove_member(
    State(state): State<Arc<AppState>>,
    Path((team_id, user_id)): Path<(String, String)>,
    user: User,
) -> Result<StatusCode, ApiError> {
    // Validate IDs
    if let Err(e) = validate_uuid(&team_id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }
    if let Err(e) = validate_uuid(&user_id, "user_id") {
        return Err(ApiError::validation_field("user_id", e));
    }

    // Get the target member
    let target_membership = get_user_team_membership(&state.db, &team_id, &user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Team member not found"))?;
    let target_role = target_membership.role_enum();

    // Allow self-removal (leaving the team)
    if user_id != user.id {
        // Check user has permission to manage members
        let membership = require_team_role(&state.db, &team_id, &user.id, TeamRole::Admin).await?;
        let user_role = membership.role_enum();

        // Check user can manage the target's role
        if !user_role.can_manage_member_role(target_role) {
            return Err(ApiError::forbidden(
                "You don't have permission to remove this member",
            ));
        }
    }

    // Cannot remove the last owner
    if target_role == TeamRole::Owner {
        let owner_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM team_members WHERE team_id = ? AND role = 'owner'",
        )
        .bind(&team_id)
        .fetch_one(&state.db)
        .await?;

        if owner_count.0 <= 1 {
            return Err(ApiError::bad_request(
                "Cannot remove the last owner. Assign another owner first or delete the team.",
            ));
        }
    }

    // Get user details for audit log before removing
    let removed_user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_optional(&state.db)
        .await?;

    // Remove the member
    let result = sqlx::query("DELETE FROM team_members WHERE team_id = ? AND user_id = ?")
        .bind(&team_id)
        .bind(&user_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Team member not found"));
    }

    // Log audit event for member removal
    let removed_email = removed_user
        .as_ref()
        .map(|u| u.email.as_str())
        .unwrap_or("unknown");
    let removed_name = removed_user.as_ref().map(|u| u.name.as_str()).unwrap_or("");
    let is_self_removal = user_id == user.id;

    if let Err(e) = log_team_audit(
        &state.db,
        &team_id,
        Some(&user.id),
        TeamAuditAction::MemberRemoved,
        TeamAuditResourceType::Member,
        Some(&user_id),
        Some(serde_json::json!({
            "email": removed_email,
            "name": removed_name,
            "role": target_membership.role,
            "removed_by": &user.email,
            "self_removal": is_self_removal
        })),
    )
    .await
    {
        tracing::error!("Failed to log member removal audit: {}", e);
    }

    tracing::info!("Removed user {} from team {}", user_id, team_id);

    Ok(StatusCode::NO_CONTENT)
}
