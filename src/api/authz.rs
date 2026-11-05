//! Resource-level authorization (SEC-C3).
//!
//! The auth middleware only proves a token belongs to *some* user. These helpers
//! add the missing ownership check: the authenticated user must belong to the
//! team that owns a resource before a handler may read or mutate it.
//!
//! Semantics:
//! - **Instance admins** (`role == "admin"`) and the synthetic `system` user
//!   (admin API token) bypass all checks — they are the operator of the box.
//! - **Legacy / global resources** (`team_id IS NULL`) remain accessible to any
//!   authenticated user. This preserves pre-teams and single-user installs where
//!   resources were never assigned a team.
//! - **Team-scoped resources** require the user to be a member of the owning team.
//!   Apps additionally honor `app_shares` (an app shared with a team the user
//!   belongs to is accessible).

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{MatchedPath, OriginalUri, State},
    http::{Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::db::rbac::{self, Ability, AbilityArea, AbilitySet, ResourceOverride};
use crate::db::{App, ManagedDatabase, Project, Server, Service, TeamMember, User};
use crate::AppState;
use sqlx::SqlitePool;

use super::error::ApiError;
use super::scopes::ActionKind;

/// Instance admins and the admin-token "system" user bypass team scoping.
fn is_privileged(user: &User) -> bool {
    user.id == "system" || user.role == "admin"
}

/// Public predicate: does this user bypass team scoping (instance admin / system)?
pub fn is_privileged_user(user: &User) -> bool {
    is_privileged(user)
}

/// Public wrapper: is `user_id` a member of `team_id`?
pub async fn user_is_member(
    state: &Arc<AppState>,
    user_id: &str,
    team_id: &str,
) -> Result<bool, ApiError> {
    is_team_member(state, user_id, team_id).await
}

/// All team ids the user belongs to (empty for the system user — privileged path
/// short-circuits before this is consulted).
pub async fn user_team_ids(state: &Arc<AppState>, user_id: &str) -> Result<Vec<String>, ApiError> {
    sqlx::query_scalar("SELECT team_id FROM team_members WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))
}

/// Is `user_id` a member of `team_id`?
async fn is_team_member(
    state: &Arc<AppState>,
    user_id: &str,
    team_id: &str,
) -> Result<bool, ApiError> {
    is_team_member_db(&state.db, user_id, team_id).await
}

async fn is_team_member_db(
    db: &SqlitePool,
    user_id: &str,
    team_id: &str,
) -> Result<bool, ApiError> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM team_members WHERE team_id = ? AND user_id = ?")
            .bind(team_id)
            .bind(user_id)
            .fetch_one(db)
            .await
            .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))?;
    Ok(count > 0)
}

/// Core team-access decision used by every resource type.
async fn check_team_access(
    state: &Arc<AppState>,
    user: &User,
    team_id: Option<&str>,
) -> Result<(), ApiError> {
    check_team_access_db(&state.db, user, team_id).await
}

async fn check_team_access_db(
    db: &SqlitePool,
    user: &User,
    team_id: Option<&str>,
) -> Result<(), ApiError> {
    if is_privileged(user) {
        return Ok(());
    }
    match team_id {
        // Legacy / unassigned resource — accessible to any authenticated user.
        None => Ok(()),
        Some(tid) => {
            if is_team_member_db(db, &user.id, tid).await? {
                Ok(())
            } else {
                Err(ApiError::forbidden(
                    "You do not have access to this resource",
                ))
            }
        }
    }
}

/// Reject a client-supplied `team_id` on create/assign unless the caller belongs to
/// that team (privileged users may assign any team). `None`/empty is allowed.
pub async fn authorize_team_assignment(
    state: &Arc<AppState>,
    user: &User,
    team_id: Option<&str>,
) -> Result<(), ApiError> {
    match team_id.filter(|t| !t.is_empty()) {
        None => Ok(()),
        Some(tid) => check_team_access_db(&state.db, user, Some(tid)).await,
    }
}

/// For list endpoints: a non-privileged caller may only filter by a team they belong to.
pub async fn authorize_team_filter(
    state: &Arc<AppState>,
    user: &User,
    team_id: Option<&str>,
) -> Result<(), ApiError> {
    authorize_team_assignment(state, user, team_id).await
}

/// For list endpoints: keep only rows the caller may see (own teams plus legacy rows
/// with no team). Privileged users keep everything.
pub async fn retain_visible<T>(
    state: &Arc<AppState>,
    user: &User,
    items: Vec<T>,
    team_of: impl Fn(&T) -> Option<&str>,
) -> Result<Vec<T>, ApiError> {
    retain_visible_db(&state.db, user, items, team_of).await
}

async fn retain_visible_db<T>(
    db: &SqlitePool,
    user: &User,
    items: Vec<T>,
    team_of: impl Fn(&T) -> Option<&str>,
) -> Result<Vec<T>, ApiError> {
    if is_privileged(user) {
        return Ok(items);
    }
    let teams: Vec<String> =
        sqlx::query_scalar("SELECT team_id FROM team_members WHERE user_id = ?")
            .bind(&user.id)
            .fetch_all(db)
            .await
            .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))?;
    Ok(items
        .into_iter()
        .filter(|i| team_of(i).is_none_or(|t| teams.iter().any(|m| m == t)))
        .collect())
}

/// Subset of `app_ids` the caller may access (privileged: all). Used to filter lists of
/// app children (previews, ...) that carry only an `app_id`.
pub async fn accessible_app_ids(
    state: &Arc<AppState>,
    user: &User,
    app_ids: Vec<String>,
) -> Result<std::collections::HashSet<String>, ApiError> {
    let mut out = std::collections::HashSet::new();
    for id in app_ids {
        if out.contains(&id) {
            continue;
        }
        if is_privileged(user) || authorize_app(state, user, &id).await.is_ok() {
            out.insert(id);
        }
    }
    Ok(out)
}

/// Tables with an `id` and a nullable `team_id` that `authorize_team_row` may read.
/// A closed set so the table name interpolated into SQL is never caller-controlled.
#[derive(Clone, Copy)]
enum TeamTable {
    BuildServers,
    NotificationChannels,
    Destinations,
    CaCertificates,
    S3StorageConfigs,
    CloudflareTunnels,
}

impl TeamTable {
    fn select_sql(self) -> &'static str {
        match self {
            Self::BuildServers => "SELECT team_id FROM build_servers WHERE id = ?",
            Self::NotificationChannels => "SELECT team_id FROM notification_channels WHERE id = ?",
            Self::Destinations => "SELECT team_id FROM destinations WHERE id = ?",
            Self::CaCertificates => "SELECT team_id FROM ca_certificates WHERE id = ?",
            Self::S3StorageConfigs => "SELECT team_id FROM s3_storage_configs WHERE id = ?",
            Self::CloudflareTunnels => "SELECT team_id FROM cloudflare_tunnels WHERE id = ?",
        }
    }

    fn not_found(self) -> &'static str {
        match self {
            Self::BuildServers => "Build server not found",
            Self::NotificationChannels => "Notification channel not found",
            Self::Destinations => "Destination not found",
            Self::CaCertificates => "CA certificate not found",
            Self::S3StorageConfigs => "S3 configuration not found",
            Self::CloudflareTunnels => "Tunnel not found",
        }
    }
}

async fn authorize_team_row(
    db: &SqlitePool,
    user: &User,
    table: TeamTable,
    id: &str,
) -> Result<(), ApiError> {
    let row: Option<Option<String>> = sqlx::query_scalar(table.select_sql())
        .bind(id)
        .fetch_optional(db)
        .await
        .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))?;
    let team_id = row.ok_or_else(|| ApiError::not_found(table.not_found()))?;
    check_team_access_db(db, user, team_id.as_deref()).await
}

/// Authorize access to a build server (team scoped like servers; `team_id IS NULL`
/// stays global, instance admins bypass).
pub async fn authorize_build_server(
    state: &Arc<AppState>,
    user: &User,
    id: &str,
) -> Result<(), ApiError> {
    authorize_team_row(&state.db, user, TeamTable::BuildServers, id).await
}

/// Reject non-privileged callers. For cluster-level resources with no natural team
/// owner (Docker Swarm nodes, swarm services not bound to an app).
pub fn require_privileged(user: &User) -> Result<(), ApiError> {
    if is_privileged(user) {
        Ok(())
    } else {
        Err(ApiError::forbidden(
            "This action requires an instance administrator",
        ))
    }
}

/// Authorize access to a Cloudflare tunnel (team scoped, `team_id IS NULL` stays global).
pub async fn authorize_tunnel(
    state: &Arc<AppState>,
    user: &User,
    id: &str,
) -> Result<(), ApiError> {
    authorize_team_row(&state.db, user, TeamTable::CloudflareTunnels, id).await
}

/// Which team gates a GitHub App for this access mode. System-wide apps are readable
/// by everyone (they exist to be shared); changing one still needs its owning team.
fn github_app_gate(is_system_wide: bool, team_id: Option<&str>, write: bool) -> Option<&str> {
    if is_system_wide && !write {
        None
    } else {
        team_id
    }
}

/// Team that gates *seeing* a GitHub App in a list (system-wide apps are visible to all).
pub fn github_app_list_team(is_system_wide: bool, team_id: Option<&str>) -> Option<&str> {
    github_app_gate(is_system_wide, team_id, false)
}

/// Global SSH keys (`app_id IS NULL`) have no team owner. Changing one needs an instance
/// admin or the user who created it; keys that predate `created_by` are admin-only.
/// App-bound keys are covered by the resource middleware through their app.
pub async fn authorize_global_ssh_key_write(
    db: &SqlitePool,
    user: &User,
    id: &str,
) -> Result<(), ApiError> {
    let row: Option<(Option<String>, Option<String>)> =
        sqlx::query_as("SELECT app_id, created_by FROM ssh_keys WHERE id = ?")
            .bind(id)
            .fetch_optional(db)
            .await
            .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))?;
    let (app_id, created_by) = row.ok_or_else(|| ApiError::not_found("SSH key not found"))?;
    if app_id.is_some() || is_privileged(user) || created_by.as_deref() == Some(user.id.as_str()) {
        return Ok(());
    }
    Err(ApiError::forbidden(
        "Global SSH keys can only be changed by their creator or an instance administrator",
    ))
}

