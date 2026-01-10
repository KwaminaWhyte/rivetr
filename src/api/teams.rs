//! Teams API endpoints for multi-user support with role-based access control.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use rand::Rng;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{
    CreateInvitationRequest, CreateTeamRequest, InviteMemberRequest, Team, TeamDetail,
    TeamInvitation, TeamInvitationResponse, TeamMember, TeamMemberWithUser, TeamRole,
    TeamWithMemberCount, UpdateMemberRoleRequest, UpdateTeamRequest, User,
};
use crate::notifications::SystemEmailService;
use crate::AppState;

use super::error::{ApiError, ValidationErrorBuilder};
use super::validation::validate_uuid;

/// Generate a URL-friendly slug from a name
fn generate_slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else if c.is_whitespace() || c == '-' || c == '_' {
                '-'
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Validate a team name
fn validate_team_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Team name is required".to_string());
    }

    if name.len() > 100 {
        return Err("Team name is too long (max 100 characters)".to_string());
    }

    if name.len() < 2 {
        return Err("Team name is too short (min 2 characters)".to_string());
    }

    Ok(())
}

/// Validate a team slug
fn validate_team_slug(slug: &str) -> Result<(), String> {
    if slug.is_empty() {
        return Err("Team slug is required".to_string());
    }

    if slug.len() > 100 {
        return Err("Team slug is too long (max 100 characters)".to_string());
    }

    if slug.len() < 2 {
        return Err("Team slug is too short (min 2 characters)".to_string());
    }

    // Slug must be lowercase alphanumeric with dashes
    if !slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err("Team slug must be lowercase alphanumeric with dashes only".to_string());
    }

    // Cannot start or end with dash
    if slug.starts_with('-') || slug.ends_with('-') {
        return Err("Team slug cannot start or end with a dash".to_string());
    }

    // Cannot have consecutive dashes
    if slug.contains("--") {
        return Err("Team slug cannot contain consecutive dashes".to_string());
    }

    Ok(())
}

/// Validate a team role string
fn validate_team_role(role: &str) -> Result<TeamRole, String> {
    role.parse::<TeamRole>()
        .map_err(|_| "Invalid role. Must be one of: owner, admin, developer, viewer".to_string())
}

/// Validate a CreateTeamRequest
fn validate_create_request(req: &CreateTeamRequest) -> Result<(), ApiError> {
    let mut errors = ValidationErrorBuilder::new();

    if let Err(e) = validate_team_name(&req.name) {
        errors.add("name", &e);
    }

    if let Some(ref slug) = req.slug {
        if let Err(e) = validate_team_slug(slug) {
            errors.add("slug", &e);
        }
    }

    errors.finish()
}

/// Validate an UpdateTeamRequest
fn validate_update_request(req: &UpdateTeamRequest) -> Result<(), ApiError> {
    let mut errors = ValidationErrorBuilder::new();

    if let Some(ref name) = req.name {
        if let Err(e) = validate_team_name(name) {
            errors.add("name", &e);
        }
    }

    if let Some(ref slug) = req.slug {
        if let Err(e) = validate_team_slug(slug) {
            errors.add("slug", &e);
        }
    }

    errors.finish()
}

