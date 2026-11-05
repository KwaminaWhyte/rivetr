//! Resource tags (E1): team-scoped labels attached to apps, databases and services.
//!
//! Routes:
//! - `GET/POST /api/tags`, `PUT/DELETE /api/tags/{id}`: tag CRUD
//! - `GET /api/tags/assignments`: which resources carry which tags (list filters)
//! - `GET /api/tags/{id}/resources`: resources carrying one tag
//! - `POST /api/tags/{id}/deploy`: deploy every app and service with the tag
//! - `PUT/POST /api/{apps|databases|services}/{id}/tags`, `DELETE .../tags/{tag_id}`
//!
//! `GET /api/apps/{id}/tags` already lists git tags for deploy-by-tag, so a
//! resource's tags are read through `/api/tags/assignments` instead.
//!
//! A tag with `team_id = NULL` is global and usable on resources without a team.
//! Team tags can only be attached to resources of the same team.

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::db::{actions, User};
use crate::AppState;

use super::audit::{audit_log, ClientIp};
use super::authz;
use super::error::ApiError;
use super::optional_json::OptionalJson;

const MAX_TAG_NAME: usize = 40;
const DEFAULT_COLOR: &str = "#6366f1";

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Tag {
    pub id: String,
    pub team_id: Option<String>,
    pub name: String,
    pub color: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct TagWithCount {
    #[serde(flatten)]
    pub tag: Tag,
    pub resource_count: i64,
}

/// Kind of resource a tag can be attached to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaggableKind {
    App,
    Database,
    Service,
}

impl TaggableKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Database => "database",
            Self::Service => "service",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "app" | "apps" => Some(Self::App),
            "database" | "databases" => Some(Self::Database),
            "service" | "services" => Some(Self::Service),
            _ => None,
        }
    }

    fn table(self) -> &'static str {
        match self {
            Self::App => "apps",
            Self::Database => "databases",
            Self::Service => "services",
        }
    }

    const ALL: [TaggableKind; 3] = [Self::App, Self::Database, Self::Service];
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TagAssignment {
    pub tag_id: String,
    pub resource_type: String,
    pub resource_id: String,
}