/// Authorize a GitHub App by id. `write` is true for mutating requests.
pub async fn authorize_github_app(
    db: &SqlitePool,
    user: &User,
    id: &str,
    write: bool,
) -> Result<(), ApiError> {
    let row: Option<(Option<String>, bool)> =
        sqlx::query_as("SELECT team_id, is_system_wide FROM github_apps WHERE id = ?")
            .bind(id)
            .fetch_optional(db)
            .await
            .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))?;
    let (team_id, system_wide) = row.ok_or_else(|| ApiError::not_found("GitHub App not found"))?;
    check_team_access_db(
        db,
        user,
        github_app_gate(system_wide, team_id.as_deref(), write),
    )
    .await
}

/// Authorize a GitHub App installation through the app that owns it.
pub async fn authorize_github_installation(
    db: &SqlitePool,
    user: &User,
    id: &str,
    write: bool,
) -> Result<(), ApiError> {
    let row: Option<(Option<String>, bool)> = sqlx::query_as(
        "SELECT a.team_id, a.is_system_wide FROM github_app_installations i \
         JOIN github_apps a ON a.id = i.github_app_id WHERE i.id = ?",
    )
    .bind(id)
    .fetch_optional(db)
    .await
    .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))?;
    let (team_id, system_wide) =
        row.ok_or_else(|| ApiError::not_found("Installation not found"))?;
    check_team_access_db(
        db,
        user,
        github_app_gate(system_wide, team_id.as_deref(), write),
    )
    .await
}

/// Authorize an app and require the developer team role for changing it, for handlers
/// outside the `/api/apps/:id` path that act on an app (swarm services, ...).
pub async fn authorize_app_write(
    state: &Arc<AppState>,
    user: &User,
    app_id: &str,
) -> Result<App, ApiError> {
    let app = authorize_app(state, user, app_id).await?;
    authorize_write(&state.db, user, ResourceKind::App, app_id).await?;
    Ok(app)
}

/// Authorize a service and require the developer team role for changing it.
pub async fn authorize_service_write(
    state: &Arc<AppState>,
    user: &User,
    service_id: &str,
) -> Result<Service, ApiError> {
    let service = authorize_service(state, user, service_id).await?;
    authorize_write(&state.db, user, ResourceKind::Service, service_id).await?;
    Ok(service)
}

/// Can the user change resources in `team_id` (the `apps.write` ability)?
/// Privileged users always can; `None` (global / legacy) is writable by any
/// authenticated user.
pub async fn can_write_team(
    state: &Arc<AppState>,
    user: &User,
    team_id: Option<&str>,
) -> Result<bool, ApiError> {
    Ok(require(&state.db, user, team_id, Ability::AppsWrite)
        .await
        .is_ok())
}

/// Authorize an app and require the `apps.deploy` ability on it, for handlers
/// outside the `/api/apps/:id` path that deploy an app (tag deploys, ...).
pub async fn authorize_app_deploy(
    state: &Arc<AppState>,
    user: &User,
    app_id: &str,
) -> Result<App, ApiError> {
    let app = authorize_app(state, user, app_id).await?;
    authorize_action(
        &state.db,
        user,
        ResourceKind::App,
        app_id,
        ActionKind::Deploy,
    )
    .await?;
    Ok(app)
}

/// Authorize a service and require the `services.deploy` ability on it.
pub async fn authorize_service_deploy(
    state: &Arc<AppState>,
    user: &User,
    service_id: &str,
) -> Result<Service, ApiError> {
    let service = authorize_service(state, user, service_id).await?;
    authorize_action(
        &state.db,
        user,
        ResourceKind::Service,
        service_id,
        ActionKind::Deploy,
    )
    .await?;
    Ok(service)
}

// ── Team RBAC (abilities) ────────────────────────────────────────────────────

/// The ability needed to perform `action` on a resource in `area`. `None` means
/// membership (or share) is enough.
pub fn ability_for(area: AbilityArea, action: ActionKind) -> Option<Ability> {
    use AbilityArea as A;
    use ActionKind as K;
    Some(match (area, action) {
        (A::Apps, K::Read) => Ability::AppsRead,
        (A::Apps, K::Deploy) => Ability::AppsDeploy,
        (A::Apps, K::Write | K::Admin) => Ability::AppsWrite,
        (A::Apps, K::Delete) => Ability::AppsDelete,
        (A::Services, K::Read) => Ability::ServicesRead,
        (A::Services, K::Deploy) => Ability::ServicesDeploy,
        (A::Services, K::Write | K::Admin) => Ability::ServicesWrite,
        (A::Services, K::Delete) => Ability::ServicesDelete,
        (A::Databases, K::Read) => Ability::DatabasesRead,
        (A::Databases, K::Deploy) => Ability::DatabasesDeploy,
        (A::Databases, K::Write | K::Admin) => Ability::DatabasesWrite,
        (A::Databases, K::Delete) => Ability::DatabasesDelete,
        (A::Projects, K::Read) => Ability::ProjectsRead,
        (A::Projects, K::Deploy | K::Write | K::Admin) => Ability::ProjectsWrite,
        (A::Projects, K::Delete) => Ability::ProjectsDelete,
        (A::Apps | A::Services | A::Databases | A::Projects, K::Secrets) => Ability::EnvReadSecrets,
        (A::Servers, K::Read) => Ability::ServersRead,
        (A::Servers, _) => Ability::ServersManage,
        (A::Integrations, K::Read) => return None,
        (A::Integrations, _) => Ability::IntegrationsManage,
    })
}

/// The ability area a resource kind belongs to.
fn area_of(kind: ResourceKind) -> AbilityArea {
    match kind {
        ResourceKind::App
        | ResourceKind::Deployment
        | ResourceKind::Volume
        | ResourceKind::Preview => AbilityArea::Apps,
        ResourceKind::Database => AbilityArea::Databases,
        ResourceKind::Service => AbilityArea::Services,
        ResourceKind::Project | ResourceKind::Environment => AbilityArea::Projects,
        ResourceKind::Server => AbilityArea::Servers,
        ResourceKind::SshKey
        | ResourceKind::NotificationSubscription
        | ResourceKind::S3Backup
        | ResourceKind::BuildServer
        | ResourceKind::NotificationChannel
        | ResourceKind::Destination
        | ResourceKind::CaCertificate
        | ResourceKind::S3Config
        | ResourceKind::Tunnel
        | ResourceKind::GitHubApp
        | ResourceKind::GitHubInstallation => AbilityArea::Integrations,
    }
}

/// Abilities a stored role value grants in a team: a preset, or a custom role
/// of that team. A custom role that no longer exists grants nothing; an unknown
/// value falls back to the viewer preset (what the old role parser did).
pub async fn role_abilities(
    db: &SqlitePool,
    team_id: &str,
    role: &str,
) -> Result<AbilitySet, ApiError> {
    if let Some(id) = rbac::custom_role_id(role) {
        let json: Option<String> =
            sqlx::query_scalar("SELECT abilities FROM team_roles WHERE id = ? AND team_id = ?")
                .bind(id)
                .bind(team_id)
                .fetch_optional(db)
                .await
                .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))?;
        return Ok(json
            .map(|j| rbac::parse_abilities_json(&j))
            .unwrap_or_default());
    }
    Ok(rbac::preset_abilities(rbac::legacy_role_to_preset(role)).unwrap_or_default())
}

/// The caller's role value in a team, if a member.
pub async fn member_role(
    db: &SqlitePool,
    team_id: &str,
    user_id: &str,
) -> Result<Option<String>, ApiError> {
    sqlx::query_scalar("SELECT role FROM team_members WHERE team_id = ? AND user_id = ?")
        .bind(team_id)
        .bind(user_id)
        .fetch_optional(db)
        .await
        .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))
}

/// THE central team ability check. Instance admins and the admin token bypass;
/// team-less (legacy/global) resources are open to any authenticated user;
/// otherwise the caller must be a member whose role grants `ability`.
pub async fn require(
    db: &SqlitePool,
    user: &User,
    team_id: Option<&str>,
    ability: Ability,
) -> Result<(), ApiError> {
    if is_privileged(user) {
        return Ok(());
    }
    let Some(team_id) = team_id else {
        return Ok(());
    };
    let role = member_role(db, team_id, &user.id)
        .await?
        .ok_or_else(|| ApiError::forbidden("You are not a member of this team"))?;
    enforce_two_factor(db, team_id, &user.id).await?;
    if role_abilities(db, team_id, &role).await?.contains(&ability) {
        Ok(())
    } else {
        Err(missing_ability(ability))
    }
}

/// Does `team_id` require 2FA while `user_id` has neither TOTP nor a passkey?
/// Such a member is blocked from the team's resources (and team management)
/// until they set one up, whether they use a session or an API token.
pub async fn two_factor_blocked(
    db: &SqlitePool,
    team_id: &str,
    user_id: &str,
) -> Result<bool, ApiError> {
    let required: Option<i64> = sqlx::query_scalar("SELECT require_2fa FROM teams WHERE id = ?")
        .bind(team_id)
        .fetch_optional(db)
        .await
        .map_err(|e| ApiError::internal(format!("2FA policy lookup failed: {e}")))?;
    if required != Some(1) {
        return Ok(false);
    }
    let totp: Option<bool> = sqlx::query_scalar("SELECT totp_enabled FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(db)
        .await
        .map_err(|e| ApiError::internal(format!("2FA status lookup failed: {e}")))?;
    if totp == Some(true) {
        return Ok(false);
    }
    let has_passkey = crate::api::passkeys::user_has_passkey(db, user_id)
        .await
        .unwrap_or(false);
    Ok(!has_passkey)
}

fn two_factor_error() -> ApiError {
    ApiError::new(
        super::error::ErrorCode::TwoFactorRequired,
        "This team requires two-factor authentication. Set up an authenticator app or a passkey under Account > Security.",
    )
}

/// Reject a member blocked by the team's 2FA requirement.
async fn enforce_two_factor(db: &SqlitePool, team_id: &str, user_id: &str) -> Result<(), ApiError> {
    if two_factor_blocked(db, team_id, user_id).await? {
        Err(two_factor_error())
    } else {
        Ok(())
    }
}

fn missing_ability(ability: Ability) -> ApiError {
    ApiError::forbidden(format!(
        "Your team role does not allow this action (needs `{}`)",
        ability.as_str()
    ))
}

/// Team-route check that also returns the caller's membership. `None` ability
/// means membership is enough. The admin token user ("system") acts as an owner
/// of every team; instance admins need a membership like everyone else (team
/// routes have always behaved this way).
pub async fn require_team_ability(
    db: &SqlitePool,
    team_id: &str,
    user_id: &str,
    ability: Option<Ability>,
) -> Result<TeamMember, ApiError> {
    if user_id == "system" {
        return Ok(TeamMember {
            id: "system".to_string(),
            team_id: team_id.to_string(),
            user_id: "system".to_string(),
            role: "owner".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        });
    }
    let membership: TeamMember =
        sqlx::query_as("SELECT * FROM team_members WHERE team_id = ? AND user_id = ?")
            .bind(team_id)
            .bind(user_id)
            .fetch_optional(db)
            .await
            .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))?
            .ok_or_else(|| ApiError::forbidden("You are not a member of this team"))?;
    if let Some(ability) = ability {
        // Reading the team itself stays possible so the dashboard can explain
        // the block; managing it needs 2FA like its resources do.
        enforce_two_factor(db, team_id, user_id).await?;
        if !role_abilities(db, team_id, &membership.role)
            .await?
            .contains(&ability)
        {
            return Err(missing_ability(ability));
        }
    }
    Ok(membership)
}