/// Get the current user's membership in a team
async fn get_user_team_membership(
    pool: &sqlx::SqlitePool,
    team_id: &str,
    user_id: &str,
) -> Result<Option<TeamMember>, sqlx::Error> {
    sqlx::query_as::<_, TeamMember>("SELECT * FROM team_members WHERE team_id = ? AND user_id = ?")
        .bind(team_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

/// Require that the current user has at least the specified role in the team
async fn require_team_role(
    pool: &sqlx::SqlitePool,
    team_id: &str,
    user_id: &str,
    required_role: TeamRole,
) -> Result<TeamMember, ApiError> {
    let membership = get_user_team_membership(pool, team_id, user_id)
        .await?
        .ok_or_else(|| ApiError::forbidden("You are not a member of this team"))?;

    let user_role = membership.role_enum();
    if !user_role.has_at_least(required_role) {
        return Err(ApiError::forbidden(format!(
            "This action requires {} role or higher",
            required_role
        )));
    }

    Ok(membership)
}

/// List teams for the current user
pub async fn list_teams(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<Vec<TeamWithMemberCount>>, ApiError> {
    // Get all teams the user is a member of
    let teams = sqlx::query_as::<_, Team>(
        r#"
        SELECT t.* FROM teams t
        INNER JOIN team_members tm ON t.id = tm.team_id
        WHERE tm.user_id = ?
        ORDER BY t.created_at DESC
        "#,
    )
    .bind(&user.id)
    .fetch_all(&state.db)
    .await?;

    // Get member counts and user roles for each team
    let mut results = Vec::new();
    for team in teams {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM team_members WHERE team_id = ?")
            .bind(&team.id)
            .fetch_one(&state.db)
            .await?;

        let membership = get_user_team_membership(&state.db, &team.id, &user.id).await?;

        results.push(TeamWithMemberCount {
            id: team.id,
            name: team.name,
            slug: team.slug,
            created_at: team.created_at,
            updated_at: team.updated_at,
            member_count: count.0,
            user_role: membership.map(|m| m.role),
        });
    }

    Ok(Json(results))
}

/// Get a specific team by ID
pub async fn get_team(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
) -> Result<Json<TeamDetail>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }

    // Check user is a member of the team
    let _membership = require_team_role(&state.db, &id, &user.id, TeamRole::Viewer).await?;

    let team = sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Team not found"))?;

    // Get members with user details
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

    Ok(Json(TeamDetail {
        id: team.id,
        name: team.name,
        slug: team.slug,
        created_at: team.created_at,
        updated_at: team.updated_at,
        members,
    }))
}

/// Create a new team
pub async fn create_team(
    State(state): State<Arc<AppState>>,
    user: User,
    Json(req): Json<CreateTeamRequest>,
) -> Result<(StatusCode, Json<Team>), ApiError> {
    // Validate request
    validate_create_request(&req)?;

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let slug = req.slug.unwrap_or_else(|| generate_slug(&req.name));

    // Validate the generated/provided slug
    validate_team_slug(&slug).map_err(|e| ApiError::validation_field("slug", e))?;

    // Create team
    sqlx::query(
        r#"
        INSERT INTO teams (id, name, slug, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&slug)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create team: {}", e);
        if e.to_string().contains("UNIQUE constraint failed") {
            ApiError::conflict("A team with this slug already exists")
        } else {
            ApiError::database("Failed to create team")
        }
    })?;

    // Add the creator as owner
    let member_id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO team_members (id, team_id, user_id, role, created_at)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(&member_id)
    .bind(&id)
    .bind(&user.id)
    .bind("owner")
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to add team owner: {}", e);
        ApiError::database("Failed to create team membership")
    })?;

    let team = sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    tracing::info!("Created team '{}' with owner {}", team.name, user.email);

    Ok((StatusCode::CREATED, Json(team)))
}

/// Update a team
pub async fn update_team(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
    Json(req): Json<UpdateTeamRequest>,
) -> Result<Json<Team>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }

    // Validate request
    validate_update_request(&req)?;

    // Check user has admin+ role
    require_team_role(&state.db, &id, &user.id, TeamRole::Admin).await?;

    // Check if team exists
    let _existing = sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Team not found"))?;

    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        UPDATE teams SET
            name = COALESCE(?, name),
            slug = COALESCE(?, slug),
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&req.name)
    .bind(&req.slug)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update team: {}", e);
        if e.to_string().contains("UNIQUE constraint failed") {
            ApiError::conflict("A team with this slug already exists")
        } else {
            ApiError::database("Failed to update team")
        }
    })?;

    let team = sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(team))
}

/// Delete a team (owner only)
pub async fn delete_team(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
) -> Result<StatusCode, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }

    // Check user is owner
    require_team_role(&state.db, &id, &user.id, TeamRole::Owner).await?;

    // Delete the team (members will be cascade deleted)
    let result = sqlx::query("DELETE FROM teams WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Team not found"));
    }

    tracing::info!("Deleted team {} by user {}", id, user.email);

    Ok(StatusCode::NO_CONTENT)
}

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
    Json(req): Json<InviteMemberRequest>,
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
        id: member_id,
        team_id: id,
        user_id: target_user.id,
        role: req.role,
        created_at: now,
        user_name: target_user.name,
        user_email: target_user.email,
    };

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

    // Remove the member
    let result = sqlx::query("DELETE FROM team_members WHERE team_id = ? AND user_id = ?")
        .bind(&team_id)
        .bind(&user_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Team member not found"));
    }

    tracing::info!("Removed user {} from team {}", user_id, team_id);

    Ok(StatusCode::NO_CONTENT)
}

