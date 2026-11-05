//! Team roles: the built-in presets and custom roles built from abilities.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::rbac::{
    self, abilities_to_json, Ability, AbilityInfo, AbilitySet, CustomRoleRequest,
    CustomRoleResponse, RolePreset, TeamRoleRow,
};
use crate::db::User;
use crate::AppState;

use super::super::authz::role_abilities;
use super::super::error::ApiError;
use super::super::validation::validate_uuid;
use super::require_team_ability;

/// Everything the role picker and the role editor need.
#[derive(Debug, Serialize)]
pub struct TeamRolesResponse {
    pub presets: Vec<RolePreset>,
    pub custom: Vec<CustomRoleResponse>,
    /// Every ability, grouped by `area` in display order.
    pub abilities: Vec<AbilityInfo>,
}

async fn to_response(
    db: &sqlx::SqlitePool,
    row: TeamRoleRow,
) -> Result<CustomRoleResponse, ApiError> {
    let key = format!("{}{}", rbac::CUSTOM_ROLE_PREFIX, row.id);
    let member_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM team_members WHERE team_id = ? AND role = ?")
            .bind(&row.team_id)
            .bind(&key)
            .fetch_one(db)
            .await?;
    Ok(CustomRoleResponse {
        id: row.id,
        key,
        team_id: row.team_id,
        name: row.name,
        description: row.description,
        abilities: rbac::parse_abilities_json(&row.abilities)
            .iter()
            .map(|a| a.as_str().to_string())
            .collect(),
        member_count,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

fn validate_team(id: &str) -> Result<(), ApiError> {
    validate_uuid(id, "team_id").map_err(|e| ApiError::validation_field("team_id", e))
}

/// Validate a role payload and that the caller holds every ability it grants.
async fn validate_request(
    db: &sqlx::SqlitePool,
    team_id: &str,
    actor_role: &str,
    req: &CustomRoleRequest,
) -> Result<(String, AbilitySet), ApiError> {
    let name = req.name.trim().to_string();
    if name.is_empty() || name.len() > 64 {
        return Err(ApiError::validation_field(
            "name",
            "Role name must be 1 to 64 characters",
        ));
    }
    if rbac::is_preset(&name.to_ascii_lowercase()) {
        return Err(ApiError::validation_field(
            "name",
            "This name is used by a built-in role",
        ));
    }
    let mut set = AbilitySet::new();
    for a in &req.abilities {
        let ability = Ability::parse(a).ok_or_else(|| {
            ApiError::validation_field("abilities", format!("Unknown ability `{a}`"))
        })?;
        set.insert(ability);
    }
    if !rbac::role_is_owner(actor_role) {
        let mine = role_abilities(db, team_id, actor_role).await?;
        if let Some(extra) = set.iter().find(|a| !mine.contains(a)) {
            return Err(ApiError::forbidden(format!(
                "You cannot grant `{}`, which your own role does not have",
                extra.as_str()
            )));
        }
    }
    Ok((name, set))
}

/// GET /api/teams/:id/roles
pub async fn list_roles(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
    user: User,
) -> Result<Json<TeamRolesResponse>, ApiError> {
    validate_team(&team_id)?;
    require_team_ability(&state.db, &team_id, &user.id, None).await?;

    let rows: Vec<TeamRoleRow> =
        sqlx::query_as("SELECT * FROM team_roles WHERE team_id = ? ORDER BY name")
            .bind(&team_id)
            .fetch_all(&state.db)
            .await?;
    let mut custom = Vec::with_capacity(rows.len());
    for row in rows {
        custom.push(to_response(&state.db, row).await?);
    }
    Ok(Json(TeamRolesResponse {
        presets: rbac::presets(),
        custom,
        abilities: Ability::ALL.iter().map(|a| a.info()).collect(),
    }))
}

/// POST /api/teams/:id/roles
pub async fn create_role(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
    user: User,
    Json(req): Json<CustomRoleRequest>,
) -> Result<(StatusCode, Json<CustomRoleResponse>), ApiError> {
    validate_team(&team_id)?;
    let membership =
        require_team_ability(&state.db, &team_id, &user.id, Some(Ability::TeamManage)).await?;
    let (name, set) = validate_request(&state.db, &team_id, &membership.role, &req).await?;

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO team_roles (id, team_id, name, description, abilities, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&team_id)
    .bind(&name)
    .bind(req.description.as_deref().map(str::trim).filter(|d| !d.is_empty()))
    .bind(abilities_to_json(&set))
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref d) if d.is_unique_violation() => {
            ApiError::conflict("A role with this name already exists in the team")
        }
        e => e.into(),
    })?;

    let row: TeamRoleRow = sqlx::query_as("SELECT * FROM team_roles WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(to_response(&state.db, row).await?),
    ))
}