/// Validate a role value for assignment in a team: a preset key, or
/// `custom:<id>` naming a custom role of that team. Returns the normalized value.
pub async fn validate_role_value(
    db: &SqlitePool,
    team_id: &str,
    role: &str,
) -> Result<String, ApiError> {
    let lower = role.trim().to_ascii_lowercase();
    if rbac::is_preset(&lower) {
        return Ok(lower);
    }
    if let Some(id) = rbac::custom_role_id(role.trim()) {
        let exists: Option<String> =
            sqlx::query_scalar("SELECT id FROM team_roles WHERE id = ? AND team_id = ?")
                .bind(id)
                .bind(team_id)
                .fetch_optional(db)
                .await
                .map_err(|e| ApiError::internal(format!("role lookup failed: {e}")))?;
        if exists.is_some() {
            return Ok(format!("{}{id}", rbac::CUSTOM_ROLE_PREFIX));
        }
        return Err(ApiError::validation_field(
            "role",
            "Custom role not found in this team",
        ));
    }
    Err(ApiError::validation_field(
        "role",
        "Invalid role. Use owner, admin, developer, deployer, viewer or custom:<role id>",
    ))
}

/// May a member holding `actor_role` manage a member holding (or hand out)
/// `target_role`? Owners may do anything. Nobody else may touch the owner role,
/// or a role with abilities the actor does not have (no privilege escalation
/// through custom roles).
pub async fn can_manage_role(
    db: &SqlitePool,
    team_id: &str,
    actor_role: &str,
    target_role: &str,
) -> Result<bool, ApiError> {
    if rbac::role_is_owner(actor_role) {
        return Ok(true);
    }
    if rbac::role_is_owner(target_role) {
        return Ok(false);
    }
    let actor = role_abilities(db, team_id, actor_role).await?;
    let target = role_abilities(db, team_id, target_role).await?;
    Ok(target.is_subset(&actor))
}

/// The per-resource override that applies to a member: overrides on the
/// resource itself, its app (for deployments, volumes, previews) and its
/// project are combined, and a deny anywhere wins over an allow.
async fn resource_override(
    db: &SqlitePool,
    team_id: &str,
    user_id: &str,
    kind: ResourceKind,
    id: &str,
    owner: &ResourceOwner,
) -> Result<Option<ResourceOverride>, ApiError> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT resource_type, resource_id, permission FROM team_resource_permissions \
         WHERE team_id = ? AND user_id = ?",
    )
    .bind(team_id)
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))?;
    if rows.is_empty() {
        return Ok(None);
    }
    Ok(pick_override(&rows, kind, id, owner))
}

/// Pure part of [`resource_override`].
fn pick_override(
    rows: &[(String, String, String)],
    kind: ResourceKind,
    id: &str,
    owner: &ResourceOwner,
) -> Option<ResourceOverride> {
    let own_type = match kind {
        ResourceKind::App => Some("app"),
        ResourceKind::Database => Some("database"),
        ResourceKind::Service => Some("service"),
        ResourceKind::Project => Some("project"),
        _ => None,
    };
    let find = |t: &str, rid: &str| {
        rows.iter()
            .find(|(rt, ri, _)| rt == t && ri == rid)
            .and_then(|(_, _, p)| ResourceOverride::parse(p))
    };
    let found = [
        own_type.and_then(|t| find(t, id)),
        owner.app_id.as_deref().and_then(|a| find("app", a)),
        owner.project_id.as_deref().and_then(|p| find("project", p)),
    ];
    // Deny wins over allow, wherever it is set (resource, app or project).
    if found.contains(&Some(ResourceOverride::Deny)) {
        Some(ResourceOverride::Deny)
    } else if found.contains(&Some(ResourceOverride::Allow)) {
        Some(ResourceOverride::Allow)
    } else {
        None
    }
}

/// Authorize `action` on one team resource: the caller's role abilities in the
/// owning team, with per-resource allow/deny overrides on top. Visibility
/// (membership or an app share) is checked separately by `authorize_*`.
pub(crate) async fn authorize_action(
    db: &SqlitePool,
    user: &User,
    kind: ResourceKind,
    id: &str,
    action: ActionKind,
) -> Result<(), ApiError> {
    if is_privileged(user) {
        return Ok(());
    }
    let Some(needed) = ability_for(area_of(kind), action) else {
        return Ok(());
    };
    let Some(owner) = resource_owner(db, kind, id).await? else {
        return Ok(());
    };
    let Some(team_id) = owner.team_id.clone() else {
        return Ok(());
    };
    let Some(role) = member_role(db, &team_id, &user.id).await? else {
        // Reached through an app share: view only.
        return if action == ActionKind::Read {
            Ok(())
        } else {
            Err(ApiError::forbidden(
                "Shared resources are read-only for the receiving team",
            ))
        };
    };
    enforce_two_factor(db, &team_id, &user.id).await?;
    let abilities = role_abilities(db, &team_id, &role).await?;
    let over = resource_override(db, &team_id, &user.id, kind, id, &owner).await?;
    if rbac::effective_allows(&abilities, over, needed) {
        Ok(())
    } else if over == Some(ResourceOverride::Deny) {
        Err(ApiError::forbidden(
            "Your access to this resource has been denied by a team administrator",
        ))
    } else {
        Err(missing_ability(needed))
    }
}

/// Authorize access to an app and return it. Honors `app_shares` so an app shared
/// with one of the user's teams is reachable.
pub async fn authorize_app(
    state: &Arc<AppState>,
    user: &User,
    app_id: &str,
) -> Result<App, ApiError> {
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(app_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    if is_privileged(user) || app.team_id.is_none() {
        return Ok(app);
    }

    // Member of the owning team?
    if let Some(tid) = &app.team_id {
        if is_team_member(state, &user.id, tid).await? {
            return Ok(app);
        }
    }

    // Or shared with one of the user's teams?
    let shared: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM app_shares s \
         JOIN team_members m ON m.team_id = s.shared_with_team_id \
         WHERE s.app_id = ? AND m.user_id = ?",
    )
    .bind(app_id)
    .bind(&user.id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))?;

    if shared > 0 {
        Ok(app)
    } else {
        Err(ApiError::forbidden("You do not have access to this app"))
    }
}

/// Authorize access to a server and return it.
pub async fn authorize_server(
    state: &Arc<AppState>,
    user: &User,
    server_id: &str,
) -> Result<Server, ApiError> {
    let server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(server_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))?
        .ok_or_else(|| ApiError::not_found("Server not found"))?;

    check_team_access(state, user, server.team_id.as_deref()).await?;
    Ok(server)
}

/// Authorize access to a database and return it.
pub async fn authorize_database(
    state: &Arc<AppState>,
    user: &User,
    database_id: &str,
) -> Result<ManagedDatabase, ApiError> {
    let database = sqlx::query_as::<_, ManagedDatabase>("SELECT * FROM databases WHERE id = ?")
        .bind(database_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))?
        .ok_or_else(|| ApiError::not_found("Database not found"))?;

    check_team_access(state, user, database.team_id.as_deref()).await?;
    Ok(database)
}

/// Authorize access to a service and return it.
pub async fn authorize_service(
    state: &Arc<AppState>,
    user: &User,
    service_id: &str,
) -> Result<Service, ApiError> {
    let service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(service_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))?
        .ok_or_else(|| ApiError::not_found("Service not found"))?;

    check_team_access(state, user, service.team_id.as_deref()).await?;
    Ok(service)
}

/// Authorize access to a project and return it.
pub async fn authorize_project(
    state: &Arc<AppState>,
    user: &User,
    project_id: &str,
) -> Result<Project, ApiError> {
    let project = sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = ?")
        .bind(project_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    check_team_access(state, user, project.team_id.as_deref()).await?;
    Ok(project)
}

/// Authorize access to a deployment by resolving its owning app.
pub async fn authorize_deployment(
    state: &Arc<AppState>,
    user: &User,
    deployment_id: &str,
) -> Result<(), ApiError> {
    let app_id: Option<String> = sqlx::query_scalar("SELECT app_id FROM deployments WHERE id = ?")
        .bind(deployment_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))?;
    let app_id = app_id.ok_or_else(|| ApiError::not_found("Deployment not found"))?;
    authorize_app(state, user, &app_id).await?;
    Ok(())
}