/// Generate a secure random token for invitations
fn generate_invitation_token() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();
    (0..48)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Validate email format (basic validation)
fn validate_email(email: &str) -> Result<(), String> {
    if email.is_empty() {
        return Err("Email is required".to_string());
    }
    if !email.contains('@') || !email.contains('.') {
        return Err("Invalid email format".to_string());
    }
    if email.len() > 255 {
        return Err("Email is too long (max 255 characters)".to_string());
    }
    Ok(())
}

/// List pending invitations for a team
pub async fn list_invitations(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
) -> Result<Json<Vec<TeamInvitationResponse>>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }

    // Check user has admin+ role to view invitations
    require_team_role(&state.db, &id, &user.id, TeamRole::Admin).await?;

    // Get all pending invitations (not accepted)
    let invitations = sqlx::query_as::<_, TeamInvitation>(
        r#"
        SELECT * FROM team_invitations
        WHERE team_id = ? AND accepted_at IS NULL
        ORDER BY created_at DESC
        "#,
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await?;

    // Get inviter names for each invitation
    let mut results = Vec::new();
    for inv in invitations {
        let inviter: Option<(String,)> = sqlx::query_as("SELECT name FROM users WHERE id = ?")
            .bind(&inv.created_by)
            .fetch_optional(&state.db)
            .await?;

        let mut response: TeamInvitationResponse = inv.into();
        response.inviter_name = inviter.map(|u| u.0);
        results.push(response);
    }

    Ok(Json(results))
}

/// Create a new invitation
pub async fn create_invitation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
    Json(req): Json<CreateInvitationRequest>,
) -> Result<(StatusCode, Json<TeamInvitationResponse>), ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }

    // Validate email
    let mut errors = ValidationErrorBuilder::new();
    if let Err(e) = validate_email(&req.email) {
        errors.add("email", &e);
    }
    errors.finish()?;

    // Validate role
    let target_role =
        validate_team_role(&req.role).map_err(|e| ApiError::validation_field("role", e))?;

    // Check user has admin+ role to create invitations
    let membership = require_team_role(&state.db, &id, &user.id, TeamRole::Admin).await?;
    let user_role = membership.role_enum();

    // Check user can assign the target role
    if !user_role.can_manage_member_role(target_role) {
        return Err(ApiError::forbidden(
            "You don't have permission to assign this role",
        ));
    }

    // Check if team exists
    let team = sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Team not found"))?;

    // Check if user with this email is already a member
    let existing_user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE email = ?")
        .bind(&req.email)
        .fetch_optional(&state.db)
        .await?;

    if let Some(existing) = existing_user {
        let existing_membership = get_user_team_membership(&state.db, &id, &existing.id).await?;
        if existing_membership.is_some() {
            return Err(ApiError::conflict(
                "A user with this email is already a member of this team",
            ));
        }
    }

    // Check for existing pending invitation for this email
    let existing_invitation: Option<TeamInvitation> = sqlx::query_as(
        r#"
        SELECT * FROM team_invitations
        WHERE team_id = ? AND email = ? AND accepted_at IS NULL
        "#,
    )
    .bind(&id)
    .bind(&req.email)
    .fetch_optional(&state.db)
    .await?;

    if existing_invitation.is_some() {
        return Err(ApiError::conflict(
            "A pending invitation already exists for this email",
        ));
    }

    // Create the invitation
    let inv_id = Uuid::new_v4().to_string();
    let token = generate_invitation_token();
    let now = chrono::Utc::now();
    let expires_at = now + chrono::Duration::days(7);

    sqlx::query(
        r#"
        INSERT INTO team_invitations (id, team_id, email, role, token, expires_at, created_by, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&inv_id)
    .bind(&id)
    .bind(&req.email)
    .bind(&req.role)
    .bind(&token)
    .bind(expires_at.to_rfc3339())
    .bind(&user.id)
    .bind(now.to_rfc3339())
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create invitation: {}", e);
        ApiError::database("Failed to create invitation")
    })?;

    let response = TeamInvitationResponse {
        id: inv_id,
        team_id: id.clone(),
        email: req.email.clone(),
        role: req.role.clone(),
        expires_at: expires_at.to_rfc3339(),
        accepted_at: None,
        created_by: user.id.clone(),
        created_at: now.to_rfc3339(),
        team_name: Some(team.name.clone()),
        inviter_name: Some(user.name.clone()),
    };

    tracing::info!(
        "Created invitation for {} to team {} by {}",
        req.email,
        id,
        user.email
    );

    // Send invitation email (non-blocking, log errors but don't fail the request)
    let email_service = SystemEmailService::new(state.config.email.clone());
    if email_service.is_enabled() {
        // Build the accept URL
        let base_url = state
            .config
            .server
            .external_url
            .as_deref()
            .unwrap_or("http://localhost:8080");
        let accept_url = format!("{}/invitations/accept?token={}", base_url, token);

        match email_service
            .send_invitation_email(
                &req.email,
                &team.name,
                &req.role,
                &user.name,
                &accept_url,
                7, // 7 days expiry
            )
            .await
        {
            Ok(()) => {
                tracing::info!(
                    to = %req.email,
                    team = %team.name,
                    "Invitation email sent successfully"
                );
            }
            Err(e) => {
                tracing::error!(
                    to = %req.email,
                    team = %team.name,
                    error = %e,
                    "Failed to send invitation email"
                );
            }
        }
    } else {
        tracing::debug!(
            "Email not configured, invitation email not sent to {}",
            req.email
        );
    }

    Ok((StatusCode::CREATED, Json(response)))
}

