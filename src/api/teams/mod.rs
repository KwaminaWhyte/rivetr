//! Teams API endpoints for multi-user support with role-based access control.

mod audit;
mod crud;
mod invitations;
mod members;
mod permissions;

use crate::db::{CreateTeamRequest, TeamMember, TeamRole, UpdateTeamRequest};

use super::error::{ApiError, ValidationErrorBuilder};

// Re-export public handlers
pub use audit::{list_audit_logs, log_team_audit};
pub use crud::{
    create_team, delete_team, get_team, list_teams, toggle_2fa_enforcement, update_team,
};
pub use invitations::{
    accept_invitation, create_invitation, delete_invitation, list_invitations, resend_invitation,
    validate_invitation,
};
pub use members::{invite_member, list_members, remove_member, update_member_role};
pub use permissions::{delete_member_permission, list_member_permissions, set_member_permissions};

/// Query parameters for listing audit logs
#[derive(Debug, serde::Deserialize)]
pub struct ListAuditLogsQuery {
    /// Filter by action type
    pub action: Option<String>,
    /// Filter by resource type
    pub resource_type: Option<String>,
    /// Start date for date range filter (RFC3339)
    pub start_date: Option<String>,
    /// End date for date range filter (RFC3339)
    pub end_date: Option<String>,
    /// Page number (1-indexed)
    pub page: Option<i32>,
    /// Items per page (default 20, max 100)
    pub per_page: Option<i32>,
}

/// Generate a URL-friendly slug from a name
fn generate_slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
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

/// Require that the current user has at least the specified role in the team.
/// The synthetic "system" user (admin API token) is treated as an owner of every team.
async fn require_team_role(
    pool: &sqlx::SqlitePool,
    team_id: &str,
    user_id: &str,
    required_role: TeamRole,
) -> Result<TeamMember, ApiError> {
    // Admin API token ("system") has owner-level access to every team
    if user_id == "system" {
        return Ok(TeamMember {
            id: "system".to_string(),
            team_id: team_id.to_string(),
            user_id: "system".to_string(),
            role: "owner".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        });
    }

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