/// Path-aware authorization middleware (SEC-C3).
///
/// Runs after `auth_middleware` (token already validated) on the protected API
/// group. It parses the request path and, when it targets a specific
/// app/server/database/service/deployment by id, verifies the authenticated user
/// has access before the handler runs. This is a single chokepoint that covers
/// every `/api/{resource}/:id/...` route — current and future — instead of
/// per-handler checks.
///
/// Non-resource paths, collection paths (`/api/apps`), and literal segments that
/// are not UUIDs (`/api/apps/with-sharing`, `/api/services/check-port`) are passed
/// through untouched; the handler / list logic owns those.
#[allow(clippy::result_large_err)] // Err carries the ready-to-send rejection response
pub async fn resource_authz_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    // Identify (resource_kind, id) from the path, if any. The router is mounted with
    // `.nest("/api", ..)`, which strips the prefix from `request.uri()`, so match on the
    // original URI or no resource path would ever be recognised.
    let path = request
        .extensions()
        .get::<OriginalUri>()
        .map(|o| o.0.path().to_string())
        .unwrap_or_else(|| request.uri().path().to_string());
    let target = parse_resource_target(&path);
    let mutating = !matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    // What the request does, from the same route table that maps API token
    // scopes (api/scopes.rs). Unmapped routes fall back to read/write by method.
    let action = request
        .extensions()
        .get::<MatchedPath>()
        .and_then(|m| {
            super::scopes::classify_request(request.method(), m.as_str(), request.uri().query())
        })
        .unwrap_or(if mutating {
            ActionKind::Write
        } else {
            ActionKind::Read
        });

    if let Some((kind, id)) = target {
        // Resolve the caller. Auth middleware already validated the token, so this
        // should succeed; treat failure as unauthorized.
        let token = extract_request_token(&request)
            .ok_or_else(|| StatusCode::UNAUTHORIZED.into_response())?;
        let user = super::auth::get_current_user(&state.db, &state.config, &token)
            .await
            .map_err(|s| s.into_response())?;

        let result = match kind {
            ResourceKind::App => authorize_app(&state, &user, id).await.map(|_| ()),
            ResourceKind::Server => authorize_server(&state, &user, id).await.map(|_| ()),
            ResourceKind::Database => authorize_database(&state, &user, id).await.map(|_| ()),
            ResourceKind::Service => authorize_service(&state, &user, id).await.map(|_| ()),
            ResourceKind::Project => authorize_project(&state, &user, id).await.map(|_| ()),
            ResourceKind::Deployment => authorize_deployment(&state, &user, id).await,
            ResourceKind::Volume => authorize_via_app(&state, &user, VOLUME_APP_SQL, id).await,
            ResourceKind::Preview => authorize_via_app(&state, &user, PREVIEW_APP_SQL, id).await,
            ResourceKind::SshKey => authorize_ssh_key(&state, &user, id).await,
            ResourceKind::Environment => authorize_environment(&state, &user, id).await,
            ResourceKind::NotificationSubscription => {
                authorize_subscription(&state, &user, id).await
            }
            ResourceKind::S3Backup => authorize_s3_backup(&state, &user, id).await,
            ResourceKind::BuildServer => {
                authorize_team_row(&state.db, &user, TeamTable::BuildServers, id).await
            }
            ResourceKind::NotificationChannel => {
                authorize_team_row(&state.db, &user, TeamTable::NotificationChannels, id).await
            }
            ResourceKind::Destination => {
                authorize_team_row(&state.db, &user, TeamTable::Destinations, id).await
            }
            ResourceKind::CaCertificate => {
                authorize_team_row(&state.db, &user, TeamTable::CaCertificates, id).await
            }
            ResourceKind::S3Config => {
                authorize_team_row(&state.db, &user, TeamTable::S3StorageConfigs, id).await
            }
            ResourceKind::Tunnel => authorize_tunnel(&state, &user, id).await,
            ResourceKind::GitHubApp => authorize_github_app(&state.db, &user, id, mutating).await,
            ResourceKind::GitHubInstallation => {
                authorize_github_installation(&state.db, &user, id, mutating).await
            }
        };

        if let Err(e) = result {
            return Err(e.into_response());
        }
        // Team RBAC: the role's abilities plus per-resource overrides.
        if let Err(e) = authorize_action(&state.db, &user, kind, id, action).await {
            return Err(e.into_response());
        }
    }

    Ok(next.run(request).await)
}

/// SQL returning the owning `team_id` for kinds whose writes are role-gated.
fn team_sql(kind: ResourceKind) -> Option<&'static str> {
    Some(match kind {
        ResourceKind::App => "SELECT team_id FROM apps WHERE id = ?",
        ResourceKind::Deployment => {
            "SELECT a.team_id FROM deployments d JOIN apps a ON a.id = d.app_id WHERE d.id = ?"
        }
        ResourceKind::Volume => {
            "SELECT a.team_id FROM volumes v JOIN apps a ON a.id = v.app_id WHERE v.id = ?"
        }
        ResourceKind::Preview => {
            "SELECT a.team_id FROM preview_deployments p JOIN apps a ON a.id = p.app_id WHERE p.id = ?"
        }
        ResourceKind::Database => "SELECT team_id FROM databases WHERE id = ?",
        ResourceKind::Service => "SELECT team_id FROM services WHERE id = ?",
        ResourceKind::Project => "SELECT team_id FROM projects WHERE id = ?",
        ResourceKind::Server => "SELECT team_id FROM servers WHERE id = ?",
        // App-bound SSH keys follow their app; global keys (app_id NULL) match no row.
        ResourceKind::SshKey => {
            "SELECT a.team_id FROM ssh_keys k JOIN apps a ON a.id = k.app_id WHERE k.id = ?"
        }
        ResourceKind::Tunnel => "SELECT team_id FROM cloudflare_tunnels WHERE id = ?",
        ResourceKind::GitHubApp => "SELECT team_id FROM github_apps WHERE id = ?",
        ResourceKind::GitHubInstallation => {
            "SELECT a.team_id FROM github_app_installations i \
             JOIN github_apps a ON a.id = i.github_app_id WHERE i.id = ?"
        }
        _ => return None,
    })
}

/// Mutating requests need the matching write ability (`apps.write`, ...) in the
/// owning team; a team that only holds an `app_shares` grant cannot write.
/// Instance admins, legacy team-less resources and kinds without a team pass.
async fn authorize_write(
    db: &SqlitePool,
    user: &User,
    kind: ResourceKind,
    id: &str,
) -> Result<(), ApiError> {
    authorize_action(db, user, kind, id, ActionKind::Write).await
}

const VOLUME_APP_SQL: &str = "SELECT app_id FROM volumes WHERE id = ?";
const PREVIEW_APP_SQL: &str = "SELECT app_id FROM preview_deployments WHERE id = ?";

/// Authorize a child resource (volume, preview) through its owning app. A missing
/// row falls through as 404 from the handler's own lookup semantics.
async fn authorize_via_app(
    state: &Arc<AppState>,
    user: &User,
    sql: &'static str,
    id: &str,
) -> Result<(), ApiError> {
    let app_id: Option<String> = sqlx::query_scalar(sql)
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))?;
    let app_id = app_id.ok_or_else(|| ApiError::not_found("Resource not found"))?;
    authorize_app(state, user, &app_id).await.map(|_| ())
}

