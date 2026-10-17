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
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::db::{App, ManagedDatabase, Project, Server, Service, User};
use crate::AppState;
use sqlx::SqlitePool;

use super::error::ApiError;

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
        .filter(|i| team_of(i).map_or(true, |t| teams.iter().any(|m| m == t)))
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
}

impl TeamTable {
    fn select_sql(self) -> &'static str {
        match self {
            Self::BuildServers => "SELECT team_id FROM build_servers WHERE id = ?",
            Self::NotificationChannels => "SELECT team_id FROM notification_channels WHERE id = ?",
            Self::Destinations => "SELECT team_id FROM destinations WHERE id = ?",
            Self::CaCertificates => "SELECT team_id FROM ca_certificates WHERE id = ?",
            Self::S3StorageConfigs => "SELECT team_id FROM s3_storage_configs WHERE id = ?",
        }
    }

    fn not_found(self) -> &'static str {
        match self {
            Self::BuildServers => "Build server not found",
            Self::NotificationChannels => "Notification channel not found",
            Self::Destinations => "Destination not found",
            Self::CaCertificates => "CA certificate not found",
            Self::S3StorageConfigs => "S3 configuration not found",
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
    // Identify (resource_kind, id) from the path, if any.
    let path = request.uri().path().to_string();
    let target = parse_resource_target(&path);

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
        };

        if let Err(e) = result {
            return Err(e.into_response());
        }
    }

    Ok(next.run(request).await)
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
enum ResourceKind {
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
}

/// Extract `(kind, id)` from a request path when it targets a specific resource.
/// Returns `None` for collection paths, non-resource paths, or non-UUID segments.
fn parse_resource_target(path: &str) -> Option<(ResourceKind, &str)> {
    let rest = path.strip_prefix("/api/")?;
    let mut segs = rest.split('/');
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
        "s3" => match segs.next()? {
            "configs" => ResourceKind::S3Config,
            "backups" => ResourceKind::S3Backup,
            _ => return None,
        },
        _ => return None,
    };
    let id = segs.next()?;
    // Only enforce when the segment is an actual resource id (UUID). Literal
    // sub-routes like `with-sharing` / `check-port` fall through to the handler.
    if uuid::Uuid::parse_str(id).is_ok() {
        Some((kind, id))
    } else {
        None
    }
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
