//! Secret manager connections, path imports and failure policy (gap #9).
//!
//! Connections are team scoped (`team_id IS NULL` = instance wide, admin only to change).
//! Credentials are write-only: responses say which credential fields are set, never
//! their values. Browsing returns folder and key names, never secret values.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::audit::{audit_log, ClientIp};
use super::authz;
use super::error::ApiError;
use crate::crypto;
use crate::db::{TeamRole, User};
use crate::secret_managers::providers::{
    self, build_provider, ConnectionConfig, ConnectionCredentials, Listing, DEFAULT_TIMEOUT,
};
use crate::secret_managers::reference::is_valid_connection_name;
use crate::secret_managers::store::{
    self, SecretConnectionRow, SecretImportRow, POLICY_FAIL, POLICY_LAST_KNOWN,
};
use crate::AppState;

const RESOURCE: &str = "secret_connection";

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/secret-connections",
            get(list_connections).post(create_connection),
        )
        .route("/secret-connections/test", post(test_unsaved))
        .route(
            "/secret-connections/{id}",
            get(get_connection)
                .put(update_connection)
                .delete(delete_connection),
        )
        .route("/secret-connections/{id}/test", post(test_connection))
        .route("/secret-connections/{id}/browse", get(browse_connection))
        .route(
            "/apps/{id}/secret-imports",
            get(list_app_imports).post(create_app_import),
        )
        .route(
            "/apps/{id}/secret-imports/{import_id}",
            delete(delete_app_import),
        )
        .route(
            "/apps/{id}/secret-policy",
            get(get_app_policy).put(set_app_policy),
        )
        .route(
            "/services/{id}/secret-imports",
            get(list_service_imports).post(create_service_import),
        )
        .route(
            "/services/{id}/secret-imports/{import_id}",
            delete(delete_service_import),
        )
        .route(
            "/services/{id}/secret-policy",
            get(get_service_policy).put(set_service_policy),
        )
        .route(
            "/projects/{id}/secret-imports",
            get(list_project_imports).post(create_project_import),
        )
        .route(
            "/projects/{id}/secret-imports/{import_id}",
            delete(delete_project_import),
        )
}

fn encryption_key(state: &AppState) -> Option<[u8; 32]> {
    state
        .config
        .auth
        .encryption_key
        .as_ref()
        .map(|s| crypto::derive_key(s))
}

fn db_err(e: impl std::fmt::Display) -> ApiError {
    ApiError::database(e.to_string())
}

// ---------------------------------------------------------------------------
// Responses / requests
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ConnectionResponse {
    pub id: String,
    pub team_id: Option<String>,
    pub name: String,
    pub provider: String,
    pub config: ConnectionConfig,
    /// Names of the credential fields that are stored (values are never returned).
    pub credentials_set: Vec<&'static str>,
    pub last_tested_at: Option<String>,
    pub last_test_ok: Option<bool>,
    pub last_test_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn credentials_set(c: &ConnectionCredentials) -> Vec<&'static str> {
    let mut out = vec![];
    let mut push = |name: &'static str, v: &Option<String>| {
        if v.as_deref().is_some_and(|s| !s.is_empty()) {
            out.push(name);
        }
    };
    push("token", &c.token);
    push("secret_id", &c.secret_id);
    push("client_secret", &c.client_secret);
    push("secret_access_key", &c.secret_access_key);
    push("session_token", &c.session_token);
    out
}