/// SSH keys: app-bound keys follow the app; global keys stay shared.
async fn authorize_ssh_key(state: &Arc<AppState>, user: &User, id: &str) -> Result<(), ApiError> {
    let row: Option<Option<String>> =
        sqlx::query_scalar("SELECT app_id FROM ssh_keys WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))?;
    match row.ok_or_else(|| ApiError::not_found("SSH key not found"))? {
        Some(app_id) => authorize_app(state, user, &app_id).await.map(|_| ()),
        None => Ok(()),
    }
}

/// Project environments follow their project.
async fn authorize_environment(
    state: &Arc<AppState>,
    user: &User,
    id: &str,
) -> Result<(), ApiError> {
    let project_id: Option<String> =
        sqlx::query_scalar("SELECT project_id FROM environments WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))?;
    let project_id = project_id.ok_or_else(|| ApiError::not_found("Environment not found"))?;
    authorize_project(state, user, &project_id)
        .await
        .map(|_| ())
}

/// Notification subscriptions follow their channel.
async fn authorize_subscription(
    state: &Arc<AppState>,
    user: &User,
    id: &str,
) -> Result<(), ApiError> {
    let row: Option<Option<String>> = sqlx::query_scalar(
        "SELECT c.team_id FROM notification_subscriptions s \
         JOIN notification_channels c ON c.id = s.channel_id WHERE s.id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))?;
    let team_id = row.ok_or_else(|| ApiError::not_found("Subscription not found"))?;
    check_team_access(state, user, team_id.as_deref()).await
}

/// S3 backups follow their storage config.
async fn authorize_s3_backup(state: &Arc<AppState>, user: &User, id: &str) -> Result<(), ApiError> {
    let row: Option<Option<String>> = sqlx::query_scalar(
        "SELECT c.team_id FROM s3_backups b \
         JOIN s3_storage_configs c ON c.id = b.storage_config_id WHERE b.id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("authz lookup failed: {e}")))?;
    let team_id = row.ok_or_else(|| ApiError::not_found("Backup not found"))?;
    check_team_access(state, user, team_id.as_deref()).await
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResourceKind {
    App,
    Server,
    Database,
    Service,
    Project,
    Deployment,
    Volume,
    Preview,
    SshKey,
    Environment,
    NotificationSubscription,
    S3Backup,
    BuildServer,
    NotificationChannel,
    Destination,
    CaCertificate,
    S3Config,
    Tunnel,
    GitHubApp,
    GitHubInstallation,
}

/// Extract `(kind, id)` from a request path when it targets a specific resource.
/// Returns `None` for collection paths, non-resource paths, or non-UUID segments.
pub(crate) fn parse_resource_target(path: &str) -> Option<(ResourceKind, &str)> {
    let rest = path.strip_prefix("/api/")?;
    let mut segs = rest.split('/');
    // Set when the first id-like segment was already consumed to pick the kind.
    let mut id_override = None;
    let kind = match segs.next()? {
        "apps" => ResourceKind::App,
        "servers" => ResourceKind::Server,
        "databases" => ResourceKind::Database,
        "services" => ResourceKind::Service,
        "projects" => ResourceKind::Project,
        "deployments" => ResourceKind::Deployment,
        "volumes" => ResourceKind::Volume,
        "previews" => ResourceKind::Preview,
        "ssh-keys" => ResourceKind::SshKey,
        "environments" => ResourceKind::Environment,
        "notification-subscriptions" => ResourceKind::NotificationSubscription,
        "build-servers" => ResourceKind::BuildServer,
        "notification-channels" => ResourceKind::NotificationChannel,
        "destinations" => ResourceKind::Destination,
        "ca-certificates" => ResourceKind::CaCertificate,
        "tunnels" => ResourceKind::Tunnel,
        // `/github-apps/installations/:iid/...` addresses an installation, every other
        // second segment is the app id.
        "github-apps" => match segs.next()? {
            "installations" => ResourceKind::GitHubInstallation,
            app_id => {
                id_override = Some(app_id);
                ResourceKind::GitHubApp
            }
        },
        "s3" => match segs.next()? {
            "configs" => ResourceKind::S3Config,
            "backups" => ResourceKind::S3Backup,
            _ => return None,
        },
        _ => return None,
    };
    let id = match id_override {
        Some(id) => id,
        None => segs.next()?,
    };
    // Only enforce when the segment is an actual resource id (UUID). Literal
    // sub-routes like `with-sharing` / `check-port` fall through to the handler.
    if uuid::Uuid::parse_str(id).is_ok() {
        Some((kind, id))
    } else {
        None
    }
}

/// The token on a request (`Authorization`, `X-API-Key` or `?token=`), as the
/// auth middleware reads it.
pub(crate) fn request_token(request: &Request<Body>) -> Option<String> {
    extract_request_token(request)
}

/// May `user` restrict an API token to `team_id`? Instance admins may pick any
/// team; everyone else must be a member whose role has `tokens.manage`.
pub async fn authorize_token_team(
    state: &Arc<AppState>,
    user: &User,
    team_id: &str,
) -> Result<(), ApiError> {
    if is_privileged(user) {
        return Ok(());
    }
    if !is_team_member(state, &user.id, team_id).await? {
        return Err(ApiError::forbidden(
            "You can only restrict a token to a team you belong to",
        ));
    }
    require(&state.db, user, Some(team_id), Ability::TokensManage).await
}

/// Owning team, project and app of a resource addressed by a request path.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ResourceOwner {
    pub team_id: Option<String>,
    pub project_id: Option<String>,
    pub app_id: Option<String>,
}

/// SQL returning `(team_id, project_id, app_id)` for a resource kind.
fn owner_sql(kind: ResourceKind) -> Option<&'static str> {
    Some(match kind {
        ResourceKind::App => "SELECT team_id, project_id, id FROM apps WHERE id = ?",
        ResourceKind::Database => "SELECT team_id, project_id, NULL FROM databases WHERE id = ?",
        ResourceKind::Service => "SELECT team_id, project_id, NULL FROM services WHERE id = ?",
        ResourceKind::Project => "SELECT team_id, id, NULL FROM projects WHERE id = ?",
        ResourceKind::Deployment => {
            "SELECT a.team_id, a.project_id, a.id FROM deployments d \
             JOIN apps a ON a.id = d.app_id WHERE d.id = ?"
        }
        ResourceKind::Volume => {
            "SELECT a.team_id, a.project_id, a.id FROM volumes v \
             JOIN apps a ON a.id = v.app_id WHERE v.id = ?"
        }
        ResourceKind::Preview => {
            "SELECT a.team_id, a.project_id, a.id FROM preview_deployments p \
             JOIN apps a ON a.id = p.app_id WHERE p.id = ?"
        }
        ResourceKind::Environment => {
            "SELECT p.team_id, p.id, NULL FROM environments e \
             JOIN projects p ON p.id = e.project_id WHERE e.id = ?"
        }
        _ => return None,
    })
}

/// Resolve who owns a resource. `None` when the row does not exist.
pub(crate) async fn resource_owner(
    db: &SqlitePool,
    kind: ResourceKind,
    id: &str,
) -> Result<Option<ResourceOwner>, ApiError> {
    let err = |e: sqlx::Error| ApiError::internal(format!("authz lookup failed: {e}"));
    if let Some(sql) = owner_sql(kind) {
        let row: Option<(Option<String>, Option<String>, Option<String>)> = sqlx::query_as(sql)
            .bind(id)
            .fetch_optional(db)
            .await
            .map_err(err)?;
        return Ok(row.map(|(team_id, project_id, app_id)| ResourceOwner {
            team_id,
            project_id,
            app_id,
        }));
    }
    let Some(sql) = team_sql(kind) else {
        return Ok(Some(ResourceOwner::default()));
    };
    let row: Option<Option<String>> = sqlx::query_scalar(sql)
        .bind(id)
        .fetch_optional(db)
        .await
        .map_err(err)?;
    Ok(row.map(|team_id| ResourceOwner {
        team_id,
        ..Default::default()
    }))
}

/// Is the addressed resource inside a token's resource list? A listed project
/// covers its apps, databases, services and environments; a listed app covers
/// its deployments, volumes and previews.
pub(crate) fn resource_listed(
    list: &[super::api_tokens::TokenResource],
    kind: ResourceKind,
    id: &str,
    owner: &ResourceOwner,
) -> bool {
    let own_type = match kind {
        ResourceKind::App => Some("app"),
        ResourceKind::Database => Some("database"),
        ResourceKind::Service => Some("service"),
        ResourceKind::Project => Some("project"),
        _ => None,
    };
    list.iter().any(|r| {
        (own_type == Some(r.kind.as_str()) && r.id == id)
            || (r.kind == "app" && owner.app_id.as_deref() == Some(r.id.as_str()))
            || (r.kind == "project" && owner.project_id.as_deref() == Some(r.id.as_str()))
    })
}

/// Enforce an API token's team and resource restrictions on one request.
///
/// - Restricted tokens never reach administration routes.
/// - A route that addresses a resource by id must target a resource owned by
///   the token's team and, with a resource list, one of the listed resources
///   (or a child of one).
/// - Team routes (`/api/teams/:id/...`) must name the token's team.
/// - With a resource list, routes that address no resource are refused.
/// - With only a team restriction, collection reads get `team_id` forced into
///   the query string, and other non-resource writes must carry the token's
///   `team_id` in their JSON body.
///
/// Returns the (possibly rewritten) request or the rejection to send.
#[allow(clippy::result_large_err)] // Err carries the ready-to-send rejection response
pub(crate) async fn check_token_restrictions(
    state: &Arc<AppState>,
    ctx: &super::api_tokens::TokenContext,
    action: ActionKind,
    request: Request<Body>,
) -> Result<Request<Body>, Response> {
    let deny = |m: &str| ApiError::forbidden(m.to_string()).into_response();

    if action == ActionKind::Admin {
        return Err(deny(
            "Team or resource restricted API tokens cannot call administration routes",
        ));
    }
    let path = request
        .extensions()
        .get::<OriginalUri>()
        .map(|o| o.0.path().to_string())
        .unwrap_or_else(|| request.uri().path().to_string());

    if let Some((kind, id)) = parse_resource_target(&path) {
        let owner = resource_owner(&state.db, kind, id)
            .await
            .map_err(IntoResponse::into_response)?;
        // A missing row is left to the handler's own 404.
        let Some(owner) = owner else {
            return Ok(request);
        };
        if let Some(tid) = &ctx.team_id {
            if owner.team_id.as_deref() != Some(tid.as_str()) {
                return Err(deny("This API token is restricted to another team"));
            }
        }
        if !ctx.resources.is_empty() && !resource_listed(&ctx.resources, kind, id, &owner) {
            return Err(deny(
                "This API token is not allowed to access this resource",
            ));
        }
        return Ok(request);
    }

    if !ctx.resources.is_empty() {
        return Err(deny(
            "This API token is restricted to specific resources; call a route that addresses one of them by id",
        ));
    }
    let Some(tid) = ctx.team_id.clone() else {
        return Ok(request);
    };

    // `/api/teams/:id/...` must be the token's own team.
    if let Some(rest) = path.strip_prefix("/api/teams/") {
        let team = rest.split('/').next().unwrap_or("");
        if !team.is_empty() {
            return if team == tid {
                Ok(request)
            } else {
                Err(deny("This API token is restricted to another team"))
            };
        }
    }

    if matches!(action, ActionKind::Read | ActionKind::Secrets) {
        return force_team_query(request, &tid).map_err(|m| deny(&m));
    }
    require_team_in_body(request, &tid).await
}

/// Make a collection read filter by the token's team: reject a different
/// `team_id`, add it when missing.
fn force_team_query(mut request: Request<Body>, team_id: &str) -> Result<Request<Body>, String> {
    let query = request.uri().query().unwrap_or("").to_string();
    let existing = query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == "team_id").then(|| v.to_string())
    });
    match existing {
        Some(v) if v == team_id => Ok(request),
        Some(_) => Err("This API token is restricted to another team".into()),
        None => {
            let new_query = if query.is_empty() {
                format!("team_id={team_id}")
            } else {
                format!("{query}&team_id={team_id}")
            };
            let pq = format!("{}?{}", request.uri().path(), new_query);
            let mut parts = request.uri().clone().into_parts();
            parts.path_and_query = pq.parse().ok();
            if let Ok(uri) = axum::http::Uri::from_parts(parts) {
                *request.uri_mut() = uri;
            }
            Ok(request)
        }
    }
}

/// Non-resource writes by a team restricted token must name its team in a JSON
/// body (`{"team_id": ...}`); anything else could act outside the team.
#[allow(clippy::result_large_err)] // Err carries the ready-to-send rejection response
async fn require_team_in_body(
    request: Request<Body>,
    team_id: &str,
) -> Result<Request<Body>, Response> {
    let deny = |m: &str| ApiError::forbidden(m.to_string()).into_response();
    let is_json = request
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("application/json"));
    if !is_json {
        return Err(deny(
            "A team restricted API token can only create resources with a JSON body that sets team_id",
        ));
    }
    let (parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, 1024 * 1024)
        .await
        .map_err(|_| deny("Request body too large for a team restricted API token"))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
    if value.get("team_id").and_then(|v| v.as_str()) != Some(team_id) {
        return Err(deny(
            "This API token is restricted to one team; set team_id to that team in the request body",
        ));
    }
    Ok(Request::from_parts(parts, Body::from(bytes)))
}