/// PUT /api/teams/:id/roles/:role_id
pub async fn update_role(
    State(state): State<Arc<AppState>>,
    Path((team_id, role_id)): Path<(String, String)>,
    user: User,
    Json(req): Json<CustomRoleRequest>,
) -> Result<Json<CustomRoleResponse>, ApiError> {
    validate_team(&team_id)?;
    let membership =
        require_team_ability(&state.db, &team_id, &user.id, Some(Ability::TeamManage)).await?;
    // Editing your own custom role could only ever narrow it (abilities must be a
    // subset of the editor's), but block it anyway so nobody locks themselves out.
    if membership.role == format!("{}{role_id}", rbac::CUSTOM_ROLE_PREFIX) {
        return Err(ApiError::forbidden("You cannot edit the role you hold"));
    }
    let (name, set) = validate_request(&state.db, &team_id, &membership.role, &req).await?;

    let result = sqlx::query(
        "UPDATE team_roles SET name = ?, description = ?, abilities = ?, updated_at = ? \
         WHERE id = ? AND team_id = ?",
    )
    .bind(&name)
    .bind(
        req.description
            .as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty()),
    )
    .bind(abilities_to_json(&set))
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(&role_id)
    .bind(&team_id)
    .execute(&state.db)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref d) if d.is_unique_violation() => {
            ApiError::conflict("A role with this name already exists in the team")
        }
        e => e.into(),
    })?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Role not found"));
    }

    let row: TeamRoleRow = sqlx::query_as("SELECT * FROM team_roles WHERE id = ?")
        .bind(&role_id)
        .fetch_one(&state.db)
        .await?;
    Ok(Json(to_response(&state.db, row).await?))
}

/// DELETE /api/teams/:id/roles/:role_id. Refused while members or pending
/// invitations hold the role.
pub async fn delete_role(
    State(state): State<Arc<AppState>>,
    Path((team_id, role_id)): Path<(String, String)>,
    user: User,
) -> Result<StatusCode, ApiError> {
    validate_team(&team_id)?;
    require_team_ability(&state.db, &team_id, &user.id, Some(Ability::TeamManage)).await?;

    let key = format!("{}{role_id}", rbac::CUSTOM_ROLE_PREFIX);
    let in_use: i64 = sqlx::query_scalar(
        "SELECT (SELECT COUNT(*) FROM team_members WHERE team_id = ? AND role = ?) + \
         (SELECT COUNT(*) FROM team_invitations WHERE team_id = ? AND role = ? AND accepted_at IS NULL)",
    )
    .bind(&team_id)
    .bind(&key)
    .bind(&team_id)
    .bind(&key)
    .fetch_one(&state.db)
    .await?;
    if in_use > 0 {
        return Err(ApiError::conflict(
            "Members or pending invitations still use this role; give them another role first",
        ));
    }

    let result = sqlx::query("DELETE FROM team_roles WHERE id = ? AND team_id = ?")
        .bind(&role_id)
        .bind(&team_id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Role not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}