fn to_response(row: &SecretConnectionRow, key: Option<&[u8; 32]>) -> ConnectionResponse {
    let creds = row.credentials(key).unwrap_or_default();
    ConnectionResponse {
        id: row.id.clone(),
        team_id: row.team_id.clone(),
        name: row.name.clone(),
        provider: row.provider.clone(),
        config: row.config(),
        credentials_set: credentials_set(&creds),
        last_tested_at: row.last_tested_at.clone(),
        last_test_ok: row.last_test_ok.map(|v| v != 0),
        last_test_message: row.last_test_message.clone(),
        created_at: row.created_at.clone(),
        updated_at: row.updated_at.clone(),
    }
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub team_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateConnectionRequest {
    pub team_id: Option<String>,
    pub name: String,
    pub provider: String,
    #[serde(default)]
    pub config: ConnectionConfig,
    #[serde(default)]
    pub credentials: ConnectionCredentials,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConnectionRequest {
    pub name: Option<String>,
    pub config: Option<ConnectionConfig>,
    /// Blank fields keep the stored value.
    pub credentials: Option<ConnectionCredentials>,
}

#[derive(Debug, Deserialize)]
pub struct TestUnsavedRequest {
    pub provider: String,
    #[serde(default)]
    pub config: ConnectionConfig,
    #[serde(default)]
    pub credentials: ConnectionCredentials,
}

#[derive(Debug, Serialize)]
pub struct TestResult {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct BrowseQuery {
    #[serde(default)]
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateImportRequest {
    pub connection_id: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub prefix: String,
}

#[derive(Debug, Serialize)]
pub struct ImportResponse {
    #[serde(flatten)]
    pub import: SecretImportRow,
    pub connection_name: Option<String>,
    pub provider: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PolicyBody {
    pub failure_policy: String,
    #[serde(default, skip_deserializing)]
    pub last_resolved_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Authorization helpers
// ---------------------------------------------------------------------------

async fn team_role(
    state: &AppState,
    user: &User,
    team_id: &str,
) -> Result<Option<TeamRole>, ApiError> {
    let role: Option<String> =
        sqlx::query_scalar("SELECT role FROM team_members WHERE team_id = ? AND user_id = ?")
            .bind(team_id)
            .bind(&user.id)
            .fetch_optional(&state.db)
            .await
            .map_err(db_err)?;
    Ok(role.map(TeamRole::from))
}

/// Reading a connection (list, browse, test): team member, or anyone for instance-wide
/// connections.
async fn require_read(
    state: &Arc<AppState>,
    user: &User,
    team_id: Option<&str>,
) -> Result<(), ApiError> {
    if authz::is_privileged_user(user) {
        return Ok(());
    }
    match team_id {
        None => Ok(()),
        Some(t) => {
            if team_role(state, user, t).await?.is_some() {
                Ok(())
            } else {
                Err(ApiError::forbidden(
                    "You do not have access to this connection",
                ))
            }
        }
    }
}

/// Changing connections (they hold credentials): team admin or owner; instance-wide
/// connections need an instance administrator.
async fn require_manage(
    state: &Arc<AppState>,
    user: &User,
    team_id: Option<&str>,
) -> Result<(), ApiError> {
    if authz::is_privileged_user(user) {
        return Ok(());
    }
    match team_id {
        None => Err(ApiError::forbidden(
            "Instance-wide secret manager connections can only be changed by an instance administrator",
        )),
        Some(t) => match team_role(state, user, t).await? {
            Some(r) if r.has_at_least(TeamRole::Admin) => Ok(()),
            _ => Err(ApiError::forbidden(
                "Managing secret manager connections requires the team admin role",
            )),
        },
    }
}

async fn load_connection(state: &AppState, id: &str) -> Result<SecretConnectionRow, ApiError> {
    store::get_connection(&state.db, id)
        .await
        .map_err(db_err)?
        .ok_or_else(|| ApiError::not_found("Connection not found"))
}

/// Outbound URLs must not point at internal addresses unless an admin allowed them
/// (Settings > Advanced > Outbound). Instance administrators are trusted.
async fn check_outbound(
    user: &User,
    provider: &str,
    config: &ConnectionConfig,
) -> Result<(), ApiError> {
    if authz::is_privileged_user(user) {
        return Ok(());
    }
    for url in providers::outbound_urls(provider, config) {
        super::ssrf::validate_external_url(&url).await?;
    }
    Ok(())
}

fn validate(provider: &str, name: &str) -> Result<(), ApiError> {
    if !providers::PROVIDERS.contains(&provider) {
        return Err(ApiError::bad_request(format!(
            "Unknown provider '{}'. Use one of: {}",
            provider,
            providers::PROVIDERS.join(", ")
        )));
    }
    if !is_valid_connection_name(name) {
        return Err(ApiError::validation_field(
            "name",
            "Use lowercase letters, digits, '-' and '_' (it appears in secret:// references)",
        ));
    }
    Ok(())
}

/// Check the settings would build a client (required fields present).
fn validate_buildable(
    provider: &str,
    config: &ConnectionConfig,
    creds: &ConnectionCredentials,
) -> Result<(), ApiError> {
    build_provider(provider, config, creds, DEFAULT_TIMEOUT)
        .map(|_| ())
        .map_err(|e| ApiError::bad_request(e.to_string()))
}

// ---------------------------------------------------------------------------
// Connections
// ---------------------------------------------------------------------------

async fn list_connections(
    State(state): State<Arc<AppState>>,
    user: User,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<ConnectionResponse>>, ApiError> {
    let team = q.team_id.as_deref().filter(|t| !t.is_empty());
    authz::authorize_team_filter(&state, &user, team).await?;
    let rows: Vec<SecretConnectionRow> = match team {
        // A team sees its own connections plus instance-wide ones.
        Some(t) => sqlx::query_as(
            "SELECT * FROM secret_connections WHERE team_id = ? OR team_id IS NULL ORDER BY name",
        )
        .bind(t)
        .fetch_all(&state.db)
        .await
        .map_err(db_err)?,
        None => sqlx::query_as("SELECT * FROM secret_connections ORDER BY name")
            .fetch_all(&state.db)
            .await
            .map_err(db_err)?,
    };
    let rows = authz::retain_visible(&state, &user, rows, |r| r.team_id.as_deref()).await?;
    let key = encryption_key(&state);
    Ok(Json(
        rows.iter().map(|r| to_response(r, key.as_ref())).collect(),
    ))
}

async fn get_connection(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
) -> Result<Json<ConnectionResponse>, ApiError> {
    let row = load_connection(&state, &id).await?;
    require_read(&state, &user, row.team_id.as_deref()).await?;
    Ok(Json(to_response(&row, encryption_key(&state).as_ref())))
}

async fn create_connection(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Json(req): Json<CreateConnectionRequest>,
) -> Result<(StatusCode, Json<ConnectionResponse>), ApiError> {
    let team_id = req.team_id.as_deref().filter(|t| !t.is_empty());
    authz::authorize_team_assignment(&state, &user, team_id).await?;
    require_manage(&state, &user, team_id).await?;
    validate(&req.provider, &req.name)?;
    validate_buildable(&req.provider, &req.config, &req.credentials)?;
    check_outbound(&user, &req.provider, &req.config).await?;

    let key = encryption_key(&state);
    let sealed = store::seal_credentials(&req.credentials, key.as_ref())
        .map_err(|e| ApiError::bad_request(format!("{:#}", e)))?;
    let id = Uuid::new_v4().to_string();
    let config_json = serde_json::to_string(&req.config).map_err(db_err)?;
    let res = sqlx::query(
        "INSERT INTO secret_connections (id, team_id, name, provider, config, credentials, created_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(team_id)
    .bind(&req.name)
    .bind(&req.provider)
    .bind(&config_json)
    .bind(&sealed)
    .bind(&user.id)
    .execute(&state.db)
    .await;
    if let Err(e) = res {
        return Err(if e.to_string().contains("UNIQUE") {
            ApiError::conflict(format!(
                "A connection named '{}' already exists in this team",
                req.name
            ))
        } else {
            db_err(e)
        });
    }

    audit_log(
        &state,
        "secret_connection.create",
        RESOURCE,
        Some(&id),
        Some(&req.name),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "provider": req.provider, "team_id": team_id })),
    )
    .await;

    let row = load_connection(&state, &id).await?;
    Ok((StatusCode::CREATED, Json(to_response(&row, key.as_ref()))))
}

async fn update_connection(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(id): Path<String>,
    Json(req): Json<UpdateConnectionRequest>,
) -> Result<Json<ConnectionResponse>, ApiError> {
    let row = load_connection(&state, &id).await?;
    require_manage(&state, &user, row.team_id.as_deref()).await?;
    let key = encryption_key(&state);

    let name = req.name.clone().unwrap_or_else(|| row.name.clone());
    validate(&row.provider, &name)?;
    let config = req.config.clone().unwrap_or_else(|| row.config());
    let previous = row
        .credentials(key.as_ref())
        .map_err(|e| ApiError::internal(format!("{:#}", e)))?;
    let creds = match req.credentials.clone() {
        Some(c) => c.merged_over(&previous),
        None => previous,
    };
    validate_buildable(&row.provider, &config, &creds)?;
    check_outbound(&user, &row.provider, &config).await?;
    let sealed = store::seal_credentials(&creds, key.as_ref())
        .map_err(|e| ApiError::bad_request(format!("{:#}", e)))?;

    let res = sqlx::query(
        "UPDATE secret_connections SET name = ?, config = ?, credentials = ?, \
         last_tested_at = NULL, last_test_ok = NULL, last_test_message = NULL, \
         updated_at = datetime('now') WHERE id = ?",
    )
    .bind(&name)
    .bind(serde_json::to_string(&config).map_err(db_err)?)
    .bind(&sealed)
    .bind(&id)
    .execute(&state.db)
    .await;
    if let Err(e) = res {
        return Err(if e.to_string().contains("UNIQUE") {
            ApiError::conflict(format!("A connection named '{}' already exists", name))
        } else {
            db_err(e)
        });
    }

    audit_log(
        &state,
        "secret_connection.update",
        RESOURCE,
        Some(&id),
        Some(&name),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({
            "renamed_from": (name != row.name).then_some(row.name.clone()),
            "credentials_changed": req.credentials.is_some(),
        })),
    )
    .await;

    let row = load_connection(&state, &id).await?;
    Ok(Json(to_response(&row, key.as_ref())))
}

async fn delete_connection(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let row = load_connection(&state, &id).await?;
    require_manage(&state, &user, row.team_id.as_deref()).await?;
    sqlx::query("DELETE FROM secret_imports WHERE connection_id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(db_err)?;
    sqlx::query("DELETE FROM secret_connections WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(db_err)?;
    audit_log(
        &state,
        "secret_connection.delete",
        RESOURCE,
        Some(&id),
        Some(&row.name),
        Some(&user.id),
        client_ip.as_deref(),
        None,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn test_connection(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
) -> Result<Json<TestResult>, ApiError> {
    let row = load_connection(&state, &id).await?;
    require_read(&state, &user, row.team_id.as_deref()).await?;
    let key = encryption_key(&state);
    let result = match row.provider(key.as_ref(), DEFAULT_TIMEOUT) {
        Ok(p) => p.test().await,
        Err(e) => Err(e),
    };
    let (ok, message) = match result {
        Ok(m) => (true, m),
        Err(e) => (false, e.to_string()),
    };
    let _ = sqlx::query(
        "UPDATE secret_connections SET last_tested_at = datetime('now'), last_test_ok = ?, last_test_message = ? WHERE id = ?",
    )
    .bind(ok as i64)
    .bind(&message)
    .bind(&id)
    .execute(&state.db)
    .await;
    Ok(Json(TestResult { ok, message }))
}

/// Test settings from the add dialog before saving them.
async fn test_unsaved(
    State(_state): State<Arc<AppState>>,
    user: User,
    Json(req): Json<TestUnsavedRequest>,
) -> Result<Json<TestResult>, ApiError> {
    if !providers::PROVIDERS.contains(&req.provider.as_str()) {
        return Err(ApiError::bad_request("Unknown provider"));
    }
    check_outbound(&user, &req.provider, &req.config).await?;
    let result = match build_provider(
        &req.provider,
        &req.config,
        &req.credentials,
        DEFAULT_TIMEOUT,
    ) {
        Ok(p) => p.test().await,
        Err(e) => Err(e),
    };
    Ok(Json(match result {
        Ok(message) => TestResult { ok: true, message },
        Err(e) => TestResult {
            ok: false,
            message: e.to_string(),
        },
    }))
}

async fn browse_connection(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
    Query(q): Query<BrowseQuery>,
) -> Result<Json<Listing>, ApiError> {
    let row = load_connection(&state, &id).await?;
    require_read(&state, &user, row.team_id.as_deref()).await?;
    let provider = row
        .provider(encryption_key(&state).as_ref(), DEFAULT_TIMEOUT)
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    provider.list(&q.path).await.map(Json).map_err(|e| {
        ApiError::bad_request(format!("Could not browse '{}': {}", q.path, e))
            .with_status(StatusCode::BAD_GATEWAY)
    })
}

// ---------------------------------------------------------------------------
// Imports and policy (apps, services, projects)
// ---------------------------------------------------------------------------

/// The owner's team, after checking read (and, when `write`, change) access.
async fn authorize_owner(
    state: &Arc<AppState>,
    user: &User,
    kind: &str,
    id: &str,
    write: bool,
) -> Result<Option<String>, ApiError> {
    match kind {
        "app" => {
            let app = if write {
                authz::authorize_app_write(state, user, id).await?
            } else {
                authz::authorize_app(state, user, id).await?
            };
            Ok(app.team_id)
        }
        "service" => {
            let svc = if write {
                authz::authorize_service_write(state, user, id).await?
            } else {
                authz::authorize_service(state, user, id).await?
            };
            Ok(svc.team_id)
        }
        "project" => {
            let project = authz::authorize_project(state, user, id).await?;
            if write && !authz::can_write_team(state, user, project.team_id.as_deref()).await? {
                return Err(ApiError::forbidden(
                    "Changing project variables requires the developer role",
                ));
            }
            Ok(project.team_id)
        }
        _ => Err(ApiError::bad_request("Unknown owner type")),
    }
}

async fn list_imports(
    state: Arc<AppState>,
    user: User,
    kind: &str,
    owner_id: String,
) -> Result<Json<Vec<ImportResponse>>, ApiError> {
    authorize_owner(&state, &user, kind, &owner_id, false).await?;
    let rows = store::imports_for(&state.db, kind, &owner_id)
        .await
        .map_err(db_err)?;
    let mut out = Vec::with_capacity(rows.len());
    for import in rows {
        let conn = store::get_connection(&state.db, &import.connection_id)
            .await
            .map_err(db_err)?;
        out.push(ImportResponse {
            connection_name: conn.as_ref().map(|c| c.name.clone()),
            provider: conn.map(|c| c.provider),
            import,
        });
    }
    Ok(Json(out))
}

fn is_valid_prefix(p: &str) -> bool {
    p.is_empty() || crate::secret_managers::resolve::is_valid_env_key(p)
}

async fn create_import(
    state: Arc<AppState>,
    user: User,
    client_ip: ClientIp,
    kind: &str,
    owner_id: String,
    req: CreateImportRequest,
) -> Result<(StatusCode, Json<ImportResponse>), ApiError> {
    let owner_team = authorize_owner(&state, &user, kind, &owner_id, true).await?;
    let conn = load_connection(&state, &req.connection_id).await?;
    if !store::connection_usable_by(conn.team_id.as_deref(), owner_team.as_deref()) {
        return Err(ApiError::forbidden(
            "That connection belongs to another team",
        ));
    }
    let prefix = req.prefix.trim().to_string();
    if !is_valid_prefix(&prefix) {
        return Err(ApiError::validation_field(
            "prefix",
            "Use letters, digits and '_', not starting with a digit",
        ));
    }
    let path = req.path.trim().trim_matches('/').to_string();
    let id = Uuid::new_v4().to_string();
    let res = sqlx::query(
        "INSERT INTO secret_imports (id, owner_type, owner_id, connection_id, path, prefix) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(kind)
    .bind(&owner_id)
    .bind(&conn.id)
    .bind(&path)
    .bind(&prefix)
    .execute(&state.db)
    .await;
    if let Err(e) = res {
        return Err(if e.to_string().contains("UNIQUE") {
            ApiError::conflict("This path is already imported")
        } else {
            db_err(e)
        });
    }
    audit_log(
        &state,
        "secret_import.create",
        kind,
        Some(&owner_id),
        None,
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "connection": conn.name, "path": path, "prefix": prefix })),
    )
    .await;
    let import: SecretImportRow = sqlx::query_as("SELECT * FROM secret_imports WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(db_err)?;
    Ok((
        StatusCode::CREATED,
        Json(ImportResponse {
            import,
            connection_name: Some(conn.name),
            provider: Some(conn.provider),
        }),
    ))
}

async fn delete_import(
    state: Arc<AppState>,
    user: User,
    client_ip: ClientIp,
    kind: &str,
    owner_id: String,
    import_id: String,
) -> Result<StatusCode, ApiError> {
    authorize_owner(&state, &user, kind, &owner_id, true).await?;
    let res =
        sqlx::query("DELETE FROM secret_imports WHERE id = ? AND owner_type = ? AND owner_id = ?")
            .bind(&import_id)
            .bind(kind)
            .bind(&owner_id)
            .execute(&state.db)
            .await
            .map_err(db_err)?;
    if res.rows_affected() == 0 {
        return Err(ApiError::not_found("Import not found"));
    }
    audit_log(
        &state,
        "secret_import.delete",
        kind,
        Some(&owner_id),
        None,
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "import_id": import_id })),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_policy(
    state: Arc<AppState>,
    user: User,
    kind: &str,
    owner_id: String,
) -> Result<Json<PolicyBody>, ApiError> {
    authorize_owner(&state, &user, kind, &owner_id, false).await?;
    Ok(Json(PolicyBody {
        failure_policy: store::get_policy(&state.db, kind, &owner_id).await,
        last_resolved_at: store::last_resolved_at(&state.db, kind, &owner_id).await,
    }))
}

async fn set_policy(
    state: Arc<AppState>,
    user: User,
    client_ip: ClientIp,
    kind: &str,
    owner_id: String,
    body: PolicyBody,
) -> Result<Json<PolicyBody>, ApiError> {
    authorize_owner(&state, &user, kind, &owner_id, true).await?;
    let policy = body.failure_policy.as_str();
    if policy != POLICY_FAIL && policy != POLICY_LAST_KNOWN {
        return Err(ApiError::validation_field(
            "failure_policy",
            "Use 'fail' or 'last_known'",
        ));
    }
    store::set_policy(&state.db, kind, &owner_id, policy)
        .await
        .map_err(db_err)?;
    if policy == POLICY_FAIL {
        // Stored values only exist for the fallback; drop them when it is turned off.
        let _ = sqlx::query(
            "UPDATE secret_resolution_state SET last_values = NULL, last_resolved_at = NULL \
             WHERE owner_id = ? AND owner_type IN (?, ?)",
        )
        .bind(&owner_id)
        .bind(kind)
        .bind(format!("{}_build", kind))
        .execute(&state.db)
        .await;
    }
    audit_log(
        &state,
        "secret_policy.update",
        kind,
        Some(&owner_id),
        None,
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "failure_policy": policy })),
    )
    .await;
    get_policy(state, user, kind, owner_id).await
}

// Thin per-owner handlers (axum needs concrete functions per route).

async fn list_app_imports(
    State(s): State<Arc<AppState>>,
    u: User,
    Path(id): Path<String>,
) -> Result<Json<Vec<ImportResponse>>, ApiError> {
    list_imports(s, u, "app", id).await
}
async fn create_app_import(
    State(s): State<Arc<AppState>>,
    u: User,
    ip: ClientIp,
    Path(id): Path<String>,
    Json(r): Json<CreateImportRequest>,
) -> Result<(StatusCode, Json<ImportResponse>), ApiError> {
    create_import(s, u, ip, "app", id, r).await
}
async fn delete_app_import(
    State(s): State<Arc<AppState>>,
    u: User,
    ip: ClientIp,
    Path((id, import_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    delete_import(s, u, ip, "app", id, import_id).await
}
async fn get_app_policy(
    State(s): State<Arc<AppState>>,
    u: User,
    Path(id): Path<String>,
) -> Result<Json<PolicyBody>, ApiError> {
    get_policy(s, u, "app", id).await
}
async fn set_app_policy(
    State(s): State<Arc<AppState>>,
    u: User,
    ip: ClientIp,
    Path(id): Path<String>,
    Json(b): Json<PolicyBody>,
) -> Result<Json<PolicyBody>, ApiError> {
    set_policy(s, u, ip, "app", id, b).await
}
async fn list_service_imports(
    State(s): State<Arc<AppState>>,
    u: User,
    Path(id): Path<String>,
) -> Result<Json<Vec<ImportResponse>>, ApiError> {
    list_imports(s, u, "service", id).await
}
async fn create_service_import(
    State(s): State<Arc<AppState>>,
    u: User,
    ip: ClientIp,
    Path(id): Path<String>,
    Json(r): Json<CreateImportRequest>,
) -> Result<(StatusCode, Json<ImportResponse>), ApiError> {
    create_import(s, u, ip, "service", id, r).await
}
async fn delete_service_import(
    State(s): State<Arc<AppState>>,
    u: User,
    ip: ClientIp,
    Path((id, import_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    delete_import(s, u, ip, "service", id, import_id).await
}
async fn get_service_policy(
    State(s): State<Arc<AppState>>,
    u: User,
    Path(id): Path<String>,
) -> Result<Json<PolicyBody>, ApiError> {
    get_policy(s, u, "service", id).await
}
async fn set_service_policy(
    State(s): State<Arc<AppState>>,
    u: User,
    ip: ClientIp,
    Path(id): Path<String>,
    Json(b): Json<PolicyBody>,
) -> Result<Json<PolicyBody>, ApiError> {
    set_policy(s, u, ip, "service", id, b).await
}
async fn list_project_imports(
    State(s): State<Arc<AppState>>,
    u: User,
    Path(id): Path<String>,
) -> Result<Json<Vec<ImportResponse>>, ApiError> {
    list_imports(s, u, "project", id).await
}
async fn create_project_import(
    State(s): State<Arc<AppState>>,
    u: User,
    ip: ClientIp,
    Path(id): Path<String>,
    Json(r): Json<CreateImportRequest>,
) -> Result<(StatusCode, Json<ImportResponse>), ApiError> {
    create_import(s, u, ip, "project", id, r).await
}
async fn delete_project_import(
    State(s): State<Arc<AppState>>,
    u: User,
    ip: ClientIp,
    Path((id, import_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    delete_import(s, u, ip, "project", id, import_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_build_without_conflicts() {
        let _router: Router<Arc<AppState>> = routes();
    }

    #[test]
    fn prefixes() {
        assert!(is_valid_prefix(""));
        assert!(is_valid_prefix("APP_"));
        assert!(!is_valid_prefix("1A"));
        assert!(!is_valid_prefix("a-b"));
    }

    #[test]
    fn credentials_set_lists_names_only() {
        let c = ConnectionCredentials {
            token: Some("hvs.x".into()),
            secret_id: Some("".into()),
            ..Default::default()
        };
        assert_eq!(credentials_set(&c), vec!["token"]);
    }

    #[test]
    fn provider_and_name_validation() {
        assert!(validate("vault", "vault-prod").is_ok());
        assert!(validate("hashicorp", "x").is_err());
        assert!(validate("vault", "Vault Prod").is_err());
    }
}
