//! Team CRUD handlers: list, get, create, update, delete.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::rbac::Ability;
use crate::db::{
    CreateTeamRequest, Team, TeamAuditAction, TeamAuditResourceType, TeamDetail,
    TeamMemberWithUser, TeamWithMemberCount, UpdateTeamRequest, User,
};
use crate::AppState;

use super::super::error::ApiError;
use super::super::validation::validate_uuid;
use super::audit::log_team_audit;
use super::{
    generate_slug, get_user_team_membership, require_team_ability, validate_create_request,
    validate_team_slug, validate_update_request,
};

/// List teams for the current user
pub async fn list_teams(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<Vec<TeamWithMemberCount>>, ApiError> {
    // Admin API token ("system") can see all teams
    let mut teams = if user.id == "system" {
        sqlx::query_as::<_, Team>("SELECT * FROM teams ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await?
    } else {
        // Get all teams the user is a member of
        sqlx::query_as::<_, Team>(
            r#"
            SELECT t.* FROM teams t
            INNER JOIN team_members tm ON t.id = tm.team_id
            WHERE tm.user_id = ?
            ORDER BY t.created_at DESC
            "#,
        )
        .bind(&user.id)
        .fetch_all(&state.db)
        .await?
    };

    // If user has no teams and is a real DB user, create a default "Personal" team for them.
    // Skip auto-create for the synthetic admin-token user ("system") which has no DB record.
    if teams.is_empty() && user.id != "system" {
        let team_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let slug = format!("personal-{}", user.id.chars().take(8).collect::<String>());

        // Use INSERT OR IGNORE to avoid UNIQUE constraint errors if slug already exists
        // (e.g. from a previous partially-failed attempt)
        sqlx::query(
            "INSERT OR IGNORE INTO teams (id, name, slug, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&team_id)
        .bind("Personal")
        .bind(&slug)
        .bind(&now)
        .bind(&now)
        .execute(&state.db)
        .await?;

        // Fetch the team (may have been created now or previously)
        let actual_team_id: Option<(String,)> =
            sqlx::query_as("SELECT id FROM teams WHERE slug = ?")
                .bind(&slug)
                .fetch_optional(&state.db)
                .await?;

        if let Some((actual_id,)) = actual_team_id {
            // Ensure user is a member (INSERT OR IGNORE handles duplicates)
            let member_id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT OR IGNORE INTO team_members (id, team_id, user_id, role, created_at) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(&member_id)
            .bind(&actual_id)
            .bind(&user.id)
            .bind("owner")
            .bind(&now)
            .execute(&state.db)
            .await?;

            tracing::info!("Ensured default Personal team for user: {}", user.email);

            // Re-fetch teams after ensuring membership
            teams = sqlx::query_as::<_, Team>(
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
        }
    }

    // Get member counts and user roles for each team
    let mut results = Vec::new();
    for team in teams {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM team_members WHERE team_id = ?")
            .bind(&team.id)
            .fetch_one(&state.db)
            .await?;

        let user_role = if user.id == "system" {
            Some("owner".to_string())
        } else {
            get_user_team_membership(&state.db, &team.id, &user.id)
                .await?
                .map(|m| m.role)
        };

        let two_factor_blocked = user.id != "system"
            && user_role.is_some()
            && super::super::authz::two_factor_blocked(&state.db, &team.id, &user.id).await?;
        results.push(TeamWithMemberCount {
            id: team.id,
            name: team.name,
            slug: team.slug,
            created_at: team.created_at,
            updated_at: team.updated_at,
            member_count: count.0,
            two_factor_blocked,
            user_role,
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

    // Check user is a member of the team and get their role
    let membership = require_team_ability(&state.db, &id, &user.id, None).await?;

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
                WHEN 'deployer' THEN 4
                WHEN 'viewer' THEN 6
                ELSE 5
            END,
            tm.created_at ASC
        "#,
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await?;

    // What the caller may do here, so the dashboard can show the right controls.
    let user_abilities = super::super::authz::role_abilities(&state.db, &id, &membership.role)
        .await?
        .iter()
        .map(|a| a.as_str().to_string())
        .collect();

    Ok(Json(TeamDetail {
        id: team.id,
        name: team.name,
        slug: team.slug,
        created_at: team.created_at,
        updated_at: team.updated_at,
        members,
        user_role: Some(membership.role),
        user_abilities,
        two_factor_blocked: user.id != "system"
            && super::super::authz::two_factor_blocked(&state.db, &id, &user.id).await?,
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

    // Add the creator as owner (skip for synthetic "system" admin-token user — no DB record)
    if user.id != "system" {
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
    }

    let team = sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    // Log audit event for team creation
    if let Err(e) = log_team_audit(
        &state.db,
        &id,
        Some(&user.id),
        TeamAuditAction::TeamCreated,
        TeamAuditResourceType::Team,
        Some(&id),
        Some(serde_json::json!({
            "name": &team.name,
            "slug": &team.slug
        })),
    )
    .await
    {
        tracing::error!("Failed to log team creation audit: {}", e);
    }

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
    require_team_ability(&state.db, &id, &user.id, Some(Ability::SettingsManage)).await?;

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

/// Toggle the `require_2fa` setting for a team (owner only)
/// PUT /api/teams/:id/2fa-enforcement
pub async fn toggle_2fa_enforcement(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
) -> Result<Json<Team>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }

    // Only the team owner can change this setting
    require_team_ability(&state.db, &id, &user.id, Some(Ability::TeamDelete)).await?;

    // Check the team exists and read current flag
    let team = sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Team not found"))?;

    let new_value = if team.require_2fa == 1 { 0_i64 } else { 1_i64 };
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query("UPDATE teams SET require_2fa = ?, updated_at = ? WHERE id = ?")
        .bind(new_value)
        .bind(&now)
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update team 2FA enforcement");
            ApiError::database("Failed to update 2FA enforcement setting")
        })?;

    let updated = sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    tracing::info!(
        team_id = %id,
        require_2fa = new_value,
        "Updated team 2FA enforcement setting"
    );

    Ok(Json(updated))
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
    require_team_ability(&state.db, &id, &user.id, Some(Ability::TeamDelete)).await?;

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
