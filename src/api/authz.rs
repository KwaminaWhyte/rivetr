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
pub async fn user_team_ids(
    state: &Arc<AppState>,
    user_id: &str,
) -> Result<Vec<String>, ApiError> {
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
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM team_members WHERE team_id = ? AND user_id = ?",
    )
    .bind(team_id)
    .bind(user_id)
    .fetch_one(&state.db)
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
    if is_privileged(user) {
        return Ok(());
    }
    match team_id {
        // Legacy / unassigned resource — accessible to any authenticated user.
        None => Ok(()),
        Some(tid) => {
            if is_team_member(state, &user.id, tid).await? {
                Ok(())
            } else {
                Err(ApiError::forbidden(
                    "You do not have access to this resource",
                ))
            }
        }
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
    let app_id: Option<String> =
        sqlx::query_scalar("SELECT app_id FROM deployments WHERE id = ?")
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
        let token = extract_request_token(&request).ok_or_else(|| {
            StatusCode::UNAUTHORIZED.into_response()
        })?;
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
        };

        if let Err(e) = result {
            return Err(e.into_response());
        }
    }

    Ok(next.run(request).await)
}

#[derive(Clone, Copy)]
enum ResourceKind {
    App,
    Server,
    Database,
    Service,
    Project,
    Deployment,
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