/// Extract the bearer token from a request the same way `auth_middleware` does:
/// `Authorization: Bearer`, `X-API-Key`, or a `?token=` query parameter.
fn extract_request_token(request: &Request<Body>) -> Option<String> {
    if let Some(h) = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
    {
        return Some(h.strip_prefix("Bearer ").unwrap_or(h).to_string());
    }
    if let Some(k) = request
        .headers()
        .get("X-API-Key")
        .and_then(|h| h.to_str().ok())
    {
        return Some(k.to_string());
    }
    request.uri().query().and_then(|q| {
        q.split('&').find_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            (key == "token").then(|| value.to_string())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    const ID: &str = "123e4567-e89b-12d3-a456-426614174000";

    fn user(id: &str, role: &str) -> User {
        User {
            id: id.to_string(),
            email: format!("{id}@example.com"),
            password_hash: String::new(),
            name: id.to_string(),
            role: role.to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            totp_secret: None,
            totp_enabled: false,
            recovery_codes: None,
            oidc_subject: None,
            oidc_provider_id: None,
        }
    }

    async fn pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for sql in [
            "CREATE TABLE team_members (team_id TEXT, user_id TEXT)",
            "CREATE TABLE build_servers (id TEXT, team_id TEXT)",
            "INSERT INTO team_members VALUES ('t1', 'alice')",
            "INSERT INTO build_servers VALUES ('bs-t1', 't1')",
            "INSERT INTO build_servers VALUES ('bs-t2', 't2')",
            "INSERT INTO build_servers VALUES ('bs-global', NULL)",
        ] {
            sqlx::query(sql).execute(&pool).await.unwrap();
        }
        pool
    }

    #[test]
    fn parse_targets_resource_ids_only() {
        let k = |p: &str| parse_resource_target(p).map(|(k, id)| (k, id.to_string()));
        let id = ID.to_string();
        assert_eq!(
            k(&format!("/api/apps/{ID}")),
            Some((ResourceKind::App, id.clone()))
        );
        assert_eq!(
            k(&format!("/api/apps/{ID}/env-vars/KEY")),
            Some((ResourceKind::App, id.clone()))
        );
        assert_eq!(
            k(&format!("/api/build-servers/{ID}/check")),
            Some((ResourceKind::BuildServer, id.clone()))
        );
        assert_eq!(
            k(&format!("/api/volumes/{ID}")),
            Some((ResourceKind::Volume, id.clone()))
        );
        assert_eq!(
            k(&format!("/api/s3/configs/{ID}/test")),
            Some((ResourceKind::S3Config, id.clone()))
        );
        assert_eq!(
            k(&format!("/api/s3/backups/{ID}")),
            Some((ResourceKind::S3Backup, id.clone()))
        );
        assert_eq!(
            k(&format!("/api/notification-channels/{ID}/subscriptions")),
            Some((ResourceKind::NotificationChannel, id))
        );
    }

    #[test]
    fn parse_tunnels_and_github_apps() {
        let k = |p: &str| parse_resource_target(p).map(|(k, id)| (k, id.to_string()));
        let id = ID.to_string();
        let other = "223e4567-e89b-12d3-a456-426614174000";
        assert_eq!(
            k(&format!("/api/tunnels/{ID}/routes/{other}")),
            Some((ResourceKind::Tunnel, id.clone()))
        );
        assert_eq!(
            k(&format!("/api/github-apps/{ID}")),
            Some((ResourceKind::GitHubApp, id.clone()))
        );
        // The app id is the one that is authorized, not the installation id.
        assert_eq!(
            k(&format!(
                "/api/github-apps/{ID}/installations/{other}/repos"
            )),
            Some((ResourceKind::GitHubApp, id.clone()))
        );
        assert_eq!(
            k(&format!("/api/github-apps/installations/{ID}/repos")),
            Some((ResourceKind::GitHubInstallation, id.clone()))
        );
        assert_eq!(
            k(&format!(
                "/api/github-apps/installations/{ID}/repos/o/r/branches"
            )),
            Some((ResourceKind::GitHubInstallation, id))
        );
        // Collections and the installations listing are left to the handlers.
        for p in [
            "/api/tunnels",
            "/api/github-apps",
            "/api/github-apps/installations",
        ] {
            assert!(parse_resource_target(p).is_none(), "{p}");
        }
    }

    #[test]
    fn require_privileged_allows_only_admins() {
        assert!(require_privileged(&user("root", "admin")).is_ok());
        assert!(require_privileged(&user("system", "member")).is_ok());
        assert_eq!(
            require_privileged(&user("alice", "user"))
                .unwrap_err()
                .status(),
            StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn github_app_gate_rules() {
        // System-wide apps are readable by anyone but changeable only by the owning team.
        assert_eq!(github_app_gate(true, Some("t1"), false), None);
        assert_eq!(github_app_gate(true, Some("t1"), true), Some("t1"));
        assert_eq!(github_app_gate(false, Some("t1"), false), Some("t1"));
        assert_eq!(github_app_gate(false, None, true), None);
        assert_eq!(github_app_list_team(true, Some("t1")), None);
        assert_eq!(github_app_list_team(false, Some("t1")), Some("t1"));
    }

    async fn github_pool() -> SqlitePool {
        let db = pool().await;
        for sql in [
            "CREATE TABLE github_apps (id TEXT, team_id TEXT, is_system_wide INTEGER)",
            "CREATE TABLE github_app_installations (id TEXT, github_app_id TEXT)",
            "CREATE TABLE cloudflare_tunnels (id TEXT, team_id TEXT)",
            "INSERT INTO github_apps VALUES ('ga-t1', 't1', 0)",
            "INSERT INTO github_apps VALUES ('ga-t2', 't2', 0)",
            "INSERT INTO github_apps VALUES ('ga-t2-shared', 't2', 1)",
            "INSERT INTO github_apps VALUES ('ga-legacy', NULL, 0)",
            "INSERT INTO github_app_installations VALUES ('in-t1', 'ga-t1')",
            "INSERT INTO github_app_installations VALUES ('in-t2', 'ga-t2')",
            "INSERT INTO github_app_installations VALUES ('in-t2-shared', 'ga-t2-shared')",
            "INSERT INTO cloudflare_tunnels VALUES ('tn-t1', 't1')",
            "INSERT INTO cloudflare_tunnels VALUES ('tn-t2', 't2')",
            "INSERT INTO cloudflare_tunnels VALUES ('tn-legacy', NULL)",
        ] {
            sqlx::query(sql).execute(&db).await.unwrap();
        }
        db
    }

    #[tokio::test]
    async fn github_apps_follow_team_and_sharing() {
        let db = github_pool().await;
        let alice = user("alice", "user");
        let bob = user("bob", "user");
        let admin = user("root", "admin");
        for write in [false, true] {
            assert!(authorize_github_app(&db, &alice, "ga-t1", write)
                .await
                .is_ok());
            assert_eq!(
                authorize_github_app(&db, &alice, "ga-t2", write)
                    .await
                    .unwrap_err()
                    .status(),
                StatusCode::FORBIDDEN
            );
            assert!(authorize_github_app(&db, &bob, "ga-legacy", write)
                .await
                .is_ok());
            assert!(authorize_github_app(&db, &admin, "ga-t2", write)
                .await
                .is_ok());
            assert_eq!(
                authorize_github_app(&db, &alice, "missing", write)
                    .await
                    .unwrap_err()
                    .status(),
                StatusCode::NOT_FOUND
            );
        }
        // System-wide app owned by t2: alice may use (read) it but not change it.
        assert!(authorize_github_app(&db, &alice, "ga-t2-shared", false)
            .await
            .is_ok());
        assert_eq!(
            authorize_github_app(&db, &alice, "ga-t2-shared", true)
                .await
                .unwrap_err()
                .status(),
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn github_installations_follow_their_app() {
        let db = github_pool().await;
        let alice = user("alice", "user");
        assert!(authorize_github_installation(&db, &alice, "in-t1", true)
            .await
            .is_ok());
        assert_eq!(
            authorize_github_installation(&db, &alice, "in-t2", false)
                .await
                .unwrap_err()
                .status(),
            StatusCode::FORBIDDEN
        );
        assert!(
            authorize_github_installation(&db, &alice, "in-t2-shared", false)
                .await
                .is_ok()
        );
        assert_eq!(
            authorize_github_installation(&db, &alice, "nope", false)
                .await
                .unwrap_err()
                .status(),
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn tunnels_follow_team_membership() {
        let db = github_pool().await;
        let alice = user("alice", "user");
        let t = |u: &User, id: &'static str| {
            let (db, u) = (db.clone(), u.clone());
            async move { authorize_team_row(&db, &u, TeamTable::CloudflareTunnels, id).await }
        };
        assert!(t(&alice, "tn-t1").await.is_ok());
        assert!(t(&alice, "tn-legacy").await.is_ok());
        assert_eq!(
            t(&alice, "tn-t2").await.unwrap_err().status(),
            StatusCode::FORBIDDEN
        );
        assert!(t(&user("root", "admin"), "tn-t2").await.is_ok());
        assert_eq!(
            t(&alice, "none").await.unwrap_err().status(),
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn app_bound_ssh_keys_and_tunnels_need_developer_role_to_write() {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for sql in [
            "CREATE TABLE apps (id TEXT, team_id TEXT, project_id TEXT)",
            "CREATE TABLE team_resource_permissions (team_id TEXT, user_id TEXT, resource_type TEXT, resource_id TEXT, permission TEXT)",
            "CREATE TABLE teams (id TEXT, require_2fa INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE users (id TEXT, totp_enabled INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE user_passkeys (id TEXT, user_id TEXT)",
            "CREATE TABLE ssh_keys (id TEXT, app_id TEXT)",
            "CREATE TABLE cloudflare_tunnels (id TEXT, team_id TEXT)",
            "CREATE TABLE team_members (team_id TEXT, user_id TEXT, role TEXT)",
            "INSERT INTO team_members VALUES ('t1', 'dev', 'developer')",
            "INSERT INTO team_members VALUES ('t1', 'view', 'viewer')",
            "INSERT INTO apps VALUES ('a1', 't1', NULL)",
            "INSERT INTO ssh_keys VALUES ('k-app', 'a1')",
            "INSERT INTO ssh_keys VALUES ('k-global', NULL)",
            "INSERT INTO cloudflare_tunnels VALUES ('tn1', 't1')",
        ] {
            sqlx::query(sql).execute(&db).await.unwrap();
        }
        let w = |u: &str, kind: ResourceKind, id: &'static str| {
            let (db, u) = (db.clone(), user(u, "member"));
            async move { authorize_write(&db, &u, kind, id).await }
        };
        assert!(w("dev", ResourceKind::SshKey, "k-app").await.is_ok());
        assert_eq!(
            w("view", ResourceKind::SshKey, "k-app")
                .await
                .unwrap_err()
                .status(),
            StatusCode::FORBIDDEN
        );
        // Global keys have no team owner and stay open.
        assert!(w("view", ResourceKind::SshKey, "k-global").await.is_ok());
        assert!(w("dev", ResourceKind::Tunnel, "tn1").await.is_ok());
        assert_eq!(
            w("view", ResourceKind::Tunnel, "tn1")
                .await
                .unwrap_err()
                .status(),
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn global_ssh_keys_are_editable_only_by_creator_or_admin() {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for sql in [
            "CREATE TABLE ssh_keys (id TEXT, app_id TEXT, created_by TEXT)",
            "INSERT INTO ssh_keys VALUES ('k-mine', NULL, 'alice')",
            "INSERT INTO ssh_keys VALUES ('k-legacy', NULL, NULL)",
            "INSERT INTO ssh_keys VALUES ('k-app', 'a1', 'alice')",
        ] {
            sqlx::query(sql).execute(&db).await.unwrap();
        }
        let check = |u: User, id: &'static str| {
            let db = db.clone();
            async move { authorize_global_ssh_key_write(&db, &u, id).await }
        };
        assert!(check(user("alice", "member"), "k-mine").await.is_ok());
        assert!(check(user("root", "admin"), "k-mine").await.is_ok());
        assert_eq!(
            check(user("bob", "member"), "k-mine")
                .await
                .unwrap_err()
                .status(),
            StatusCode::FORBIDDEN
        );
        // Keys from before created_by existed are admin-only.
        assert!(check(user("root", "admin"), "k-legacy").await.is_ok());
        assert_eq!(
            check(user("alice", "member"), "k-legacy")
                .await
                .unwrap_err()
                .status(),
            StatusCode::FORBIDDEN
        );
        // App-bound keys are gated by the resource middleware, not here.
        assert!(check(user("bob", "member"), "k-app").await.is_ok());
        assert_eq!(
            check(user("bob", "member"), "missing")
                .await
                .unwrap_err()
                .status(),
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn token_resource_lists_cover_children() {
        use crate::api::api_tokens::TokenResource;
        let list = vec![
            TokenResource {
                kind: "project".into(),
                id: "p1".into(),
            },
            TokenResource {
                kind: "app".into(),
                id: "a9".into(),
            },
        ];
        let in_p1 = ResourceOwner {
            team_id: Some("t".into()),
            project_id: Some("p1".into()),
            app_id: Some("a1".into()),
        };
        let elsewhere = ResourceOwner {
            team_id: Some("t".into()),
            project_id: Some("p2".into()),
            app_id: Some("a2".into()),
        };
        // An app, a deployment and a database inside a listed project.
        assert!(resource_listed(&list, ResourceKind::App, "a1", &in_p1));
        assert!(resource_listed(
            &list,
            ResourceKind::Deployment,
            "d1",
            &in_p1
        ));
        assert!(resource_listed(&list, ResourceKind::Project, "p1", &in_p1));
        // A listed app outside the project, and its deployments.
        let a9 = ResourceOwner {
            app_id: Some("a9".into()),
            ..elsewhere.clone()
        };
        assert!(resource_listed(&list, ResourceKind::App, "a9", &a9));
        assert!(resource_listed(&list, ResourceKind::Deployment, "d9", &a9));
        // Anything else is refused.
        assert!(!resource_listed(&list, ResourceKind::App, "a2", &elsewhere));
        assert!(!resource_listed(
            &list,
            ResourceKind::Server,
            "s1",
            &ResourceOwner::default()
        ));
    }

    #[test]
    fn team_restricted_reads_get_the_team_filter() {
        let req = |uri: &str| Request::builder().uri(uri).body(Body::empty()).unwrap();
        let r = force_team_query(req("/api/apps"), "t1").unwrap();
        assert_eq!(r.uri().to_string(), "/api/apps?team_id=t1");
        let r = force_team_query(req("/api/apps?limit=5"), "t1").unwrap();
        assert_eq!(r.uri().query(), Some("limit=5&team_id=t1"));
        assert!(force_team_query(req("/api/apps?team_id=t1"), "t1").is_ok());
        assert!(force_team_query(req("/api/apps?team_id=t2"), "t1").is_err());
    }

    #[tokio::test]
    async fn team_restricted_writes_need_the_team_in_the_body() {
        let req = |body: &str, ct: &str| {
            Request::builder()
                .method(Method::POST)
                .uri("/api/apps")
                .header("content-type", ct)
                .body(Body::from(body.to_string()))
                .unwrap()
        };
        let ok = require_team_in_body(
            req(r#"{"name":"x","team_id":"t1"}"#, "application/json"),
            "t1",
        )
        .await
        .unwrap();
        // The body is passed on intact.
        let bytes = axum::body::to_bytes(ok.into_body(), 1024).await.unwrap();
        assert_eq!(&bytes[..], br#"{"name":"x","team_id":"t1"}"#);
        assert!(
            require_team_in_body(req(r#"{"team_id":"t2"}"#, "application/json"), "t1")
                .await
                .is_err()
        );
        assert!(
            require_team_in_body(req(r#"{"name":"x"}"#, "application/json"), "t1")
                .await
                .is_err()
        );
        assert!(require_team_in_body(req("x", "multipart/form-data"), "t1")
            .await
            .is_err());
    }

    #[test]
    fn parse_ignores_collections_literals_and_other_paths() {
        for p in [
            "/api/apps",
            "/api/apps/with-sharing",
            "/api/services/check-port",
            "/api/build-servers",
            "/api/previews",
            "/api/previews/status/running",
            "/api/s3/backup",
            "/api/s3/configs",
            "/api/s3/unknown/123e4567-e89b-12d3-a456-426614174000",
            "/api/tokens/123e4567-e89b-12d3-a456-426614174000",
            "/health",
            "/api",
        ] {
            assert!(parse_resource_target(p).is_none(), "{p}");
        }
    }

    #[tokio::test]
    async fn team_access_member_non_member_global_and_privileged() {
        let db = pool().await;
        let alice = user("alice", "user");
        let bob = user("bob", "user");
        assert!(check_team_access_db(&db, &alice, Some("t1")).await.is_ok());
        assert!(check_team_access_db(&db, &alice, Some("t2")).await.is_err());
        assert!(check_team_access_db(&db, &bob, Some("t1")).await.is_err());
        // Legacy team-less resources stay open to any authenticated user.
        assert!(check_team_access_db(&db, &bob, None).await.is_ok());
        // Instance admins and the admin-token user bypass.
        assert!(
            check_team_access_db(&db, &user("root", "admin"), Some("t2"))
                .await
                .is_ok()
        );
        assert!(
            check_team_access_db(&db, &user("system", "admin"), Some("t2"))
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn build_server_rows_follow_team_membership() {
        let db = pool().await;
        let alice = user("alice", "user");
        let bob = user("bob", "user");
        let admin = user("root", "admin");
        let check = |u: &User, id: &'static str| {
            let db = db.clone();
            let u = u.clone();
            async move { authorize_team_row(&db, &u, TeamTable::BuildServers, id).await }
        };
        assert!(check(&alice, "bs-t1").await.is_ok());
        assert_eq!(
            check(&alice, "bs-t2").await.unwrap_err().status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            check(&bob, "bs-t1").await.unwrap_err().status(),
            StatusCode::FORBIDDEN
        );
        assert!(check(&bob, "bs-global").await.is_ok());
        assert!(check(&admin, "bs-t2").await.is_ok());
        assert_eq!(
            check(&alice, "missing").await.unwrap_err().status(),
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn writes_need_developer_role_and_shares_are_read_only() {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for sql in [
            "CREATE TABLE apps (id TEXT, team_id TEXT, project_id TEXT)",
            "CREATE TABLE team_resource_permissions (team_id TEXT, user_id TEXT, resource_type TEXT, resource_id TEXT, permission TEXT)",
            "CREATE TABLE teams (id TEXT, require_2fa INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE users (id TEXT, totp_enabled INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE user_passkeys (id TEXT, user_id TEXT)",
            "CREATE TABLE team_members (team_id TEXT, user_id TEXT, role TEXT)",
            "INSERT INTO team_members VALUES ('t1', 'dev', 'developer')",
            "INSERT INTO team_members VALUES ('t1', 'view', 'viewer')",
            "INSERT INTO team_members VALUES ('t1', 'adm', 'admin')",
            "INSERT INTO apps VALUES ('a1', 't1', NULL)",
            "INSERT INTO apps VALUES ('legacy', NULL, NULL)",
        ] {
            sqlx::query(sql).execute(&db).await.unwrap();
        }
        let w = |u: &str, app: &'static str| {
            let db = db.clone();
            let u = user(u, "member");
            async move { authorize_write(&db, &u, ResourceKind::App, app).await }
        };
        assert!(w("dev", "a1").await.is_ok());
        assert!(w("adm", "a1").await.is_ok());
        assert_eq!(
            w("view", "a1").await.unwrap_err().status(),
            StatusCode::FORBIDDEN
        );
        // Not a member (shared-only access) is read-only.
        assert_eq!(
            w("outsider", "a1").await.unwrap_err().status(),
            StatusCode::FORBIDDEN
        );
        assert!(w("view", "legacy").await.is_ok());
        assert!(
            authorize_write(&db, &user("root", "admin"), ResourceKind::App, "a1")
                .await
                .is_ok()
        );
        // Kinds without a team lookup are not role-gated here.
        assert!(
            authorize_write(&db, &user("view", "member"), ResourceKind::S3Backup, "x")
                .await
                .is_ok()
        );
    }

    /// Presets, custom roles and per-resource overrides through the same check
    /// the resource middleware runs.
    #[tokio::test]
    async fn abilities_presets_custom_roles_and_overrides() {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for sql in [
            "CREATE TABLE apps (id TEXT, team_id TEXT, project_id TEXT)",
            "CREATE TABLE teams (id TEXT, require_2fa INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE users (id TEXT, totp_enabled INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE user_passkeys (id TEXT, user_id TEXT)",
            "CREATE TABLE deployments (id TEXT, app_id TEXT)",
            "CREATE TABLE databases (id TEXT, team_id TEXT, project_id TEXT)",
            "CREATE TABLE team_members (id TEXT, team_id TEXT, user_id TEXT, role TEXT, created_at TEXT)",
            "CREATE TABLE team_roles (id TEXT, team_id TEXT, name TEXT, description TEXT, abilities TEXT, created_at TEXT, updated_at TEXT)",
            "CREATE TABLE team_resource_permissions (team_id TEXT, user_id TEXT, resource_type TEXT, resource_id TEXT, permission TEXT)",
            "INSERT INTO apps VALUES ('a1', 't1', 'p1'), ('a2', 't1', 'p2')",
            "INSERT INTO deployments VALUES ('d1', 'a1')",
            "INSERT INTO databases VALUES ('db1', 't1', 'p1')",
            "INSERT INTO team_members VALUES ('m1', 't1', 'view', 'viewer', ''), \
             ('m2', 't1', 'dep', 'deployer', ''), ('m3', 't1', 'dev', 'developer', ''), \
             ('m4', 't1', 'ops', 'custom:r1', ''), ('m5', 't1', 'gone', 'custom:deleted', ''), \
             ('m6', 't1', 'adm', 'admin', ''), ('m7', 't1', 'own', 'owner', '')",
            "INSERT INTO team_roles VALUES ('r1', 't1', 'Ops', NULL, '[\"apps.read\",\"apps.deploy\",\"databases.read\"]', '', '')",
        ] {
            sqlx::query(sql).execute(&db).await.unwrap();
        }
        let can = |u: &'static str, kind: ResourceKind, id: &'static str, action: ActionKind| {
            let db = db.clone();
            async move {
                authorize_action(&db, &user(u, "member"), kind, id, action)
                    .await
                    .is_ok()
            }
        };
        use ActionKind::*;
        use ResourceKind::{App, Database, Deployment};

        // Viewer reads but cannot deploy.
        assert!(can("view", App, "a1", Read).await);
        assert!(!can("view", App, "a1", Deploy).await);
        assert!(!can("view", Deployment, "d1", Deploy).await);
        // Deployer deploys but cannot edit or delete.
        assert!(can("dep", App, "a1", Deploy).await);
        assert!(can("dep", Deployment, "d1", Deploy).await);
        assert!(!can("dep", App, "a1", Write).await);
        assert!(!can("dep", App, "a1", Delete).await);
        assert!(!can("dep", Database, "db1", Delete).await);
        assert!(!can("dep", App, "a1", Secrets).await);
        // Developer keeps the old developer powers, including delete and secrets.
        assert!(can("dev", App, "a1", Delete).await);
        assert!(can("dev", App, "a1", Secrets).await);
        // Custom role: exactly its abilities.
        assert!(can("ops", App, "a1", Deploy).await);
        assert!(!can("ops", App, "a1", Write).await);
        assert!(can("ops", Database, "db1", Read).await);
        assert!(!can("ops", Database, "db1", Deploy).await);
        // A deleted custom role grants nothing.
        assert!(!can("gone", App, "a1", Read).await);

        // Overrides on top of the role.
        for sql in [
            // Viewer may change a1 (allow) but nothing in project p2 (deny).
            "INSERT INTO team_resource_permissions VALUES ('t1', 'view', 'app', 'a1', 'allow')",
            "INSERT INTO team_resource_permissions VALUES ('t1', 'view', 'project', 'p2', 'deny')",
            // Developer is denied a1 but allowed the rest of project p1.
            "INSERT INTO team_resource_permissions VALUES ('t1', 'dev', 'app', 'a1', 'deny')",
            "INSERT INTO team_resource_permissions VALUES ('t1', 'dev', 'project', 'p1', 'allow')",
        ] {
            sqlx::query(sql).execute(&db).await.unwrap();
        }
        assert!(can("view", App, "a1", Write).await);
        assert!(can("view", Deployment, "d1", Deploy).await);
        // Allow never grants delete or secrets.
        assert!(!can("view", App, "a1", Delete).await);
        assert!(!can("view", App, "a1", Secrets).await);
        // Project deny covers its apps, even reads.
        assert!(!can("view", App, "a2", Read).await);
        // Deny on the app wins over allow on its project.
        assert!(!can("dev", App, "a1", Read).await);
        assert!(!can("dev", Deployment, "d1", Deploy).await);
        assert!(can("dev", Database, "db1", Delete).await);
        // Instance admins bypass everything.
        assert!(
            authorize_action(&db, &user("root", "admin"), App, "a1", Delete)
                .await
                .is_ok()
        );

        // Team route checks and the "no escalation" rule.
        assert!(
            require_team_ability(&db, "t1", "adm", Some(Ability::TeamManage))
                .await
                .is_ok()
        );
        assert!(
            require_team_ability(&db, "t1", "adm", Some(Ability::TeamDelete))
                .await
                .is_err()
        );
        assert!(
            require_team_ability(&db, "t1", "dev", Some(Ability::TeamManage))
                .await
                .is_err()
        );
        assert!(require_team_ability(&db, "t1", "nobody", None)
            .await
            .is_err());
        assert!(
            require_team_ability(&db, "t1", "system", Some(Ability::TeamDelete))
                .await
                .is_ok()
        );
        assert!(can_manage_role(&db, "t1", "owner", "owner").await.unwrap());
        assert!(!can_manage_role(&db, "t1", "admin", "owner").await.unwrap());
        assert!(can_manage_role(&db, "t1", "admin", "custom:r1")
            .await
            .unwrap());
        // A developer cannot hand out admin (it lacks team.manage).
        assert!(!can_manage_role(&db, "t1", "developer", "admin")
            .await
            .unwrap());
    }

    /// A team that requires 2FA blocks members without TOTP or a passkey from
    /// its resources and team management, with the `two_factor_required` code.
    #[tokio::test]
    async fn team_2fa_requirement_blocks_members_without_2fa() {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for sql in [
            "CREATE TABLE teams (id TEXT, require_2fa INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE users (id TEXT, totp_enabled INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE user_passkeys (id TEXT, user_id TEXT)",
            "CREATE TABLE apps (id TEXT, team_id TEXT, project_id TEXT)",
            "CREATE TABLE team_members (id TEXT, team_id TEXT, user_id TEXT, role TEXT, created_at TEXT)",
            "CREATE TABLE team_roles (id TEXT, team_id TEXT, name TEXT, description TEXT, abilities TEXT, created_at TEXT, updated_at TEXT)",
            "CREATE TABLE team_resource_permissions (team_id TEXT, user_id TEXT, resource_type TEXT, resource_id TEXT, permission TEXT)",
            "INSERT INTO teams VALUES ('t1', 1), ('t2', 0)",
            "INSERT INTO users VALUES ('plain', 0), ('totp', 1), ('pk', 0)",
            "INSERT INTO user_passkeys VALUES ('k1', 'pk')",
            "INSERT INTO apps VALUES ('a1', 't1', NULL), ('a2', 't2', NULL)",
            "INSERT INTO team_members VALUES ('m1', 't1', 'plain', 'admin', ''), \
             ('m2', 't1', 'totp', 'admin', ''), ('m3', 't1', 'pk', 'admin', ''), \
             ('m4', 't2', 'plain', 'admin', '')",
        ] {
            sqlx::query(sql).execute(&db).await.unwrap();
        }
        let act = |u: &'static str, id: &'static str| {
            let db = db.clone();
            async move {
                authorize_action(
                    &db,
                    &user(u, "member"),
                    ResourceKind::App,
                    id,
                    ActionKind::Read,
                )
                .await
            }
        };
        let err = act("plain", "a1").await.unwrap_err();
        assert_eq!(err.status(), StatusCode::FORBIDDEN);
        assert!(format!("{err}").contains("two_factor_required"));
        // TOTP or a passkey satisfies the requirement.
        assert!(act("totp", "a1").await.is_ok());
        assert!(act("pk", "a1").await.is_ok());
        // Teams without the requirement are unaffected.
        assert!(act("plain", "a2").await.is_ok());
        // Team management is blocked too; reading the team is not.
        assert!(
            require_team_ability(&db, "t1", "plain", Some(Ability::TeamManage))
                .await
                .is_err()
        );
        assert!(require_team_ability(&db, "t1", "plain", None).await.is_ok());
        assert!(
            require(&db, &user("plain", "member"), Some("t1"), Ability::AppsRead)
                .await
                .is_err()
        );
        // Instance admins and the admin token are not team members and bypass.
        assert!(authorize_action(
            &db,
            &user("root", "admin"),
            ResourceKind::App,
            "a1",
            ActionKind::Write
        )
        .await
        .is_ok());
        assert!(two_factor_blocked(&db, "t1", "plain").await.unwrap());
        assert!(!two_factor_blocked(&db, "t2", "plain").await.unwrap());
    }

    #[test]
    fn override_specificity() {
        let rows = vec![
            ("project".to_string(), "p1".to_string(), "deny".to_string()),
            ("app".to_string(), "a1".to_string(), "allow".to_string()),
        ];
        let owner = ResourceOwner {
            team_id: Some("t".into()),
            project_id: Some("p1".into()),
            app_id: Some("a1".into()),
        };
        // Deny on the project wins over allow on the app.
        assert_eq!(
            pick_override(&rows, ResourceKind::App, "a1", &owner),
            Some(ResourceOverride::Deny)
        );
        assert_eq!(
            pick_override(&rows, ResourceKind::Deployment, "d1", &owner),
            Some(ResourceOverride::Deny)
        );
        // Allow alone applies.
        let in_p9 = ResourceOwner {
            project_id: Some("p9".into()),
            ..owner.clone()
        };
        assert_eq!(
            pick_override(&rows, ResourceKind::App, "a1", &in_p9),
            Some(ResourceOverride::Allow)
        );
        let other = ResourceOwner {
            app_id: Some("a5".into()),
            ..owner.clone()
        };
        assert_eq!(
            pick_override(&rows, ResourceKind::App, "a5", &other),
            Some(ResourceOverride::Deny)
        );
        assert_eq!(
            pick_override(&rows, ResourceKind::App, "z", &ResourceOwner::default()),
            None
        );
    }

    #[tokio::test]
    async fn retain_visible_filters_other_teams() {
        let db = pool().await;
        let rows = vec![("a", Some("t1")), ("b", Some("t2")), ("c", None)];
        fn names<'a>(v: Vec<(&'a str, Option<&'a str>)>) -> Vec<&'a str> {
            v.into_iter().map(|r| r.0).collect()
        }
        let alice = retain_visible_db(&db, &user("alice", "user"), rows.clone(), |r| r.1)
            .await
            .unwrap();
        assert_eq!(names(alice), ["a", "c"]);
        let bob = retain_visible_db(&db, &user("bob", "user"), rows.clone(), |r| r.1)
            .await
            .unwrap();
        assert_eq!(names(bob), ["c"]);
        let admin = retain_visible_db(&db, &user("root", "admin"), rows, |r| r.1)
            .await
            .unwrap();
        assert_eq!(names(admin), ["a", "b", "c"]);
    }
}