/// Delete/revoke a pending invitation
pub async fn delete_invitation(
    State(state): State<Arc<AppState>>,
    Path((team_id, inv_id)): Path<(String, String)>,
    user: User,
) -> Result<StatusCode, ApiError> {
    // Validate IDs
    if let Err(e) = validate_uuid(&team_id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }
    if let Err(e) = validate_uuid(&inv_id, "invitation_id") {
        return Err(ApiError::validation_field("invitation_id", e));
    }

    // Check user has admin+ role
    require_team_role(&state.db, &team_id, &user.id, TeamRole::Admin).await?;

    // Delete the invitation
    let result = sqlx::query("DELETE FROM team_invitations WHERE id = ? AND team_id = ?")
        .bind(&inv_id)
        .bind(&team_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Invitation not found"));
    }

    tracing::info!("Deleted invitation {} from team {}", inv_id, team_id);

    Ok(StatusCode::NO_CONTENT)
}

/// Resend invitation email
pub async fn resend_invitation(
    State(state): State<Arc<AppState>>,
    Path((team_id, inv_id)): Path<(String, String)>,
    user: User,
) -> Result<StatusCode, ApiError> {
    // Validate IDs
    if let Err(e) = validate_uuid(&team_id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }
    if let Err(e) = validate_uuid(&inv_id, "invitation_id") {
        return Err(ApiError::validation_field("invitation_id", e));
    }

    // Check user has admin+ role
    require_team_role(&state.db, &team_id, &user.id, TeamRole::Admin).await?;

    // Get the invitation
    let invitation = sqlx::query_as::<_, TeamInvitation>(
        "SELECT * FROM team_invitations WHERE id = ? AND team_id = ?",
    )
    .bind(&inv_id)
    .bind(&team_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Invitation not found"))?;

    // Check if already accepted
    if invitation.is_accepted() {
        return Err(ApiError::bad_request(
            "Cannot resend an accepted invitation",
        ));
    }

    // Check if expired
    if invitation.is_expired() {
        return Err(ApiError::bad_request(
            "Cannot resend an expired invitation. Please create a new invitation.",
        ));
    }

    // Get team name
    let team = sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = ?")
        .bind(&team_id)
        .fetch_one(&state.db)
        .await?;

    // Get inviter name
    let inviter: Option<User> = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&invitation.created_by)
        .fetch_optional(&state.db)
        .await?;
    let inviter_name = inviter
        .map(|u| u.name)
        .unwrap_or_else(|| "A team member".to_string());

    // Send invitation email
    let email_service = SystemEmailService::new(state.config.email.clone());
    if !email_service.is_enabled() {
        return Err(ApiError::bad_request(
            "Email is not configured. Cannot resend invitation.",
        ));
    }

    // Build the accept URL
    let base_url = state
        .config
        .server
        .external_url
        .as_deref()
        .unwrap_or("http://localhost:8080");
    let accept_url = format!("{}/invitations/accept?token={}", base_url, invitation.token);

    email_service
        .send_invitation_email(
            &invitation.email,
            &team.name,
            &invitation.role,
            &inviter_name,
            &accept_url,
            7, // 7 days expiry
        )
        .await
        .map_err(|e| {
            tracing::error!(
                to = %invitation.email,
                team = %team.name,
                error = %e,
                "Failed to resend invitation email"
            );
            ApiError::internal("Failed to send invitation email")
        })?;

    tracing::info!(
        to = %invitation.email,
        team = %team.name,
        "Invitation email resent successfully"
    );

    Ok(StatusCode::NO_CONTENT)
}