#[derive(Debug, Serialize)]
pub struct TaggedResource {
    pub resource_type: TaggableKind,
    pub id: String,
    pub name: String,
    pub status: Option<String>,
    pub project_id: Option<String>,
    pub team_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListTagsQuery {
    /// Team context. Missing: everything the caller can see. Empty: global tags only.
    pub team_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AssignmentsQuery {
    pub team_id: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    pub color: Option<String>,
    pub team_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTagRequest {
    pub name: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AttachTagRequest {
    pub tag_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SetTagsRequest {
    pub tag_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DeployResult {
    pub resource_type: TaggableKind,
    pub id: String,
    pub name: String,
    /// "queued" | "pending_approval" | "scheduled" | "skipped" | "failed"
    pub status: String,
    pub message: Option<String>,
    pub deployment_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeployByTagResponse {
    pub tag: Tag,
    pub results: Vec<DeployResult>,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Trim and validate a tag name: 1..=40 chars, letters, digits, space, `-_.:/`.
pub fn normalize_name(raw: &str) -> Result<String, String> {
    let name = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return Err("Tag name is required".into());
    }
    if name.chars().count() > MAX_TAG_NAME {
        return Err(format!(
            "Tag name must be at most {MAX_TAG_NAME} characters"
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | ':' | '/'))
    {
        return Err("Tag name may only contain letters, digits, spaces and - _ . : /".into());
    }
    Ok(name)
}

/// Accept `#rgb` or `#rrggbb` (case-insensitive) and return it lowercased.
pub fn normalize_color(raw: &str) -> Result<String, String> {
    let c = raw.trim().to_ascii_lowercase();
    let hex = c.strip_prefix('#').unwrap_or("");
    if (hex.len() == 3 || hex.len() == 6) && hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Ok(c)
    } else {
        Err("Color must be a hex value such as #6366f1".into())
    }
}

/// May a tag owned by `tag_team` be attached to a resource owned by `resource_team`?
pub fn tag_fits_resource(tag_team: Option<&str>, resource_team: Option<&str>) -> bool {
    match tag_team {
        None => true,
        Some(t) => resource_team == Some(t),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn non_empty(v: Option<&str>) -> Option<&str> {
    v.filter(|s| !s.is_empty())
}

async fn load_tag(state: &AppState, id: &str) -> Result<Tag, ApiError> {
    sqlx::query_as::<_, Tag>("SELECT * FROM tags WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Tag not found"))
}

/// Load a tag the caller may see.
async fn readable_tag(state: &Arc<AppState>, user: &User, id: &str) -> Result<Tag, ApiError> {
    let tag = load_tag(state, id).await?;
    if let Some(team) = tag.team_id.as_deref() {
        if !authz::is_privileged_user(user) && !authz::user_is_member(state, &user.id, team).await?
        {
            return Err(ApiError::not_found("Tag not found"));
        }
    }
    Ok(tag)
}

/// Load a tag the caller may change (developer role or higher in its team).
async fn writable_tag(state: &Arc<AppState>, user: &User, id: &str) -> Result<Tag, ApiError> {
    let tag = readable_tag(state, user, id).await?;
    if !authz::can_write_team(state, user, tag.team_id.as_deref()).await? {
        return Err(ApiError::forbidden(
            "Your team role is read-only; managing tags requires the developer role or higher",
        ));
    }
    Ok(tag)
}

/// Tags visible for a team filter (see `ListTagsQuery`).
async fn visible_tags(
    state: &Arc<AppState>,
    user: &User,
    team_id: Option<&str>,
) -> Result<Vec<Tag>, ApiError> {
    let all = match team_id {
        Some("") => {
            sqlx::query_as::<_, Tag>("SELECT * FROM tags WHERE team_id IS NULL ORDER BY name")
                .fetch_all(&state.db)
                .await?
        }
        Some(tid) => {
            authz::authorize_team_filter(state, user, Some(tid)).await?;
            sqlx::query_as::<_, Tag>(
                "SELECT * FROM tags WHERE team_id = ? OR team_id IS NULL ORDER BY name",
            )
            .bind(tid)
            .fetch_all(&state.db)
            .await?
        }
        None => {
            sqlx::query_as::<_, Tag>("SELECT * FROM tags ORDER BY name")
                .fetch_all(&state.db)
                .await?
        }
    };
    authz::retain_visible(state, user, all, |t| t.team_id.as_deref()).await
}

/// Owning team of a resource, or 404.
async fn resource_team(
    state: &AppState,
    kind: TaggableKind,
    id: &str,
) -> Result<Option<String>, ApiError> {
    let sql = format!("SELECT team_id FROM {} WHERE id = ?", kind.table());
    let row: Option<Option<String>> = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
    row.ok_or_else(|| ApiError::not_found("Resource not found"))
}

/// `(id, name, status, project_id, team_id)` of a tagged resource.
type TaggedRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// Resources of one kind carrying `tag_id`.
async fn tagged_of_kind(
    state: &AppState,
    kind: TaggableKind,
    tag_id: &str,
) -> Result<Vec<TaggedResource>, ApiError> {
    let sql = format!(
        "SELECT r.id, r.name, r.status, r.project_id, r.team_id FROM {} r \
         JOIN resource_tags rt ON rt.resource_id = r.id AND rt.resource_type = ? \
         WHERE rt.tag_id = ? ORDER BY r.name",
        kind.table()
    );
    let rows: Vec<TaggedRow> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .bind(kind.as_str())
        .bind(tag_id)
        .fetch_all(&state.db)
        .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name, status, project_id, team_id)| TaggedResource {
            resource_type: kind,
            id,
            name,
            status,
            project_id,
            team_id,
        })
        .collect())
}

/// Every resource with the tag that the caller may see.
async fn tagged_resources(
    state: &Arc<AppState>,
    user: &User,
    tag_id: &str,
) -> Result<Vec<TaggedResource>, ApiError> {
    let mut out = Vec::new();
    for kind in TaggableKind::ALL {
        out.extend(tagged_of_kind(state, kind, tag_id).await?);
    }
    authz::retain_visible(state, user, out, |r| r.team_id.as_deref()).await
}

fn map_unique(e: sqlx::Error) -> ApiError {
    if let sqlx::Error::Database(db) = &e {
        if db.is_unique_violation() {
            return ApiError::conflict("A tag with this name already exists");
        }
    }
    ApiError::from(e)
}

async fn audit(state: &AppState, user: &User, ip: &ClientIp, action: &str, tag: &Tag) {
    audit_log(
        state,
        action,
        "tag",
        Some(&tag.id),
        Some(&tag.name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;
}

// ---------------------------------------------------------------------------
// Tag CRUD
// ---------------------------------------------------------------------------

/// GET /api/tags?team_id=
pub async fn list_tags(
    State(state): State<Arc<AppState>>,
    user: User,
    Query(q): Query<ListTagsQuery>,
) -> Result<Json<Vec<TagWithCount>>, ApiError> {
    let tags = visible_tags(&state, &user, q.team_id.as_deref()).await?;
    let counts: Vec<(String, i64)> =
        sqlx::query_as("SELECT tag_id, COUNT(*) FROM resource_tags GROUP BY tag_id")
            .fetch_all(&state.db)
            .await?;
    let counts: HashMap<String, i64> = counts.into_iter().collect();
    Ok(Json(
        tags.into_iter()
            .map(|tag| {
                let resource_count = counts.get(&tag.id).copied().unwrap_or(0);
                TagWithCount {
                    tag,
                    resource_count,
                }
            })
            .collect(),
    ))
}

/// POST /api/tags
pub async fn create_tag(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Json(req): Json<CreateTagRequest>,
) -> Result<(StatusCode, Json<Tag>), ApiError> {
    let name = normalize_name(&req.name).map_err(|m| ApiError::validation_field("name", m))?;
    let color = match req.color.as_deref() {
        Some(c) => normalize_color(c).map_err(|m| ApiError::validation_field("color", m))?,
        None => DEFAULT_COLOR.to_string(),
    };
    let team_id = non_empty(req.team_id.as_deref()).map(str::to_string);
    authz::authorize_team_assignment(&state, &user, team_id.as_deref()).await?;
    if !authz::can_write_team(&state, &user, team_id.as_deref()).await? {
        return Err(ApiError::forbidden(
            "Your team role is read-only; managing tags requires the developer role or higher",
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO tags (id, team_id, name, color, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&team_id)
    .bind(&name)
    .bind(&color)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(map_unique)?;

    let tag = load_tag(&state, &id).await?;
    audit(&state, &user, &client_ip, actions::TAG_CREATE, &tag).await;
    Ok((StatusCode::CREATED, Json(tag)))
}

/// PUT /api/tags/{id}
pub async fn update_tag(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(id): Path<String>,
    Json(req): Json<UpdateTagRequest>,
) -> Result<Json<Tag>, ApiError> {
    let mut tag = writable_tag(&state, &user, &id).await?;
    if let Some(name) = req.name.as_deref() {
        tag.name = normalize_name(name).map_err(|m| ApiError::validation_field("name", m))?;
    }
    if let Some(color) = req.color.as_deref() {
        tag.color = normalize_color(color).map_err(|m| ApiError::validation_field("color", m))?;
    }
    sqlx::query("UPDATE tags SET name = ?, color = ?, updated_at = ? WHERE id = ?")
        .bind(&tag.name)
        .bind(&tag.color)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(map_unique)?;
    let tag = load_tag(&state, &id).await?;
    audit(&state, &user, &client_ip, actions::TAG_UPDATE, &tag).await;
    Ok(Json(tag))
}

/// DELETE /api/tags/{id}
pub async fn delete_tag(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let tag = writable_tag(&state, &user, &id).await?;
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM resource_tags WHERE tag_id = ?")
        .bind(&id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM tags WHERE id = ?")
        .bind(&id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    audit(&state, &user, &client_ip, actions::TAG_DELETE, &tag).await;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/tags/{id}/resources
pub async fn list_tag_resources(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
) -> Result<Json<Vec<TaggedResource>>, ApiError> {
    readable_tag(&state, &user, &id).await?;
    Ok(Json(tagged_resources(&state, &user, &id).await?))
}

/// GET /api/tags/assignments?team_id=&resource_type=&resource_id=
///
/// Tag assignments on resources that still exist, limited to tags the caller can
/// see. Used by list pages and the canvas to filter by tag.
pub async fn list_assignments(
    State(state): State<Arc<AppState>>,
    user: User,
    Query(q): Query<AssignmentsQuery>,
) -> Result<Json<Vec<TagAssignment>>, ApiError> {
    let tags = visible_tags(&state, &user, q.team_id.as_deref()).await?;
    let visible: std::collections::HashSet<String> = tags.into_iter().map(|t| t.id).collect();
    let kind = match q.resource_type.as_deref() {
        Some(raw) => Some(TaggableKind::parse(raw).ok_or_else(|| {
            ApiError::validation_field("resource_type", "Use app, database or service")
        })?),
        None => None,
    };

    let rows: Vec<TagAssignment> = sqlx::query_as(
        "SELECT rt.tag_id, rt.resource_type, rt.resource_id FROM resource_tags rt \
         WHERE ((rt.resource_type = 'app' AND rt.resource_id IN (SELECT id FROM apps)) \
             OR (rt.resource_type = 'database' AND rt.resource_id IN (SELECT id FROM databases)) \
             OR (rt.resource_type = 'service' AND rt.resource_id IN (SELECT id FROM services))) \
           AND (? IS NULL OR rt.resource_type = ?) \
           AND (? IS NULL OR rt.resource_id = ?) \
         ORDER BY rt.created_at",
    )
    .bind(kind.map(TaggableKind::as_str))
    .bind(kind.map(TaggableKind::as_str))
    .bind(q.resource_id.as_deref())
    .bind(q.resource_id.as_deref())
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        rows.into_iter()
            .filter(|r| visible.contains(&r.tag_id))
            .collect(),
    ))
}

// ---------------------------------------------------------------------------
// Attach / detach (resource access is checked by `resource_authz_middleware`)
// ---------------------------------------------------------------------------

async fn attach(
    state: &Arc<AppState>,
    user: &User,
    kind: TaggableKind,
    resource_id: &str,
    tag_id: &str,
) -> Result<(), ApiError> {
    let team = resource_team(state, kind, resource_id).await?;
    let tag = readable_tag(state, user, tag_id).await?;
    if !tag_fits_resource(tag.team_id.as_deref(), team.as_deref()) {
        return Err(ApiError::bad_request(format!(
            "Tag '{}' belongs to another team",
            tag.name
        )));
    }
    sqlx::query(
        "INSERT OR IGNORE INTO resource_tags (tag_id, resource_type, resource_id) VALUES (?, ?, ?)",
    )
    .bind(&tag.id)
    .bind(kind.as_str())
    .bind(resource_id)
    .execute(&state.db)
    .await?;
    Ok(())
}

async fn tags_of(
    state: &AppState,
    kind: TaggableKind,
    resource_id: &str,
) -> Result<Vec<Tag>, ApiError> {
    Ok(sqlx::query_as::<_, Tag>(
        "SELECT t.* FROM tags t JOIN resource_tags rt ON rt.tag_id = t.id \
         WHERE rt.resource_type = ? AND rt.resource_id = ? ORDER BY t.name",
    )
    .bind(kind.as_str())
    .bind(resource_id)
    .fetch_all(&state.db)
    .await?)
}

/// POST body `{ "tag_id": "..." }`: attach one tag.
async fn attach_tag(
    state: Arc<AppState>,
    user: User,
    kind: TaggableKind,
    id: String,
    req: AttachTagRequest,
) -> Result<Json<Vec<Tag>>, ApiError> {
    attach(&state, &user, kind, &id, &req.tag_id).await?;
    Ok(Json(tags_of(&state, kind, &id).await?))
}

/// PUT body `{ "tag_ids": [...] }`: replace the resource's tags with exactly this set.
async fn set_tags(
    state: Arc<AppState>,
    user: User,
    kind: TaggableKind,
    id: String,
    req: SetTagsRequest,
) -> Result<Json<Vec<Tag>>, ApiError> {
    // Validate every tag first so a bad id leaves the old set untouched.
    let team = resource_team(&state, kind, &id).await?;
    for tag_id in &req.tag_ids {
        let tag = readable_tag(&state, &user, tag_id).await?;
        if !tag_fits_resource(tag.team_id.as_deref(), team.as_deref()) {
            return Err(ApiError::bad_request(format!(
                "Tag '{}' belongs to another team",
                tag.name
            )));
        }
    }
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM resource_tags WHERE resource_type = ? AND resource_id = ?")
        .bind(kind.as_str())
        .bind(&id)
        .execute(&mut *tx)
        .await?;
    for tag_id in &req.tag_ids {
        sqlx::query(
            "INSERT OR IGNORE INTO resource_tags (tag_id, resource_type, resource_id) VALUES (?, ?, ?)",
        )
        .bind(tag_id)
        .bind(kind.as_str())
        .bind(&id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(Json(tags_of(&state, kind, &id).await?))
}

/// DELETE: detach one tag.
async fn detach_tag(
    state: Arc<AppState>,
    kind: TaggableKind,
    id: String,
    tag_id: String,
) -> Result<Json<Vec<Tag>>, ApiError> {
    resource_team(&state, kind, &id).await?;
    sqlx::query(
        "DELETE FROM resource_tags WHERE tag_id = ? AND resource_type = ? AND resource_id = ?",
    )
    .bind(&tag_id)
    .bind(kind.as_str())
    .bind(&id)
    .execute(&state.db)
    .await?;
    Ok(Json(tags_of(&state, kind, &id).await?))
}

/// Per-kind handlers for `/api/{apps|databases|services}/{id}/tags[/{tag_id}]`.
macro_rules! resource_tag_handlers {
    ($kind:expr, $attach:ident, $set:ident, $detach:ident) => {
        pub async fn $attach(
            State(state): State<Arc<AppState>>,
            user: User,
            Path(id): Path<String>,
            Json(req): Json<AttachTagRequest>,
        ) -> Result<Json<Vec<Tag>>, ApiError> {
            attach_tag(state, user, $kind, id, req).await
        }

        pub async fn $set(
            State(state): State<Arc<AppState>>,
            user: User,
            Path(id): Path<String>,
            Json(req): Json<SetTagsRequest>,
        ) -> Result<Json<Vec<Tag>>, ApiError> {
            set_tags(state, user, $kind, id, req).await
        }

        pub async fn $detach(
            State(state): State<Arc<AppState>>,
            Path((id, tag_id)): Path<(String, String)>,
        ) -> Result<Json<Vec<Tag>>, ApiError> {
            detach_tag(state, $kind, id, tag_id).await
        }
    };
}

resource_tag_handlers!(
    TaggableKind::App,
    attach_app_tag,
    set_app_tags,
    detach_app_tag
);
resource_tag_handlers!(
    TaggableKind::Database,
    attach_database_tag,
    set_database_tags,
    detach_database_tag
);
resource_tag_handlers!(
    TaggableKind::Service,
    attach_service_tag,
    set_service_tags,
    detach_service_tag
);

// ---------------------------------------------------------------------------
// Deploy everything with a tag
// ---------------------------------------------------------------------------

fn failed(r: &TaggedResource, message: impl Into<String>) -> DeployResult {
    DeployResult {
        resource_type: r.resource_type,
        id: r.id.clone(),
        name: r.name.clone(),
        status: "failed".into(),
        message: Some(message.into()),
        deployment_id: None,
    }
}

/// POST /api/tags/{id}/deploy
///
/// Deploys every app and service carrying the tag through the same code paths
/// as their own Deploy buttons, so team roles, approvals, freeze windows and
/// in-progress checks all apply per resource. Databases are not redeployed.
pub async fn deploy_by_tag(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(id): Path<String>,
) -> Result<Json<DeployByTagResponse>, ApiError> {
    let tag = readable_tag(&state, &user, &id).await?;
    let resources = tagged_resources(&state, &user, &id).await?;
    let mut results = Vec::with_capacity(resources.len());

    for r in &resources {
        match r.resource_type {
            TaggableKind::App => {
                if let Err(e) = authz::authorize_app_deploy(&state, &user, &r.id).await {
                    results.push(failed(r, e.message()));
                    continue;
                }
                match super::deployments::trigger_deploy(
                    State(state.clone()),
                    user.clone(),
                    ClientIp(client_ip.0.clone()),
                    Path(r.id.clone()),
                    OptionalJson(None),
                )
                .await
                {
                    Ok((_, Json(dep))) => {
                        let status = if dep.approval_status.as_deref() == Some("pending") {
                            "pending_approval"
                        } else if dep.scheduled_at.is_some() {
                            "scheduled"
                        } else {
                            "queued"
                        };
                        results.push(DeployResult {
                            resource_type: r.resource_type,
                            id: r.id.clone(),
                            name: r.name.clone(),
                            status: status.into(),
                            message: None,
                            deployment_id: Some(dep.id),
                        });
                    }
                    Err(e) => results.push(failed(r, e.message())),
                }
            }
            TaggableKind::Service => {
                if let Err(e) = authz::authorize_service_deploy(&state, &user, &r.id).await {
                    results.push(failed(r, e.message()));
                    continue;
                }
                match super::services::deploy_service(
                    State(state.clone()),
                    user.clone(),
                    ClientIp(client_ip.0.clone()),
                    Path(r.id.clone()),
                )
                .await
                {
                    Ok((_, Json(dep))) => results.push(DeployResult {
                        resource_type: r.resource_type,
                        id: r.id.clone(),
                        name: r.name.clone(),
                        status: "queued".into(),
                        message: None,
                        deployment_id: Some(dep.id),
                    }),
                    Err(e) => results.push(failed(r, e.message())),
                }
            }
            TaggableKind::Database => results.push(DeployResult {
                resource_type: r.resource_type,
                id: r.id.clone(),
                name: r.name.clone(),
                status: "skipped".into(),
                message: Some("Databases are not redeployed".into()),
                deployment_id: None,
            }),
        }
    }

    audit_log(
        &state,
        actions::TAG_DEPLOY,
        "tag",
        Some(&tag.id),
        Some(&tag.name),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({
            "resources": results.len(),
            "queued": results.iter().filter(|r| r.status == "queued").count(),
        })),
    )
    .await;

    Ok(Json(DeployByTagResponse { tag, results }))
}

// ---------------------------------------------------------------------------
// Shared lookups (MCP)
// ---------------------------------------------------------------------------

/// Map of `resource_id -> tag names` for one resource kind (MCP list tools).
pub async fn tag_names_by_resource(
    db: &sqlx::SqlitePool,
    kind: TaggableKind,
) -> Result<HashMap<String, Vec<String>>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT rt.resource_id, t.name FROM resource_tags rt JOIN tags t ON t.id = rt.tag_id \
         WHERE rt.resource_type = ? ORDER BY t.name",
    )
    .bind(kind.as_str())
    .fetch_all(db)
    .await?;
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for (rid, name) in rows {
        map.entry(rid).or_default().push(name);
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_trimmed_and_validated() {
        assert_eq!(normalize_name("  prod   eu ").unwrap(), "prod eu");
        assert_eq!(
            normalize_name("env:prod/v1.2_x-y").unwrap(),
            "env:prod/v1.2_x-y"
        );
        assert!(normalize_name("   ").is_err());
        assert!(normalize_name("bad<tag>").is_err());
        assert!(normalize_name(&"a".repeat(41)).is_err());
    }

    #[test]
    fn colors_must_be_hex() {
        assert_eq!(normalize_color("#ABC").unwrap(), "#abc");
        assert_eq!(normalize_color(" #6366F1 ").unwrap(), "#6366f1");
        assert!(normalize_color("6366f1").is_err());
        assert!(normalize_color("#12345").is_err());
        assert!(normalize_color("#gggggg").is_err());
    }

    #[test]
    fn team_tags_only_fit_same_team_resources() {
        assert!(tag_fits_resource(None, None));
        assert!(tag_fits_resource(None, Some("t1")));
        assert!(tag_fits_resource(Some("t1"), Some("t1")));
        assert!(!tag_fits_resource(Some("t1"), Some("t2")));
        assert!(!tag_fits_resource(Some("t1"), None));
    }

    #[tokio::test]
    async fn migration_creates_tables_and_sweeps_orphans() {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::db::init(dir.path()).await.unwrap();
        sqlx::query("INSERT INTO tags (id, name) VALUES ('t1', 'prod')")
            .execute(&pool)
            .await
            .unwrap();
        // Same name, different case, same (global) team is rejected.
        assert!(
            sqlx::query("INSERT INTO tags (id, name) VALUES ('t2', 'PROD')")
                .execute(&pool)
                .await
                .is_err()
        );
        sqlx::query(
            "INSERT INTO resource_tags (tag_id, resource_type, resource_id) VALUES ('t1', 'app', 'gone')",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;

        let pool = crate::db::init(dir.path()).await.unwrap();
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM resource_tags")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n, 0, "orphaned assignment is swept on start");
        let tags = tag_names_by_resource(&pool, TaggableKind::App)
            .await
            .unwrap();
        assert!(tags.is_empty());
    }
}
