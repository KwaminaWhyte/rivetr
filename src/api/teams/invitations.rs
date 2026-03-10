//! Team invitation handlers.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use rand::Rng;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{
    CreateInvitationRequest, Team, TeamAuditAction, TeamAuditResourceType, TeamInvitation,
    TeamInvitationResponse, TeamMemberWithUser, TeamRole, User,
};
use crate::notifications::SystemEmailService;
use crate::AppState;

use super::super::error::{ApiError, ValidationErrorBuilder};
use super::super::validation::validate_uuid;
use super::audit::log_team_audit;
use super::{get_user_team_membership, require_team_role, validate_team_role};

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

    // Log audit event for invitation creation
    if let Err(e) = log_team_audit(
        &state.db,
        &id,
        Some(&user.id),
        TeamAuditAction::InvitationCreated,
        TeamAuditResourceType::Invitation,
        Some(&response.id),
        Some(serde_json::json!({
            "email": &req.email,
            "role": &req.role,
            "invited_by": &user.email
        })),
    )
    .await
    {
        tracing::error!("Failed to log invitation creation audit: {}", e);
    }

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

    // Get invitation details for audit log before deleting
    let invitation: Option<TeamInvitation> =
        sqlx::query_as("SELECT * FROM team_invitations WHERE id = ? AND team_id = ?")
            .bind(&inv_id)
            .bind(&team_id)
            .fetch_optional(&state.db)
            .await?;

    // Delete the invitation
    let result = sqlx::query("DELETE FROM team_invitations WHERE id = ? AND team_id = ?")
        .bind(&inv_id)
        .bind(&team_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Invitation not found"));
    }

    // Log audit event for invitation revocation
    if let Some(inv) = invitation {
        if let Err(e) = log_team_audit(
            &state.db,
            &team_id,
            Some(&user.id),
            TeamAuditAction::InvitationRevoked,
            TeamAuditResourceType::Invitation,
            Some(&inv_id),
            Some(serde_json::json!({
                "email": &inv.email,
                "role": &inv.role,
                "revoked_by": &user.email
            })),
        )
        .await
        {
            tracing::error!("Failed to log invitation revocation audit: {}", e);
        }
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
        role: invitation.role.clone(),
        created_at: now,
        user_name: user.name.clone(),
        user_email: user.email.clone(),
    };

    // Log audit event for invitation acceptance
    if let Err(e) = log_team_audit(
        &state.db,
        &invitation.team_id,
        Some(&user.id),
        TeamAuditAction::InvitationAccepted,
        TeamAuditResourceType::Invitation,
        Some(&invitation.id),
        Some(serde_json::json!({
            "email": &user.email,
            "name": &user.name,
            "role": &invitation.role
        })),
    )
    .await
    {
        tracing::error!("Failed to log invitation acceptance audit: {}", e);
    }

    tracing::info!(
        "User {} accepted invitation and joined team {}",
        user.email,
        invitation.team_id
    );

    Ok(Json(member))
}