/// Validate an invitation token (public endpoint)
pub async fn validate_invitation(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Result<Json<TeamInvitationResponse>, ApiError> {
    // Get the invitation by token
    let invitation =
        sqlx::query_as::<_, TeamInvitation>("SELECT * FROM team_invitations WHERE token = ?")
            .bind(&token)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| ApiError::not_found("Invitation not found"))?;

    // Check if already accepted
    if invitation.is_accepted() {
        return Err(ApiError::bad_request(
            "This invitation has already been accepted",
        ));
    }

    // Check if expired
    if invitation.is_expired() {
        return Err(ApiError::bad_request("This invitation has expired"));
    }

    // Get team name
    let team: Option<Team> = sqlx::query_as("SELECT * FROM teams WHERE id = ?")
        .bind(&invitation.team_id)
        .fetch_optional(&state.db)
        .await?;

    // Get inviter name
    let inviter: Option<(String,)> = sqlx::query_as("SELECT name FROM users WHERE id = ?")
        .bind(&invitation.created_by)
        .fetch_optional(&state.db)
        .await?;

    let mut response: TeamInvitationResponse = invitation.into();
    response.team_name = team.map(|t| t.name);
    response.inviter_name = inviter.map(|u| u.0);

    Ok(Json(response))
}

/// Accept an invitation (requires authenticated user)
pub async fn accept_invitation(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
    user: User,
) -> Result<Json<TeamMemberWithUser>, ApiError> {
    // Get the invitation by token
    let invitation =
        sqlx::query_as::<_, TeamInvitation>("SELECT * FROM team_invitations WHERE token = ?")
            .bind(&token)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| ApiError::not_found("Invitation not found"))?;

    // Check if already accepted
    if invitation.is_accepted() {
        return Err(ApiError::bad_request(
            "This invitation has already been accepted",
        ));
    }

    // Check if expired
    if invitation.is_expired() {
        return Err(ApiError::bad_request("This invitation has expired"));
    }

    // Optionally verify the user's email matches the invitation (for security)
    // Note: This is optional - you may want to allow any authenticated user to accept
    // if they have the token, or you may want strict email matching
    // For now, we'll check that the email matches (case-insensitive)
    if user.email.to_lowercase() != invitation.email.to_lowercase() {
        return Err(ApiError::forbidden(
            "This invitation was sent to a different email address",
        ));
    }

    // Check if user is already a member
    let existing_membership =
        get_user_team_membership(&state.db, &invitation.team_id, &user.id).await?;
    if existing_membership.is_some() {
        return Err(ApiError::conflict("You are already a member of this team"));
    }

    let now = chrono::Utc::now().to_rfc3339();

    // Add the user as a team member
    let member_id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO team_members (id, team_id, user_id, role, created_at)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(&member_id)
    .bind(&invitation.team_id)
    .bind(&user.id)
    .bind(&invitation.role)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to add team member: {}", e);
        ApiError::database("Failed to add team member")
    })?;

    // Mark the invitation as accepted
    sqlx::query("UPDATE team_invitations SET accepted_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&invitation.id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update invitation: {}", e);
            ApiError::database("Failed to update invitation")
        })?;

    let member = TeamMemberWithUser {
        id: member_id,
        team_id: invitation.team_id.clone(),
        user_id: user.id.clone(),
        role: invitation.role,
        created_at: now,
        user_name: user.name,
        user_email: user.email.clone(),
    };

    tracing::info!(
        "User {} accepted invitation and joined team {}",
        user.email,
        invitation.team_id
    );

    Ok(Json(member))
}
