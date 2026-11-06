//! Community template submissions.
//!
//! Any signed-in user can submit a Docker Compose template. Instance admins
//! review the queue: approve (optionally editing first) or reject with a
//! reason. Approving copies the submission into `service_templates` with
//! `is_community = 1`, so it shows in the catalogue (marked "Community") and
//! deploys exactly like a built-in template.
//!
//! States: `pending` -> `approved` | `rejected`. Only pending submissions can
//! be edited by an admin or reviewed. A submitter can withdraw (delete) a
//! pending or rejected submission.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::{EnvSchemaEntry, TemplateCategory, User};
use crate::AppState;

use super::audit::{audit_log, ClientIp};
use super::error::ApiError;
use super::services::compose::validate_compose_content;

const MAX_COMPOSE_BYTES: usize = 64 * 1024;
const MAX_ENV_HINTS: usize = 50;

/// A row of `community_template_submissions` (plus the submitter's email).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TemplateSubmission {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub icon: Option<String>,
    pub compose_content: String,
    /// JSON array of env var hints (same shape as a template's env_schema)
    #[sqlx(default)]
    pub env_schema: Option<String>,
    pub submitted_by: String,
    #[sqlx(default)]
    pub submitter_email: Option<String>,
    /// pending | approved | rejected
    pub status: String,
    pub admin_notes: Option<String>,
    #[sqlx(default)]
    pub rejection_reason: Option<String>,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<String>,
    #[sqlx(default)]
    pub approved_template_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// API shape: env hints parsed.
#[derive(Debug, Clone, Serialize)]
pub struct TemplateSubmissionResponse {
    #[serde(flatten)]
    pub submission: TemplateSubmission,
    pub env_hints: Vec<EnvSchemaEntry>,
}

impl From<TemplateSubmission> for TemplateSubmissionResponse {
    fn from(mut s: TemplateSubmission) -> Self {
        let env_hints = s
            .env_schema
            .as_deref()
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_default();
        s.env_schema = None;
        Self {
            submission: s,
            env_hints,
        }
    }
}

/// Body for submitting, editing or approving-with-edits.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SubmissionInput {
    pub name: String,
    pub description: String,
    pub category: String,
    #[serde(default)]
    pub icon: Option<String>,
    pub compose_content: String,
    #[serde(default)]
    pub env_hints: Vec<EnvSchemaEntry>,
}

/// Optional edits applied when approving.
#[derive(Debug, Default, Deserialize)]
pub struct ApproveRequest {
    #[serde(default)]
    pub edits: Option<SubmissionInput>,
    #[serde(default)]
    pub admin_notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RejectRequest {
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
}

/// Normalised, validated input.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidSubmission {
    pub name: String,
    pub description: String,
    pub category: String,
    pub icon: Option<String>,
    pub compose_content: String,
    pub env_schema: String,
}

fn valid_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Validate a submission. Uses the same compose check as compose services
/// (`validate_compose_content`), plus size and field limits.
pub fn validate_submission(input: &SubmissionInput) -> Result<ValidSubmission, ApiError> {
    let name = input.name.trim();
    if name.len() < 2 || name.len() > 64 {
        return Err(ApiError::validation_field(
            "name",
            "Name must be 2 to 64 characters",
        ));
    }
    if name.chars().any(|c| c.is_control()) {
        return Err(ApiError::validation_field(
            "name",
            "Name has invalid characters",
        ));
    }
    let description = input.description.trim();
    if description.is_empty() || description.len() > 1000 {
        return Err(ApiError::validation_field(
            "description",
            "Description must be 1 to 1000 characters",
        ));
    }
    let category = input
        .category
        .parse::<TemplateCategory>()
        .map_err(|e| ApiError::validation_field("category", e))?
        .to_string();
    let icon = match input
        .icon
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        None => None,
        Some(url) => {
            let ok = url.len() <= 500
                && (url.starts_with("https://") || url.starts_with("http://"))
                && !url
                    .chars()
                    .any(|c| c.is_whitespace() || c == '"' || c == '<' || c == '>');
            if !ok {
                return Err(ApiError::validation_field(
                    "icon",
                    "Icon must be an http(s) URL of at most 500 characters",
                ));
            }
            Some(url.to_string())
        }
    };
    if input.compose_content.len() > MAX_COMPOSE_BYTES {
        return Err(ApiError::validation_field(
            "compose_content",
            "Compose file is larger than 64 KB",
        ));
    }
    validate_compose_content(&input.compose_content)
        .map_err(|e| ApiError::validation_field("compose_content", e))?;
    if input.env_hints.len() > MAX_ENV_HINTS {
        return Err(ApiError::validation_field(
            "env_hints",
            "At most 50 environment variable hints",
        ));
    }
    let mut hints = Vec::with_capacity(input.env_hints.len());
    let mut seen = std::collections::HashSet::new();
    for h in &input.env_hints {
        let n = h.name.trim();
        if !valid_env_name(n) {
            return Err(ApiError::validation_field(
                "env_hints",
                format!("'{}' is not a valid environment variable name", n),
            ));
        }
        if !seen.insert(n.to_string()) {
            return Err(ApiError::validation_field(
                "env_hints",
                format!("'{}' is listed twice", n),
            ));
        }
        let label = if h.label.trim().is_empty() {
            n.to_string()
        } else {
            h.label.trim().to_string()
        };
        hints.push(EnvSchemaEntry {
            name: n.to_string(),
            label,
            required: h.required,
            default: h.default.clone(),
            secret: h.secret,
        });
    }
    Ok(ValidSubmission {
        name: name.to_string(),
        description: description.to_string(),
        category,
        icon,
        compose_content: input.compose_content.clone(),
        env_schema: serde_json::to_string(&hints).unwrap_or_else(|_| "[]".into()),
    })
}

/// Review actions on a submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewAction {
    Edit,
    Approve,
    Reject,
    Withdraw,
}

/// The state a submission moves to, or why it cannot.
pub fn transition(current: &str, action: ReviewAction) -> Result<&'static str, String> {
    match (current, action) {
        ("pending", ReviewAction::Edit) => Ok("pending"),
        ("pending", ReviewAction::Approve) => Ok("approved"),
        ("pending", ReviewAction::Reject) => Ok("rejected"),
        ("pending" | "rejected", ReviewAction::Withdraw) => Ok("withdrawn"),
        (s, a) => Err(format!(
            "A {} submission cannot be {}",
            s,
            match a {
                ReviewAction::Edit => "edited",
                ReviewAction::Approve => "approved",
                ReviewAction::Reject => "rejected",
                ReviewAction::Withdraw => "withdrawn",
            }
        )),
    }
}

fn is_admin(user: &User) -> bool {
    user.role == "admin"
}

fn require_admin(user: &User) -> Result<(), ApiError> {
    if is_admin(user) {
        Ok(())
    } else {
        Err(ApiError::forbidden(
            "Only instance admins can review templates",
        ))
    }
}

const SELECT: &str = "SELECT s.*, u.email AS submitter_email \
     FROM community_template_submissions s LEFT JOIN users u ON u.id = s.submitted_by";

async fn load(state: &AppState, id: &str) -> Result<TemplateSubmission, ApiError> {
    sqlx::query_as::<_, TemplateSubmission>(sqlx::AssertSqlSafe(format!("{SELECT} WHERE s.id = ?")))
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Submission not found"))
}

async fn name_taken(state: &AppState, name: &str) -> Result<bool, ApiError> {
    let n: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM service_templates WHERE LOWER(name) = LOWER(?)")
            .bind(name)
            .fetch_one(&state.db)
            .await?;
    Ok(n > 0)
}

/// POST /api/templates/submissions
pub async fn create_submission(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Json(input): Json<SubmissionInput>,
) -> Result<(StatusCode, Json<TemplateSubmissionResponse>), ApiError> {
    let v = validate_submission(&input)?;
    if name_taken(&state, &v.name).await? {
        return Err(ApiError::validation_field(
            "name",
            "A template with this name already exists",
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO community_template_submissions \
         (id, name, description, category, icon, compose_content, env_schema, submitted_by, status, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?)",
    )
    .bind(&id)
    .bind(&v.name)
    .bind(&v.description)
    .bind(&v.category)
    .bind(&v.icon)
    .bind(&v.compose_content)
    .bind(&v.env_schema)
    .bind(&user.id)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;

    // No notification event fits an instance-level review queue (channels
    // subscribe to app events), so admins see new submissions as a pending
    // count on /templates and /settings/templates, and in the audit log.
    audit_log(
        &state,
        "template_submission.create",
        "template_submission",
        Some(&id),
        Some(&v.name),
        Some(&user.id),
        client_ip.as_deref(),
        None,
    )
    .await;

    Ok((StatusCode::CREATED, Json(load(&state, &id).await?.into())))
}

/// GET /api/templates/submissions (admins: the review queue, `?status=`)
pub async fn list_submissions(
    State(state): State<Arc<AppState>>,
    user: User,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<TemplateSubmissionResponse>>, ApiError> {
    require_admin(&user)?;
    let rows = sqlx::query_as::<_, TemplateSubmission>(sqlx::AssertSqlSafe(format!(
        "{SELECT} WHERE (? IS NULL OR s.status = ?) ORDER BY s.created_at DESC"
    )))
    .bind(&q.status)
    .bind(&q.status)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

/// GET /api/templates/submissions/mine
pub async fn list_my_submissions(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<Vec<TemplateSubmissionResponse>>, ApiError> {
    let rows = sqlx::query_as::<_, TemplateSubmission>(sqlx::AssertSqlSafe(format!(
        "{SELECT} WHERE s.submitted_by = ? ORDER BY s.created_at DESC"
    )))
    .bind(&user.id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

/// GET /api/templates/submissions/pending-count (admins; 0 for others)
pub async fn pending_count(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<serde_json::Value>, ApiError> {
    let count: i64 = if is_admin(&user) {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM community_template_submissions WHERE status = 'pending'",
        )
        .fetch_one(&state.db)
        .await?
    } else {
        0
    };
    Ok(Json(serde_json::json!({ "count": count })))
}

/// GET /api/templates/submissions/{id} (admin or submitter)
pub async fn get_submission(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
) -> Result<Json<TemplateSubmissionResponse>, ApiError> {
    let s = load(&state, &id).await?;
    if !is_admin(&user) && s.submitted_by != user.id {
        return Err(ApiError::not_found("Submission not found"));
    }
    Ok(Json(s.into()))
}

async fn write_fields(state: &AppState, id: &str, v: &ValidSubmission) -> Result<(), ApiError> {
    sqlx::query(
        "UPDATE community_template_submissions SET name = ?, description = ?, category = ?, \
         icon = ?, compose_content = ?, env_schema = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&v.name)
    .bind(&v.description)
    .bind(&v.category)
    .bind(&v.icon)
    .bind(&v.compose_content)
    .bind(&v.env_schema)
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(id)
    .execute(&state.db)
    .await?;
    Ok(())
}

/// PUT /api/templates/submissions/{id} (admin edit before approving)
pub async fn update_submission(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
    Json(input): Json<SubmissionInput>,
) -> Result<Json<TemplateSubmissionResponse>, ApiError> {
    require_admin(&user)?;
    let s = load(&state, &id).await?;
    transition(&s.status, ReviewAction::Edit).map_err(ApiError::conflict)?;
    let v = validate_submission(&input)?;
    write_fields(&state, &id, &v).await?;
    Ok(Json(load(&state, &id).await?.into()))
}

/// POST /api/templates/submissions/{id}/approve
pub async fn approve_submission(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(id): Path<String>,
    body: Option<Json<ApproveRequest>>,
) -> Result<Json<TemplateSubmissionResponse>, ApiError> {
    require_admin(&user)?;
    let req = body.map(|Json(b)| b).unwrap_or_default();
    let s = load(&state, &id).await?;
    let next = transition(&s.status, ReviewAction::Approve).map_err(ApiError::conflict)?;

    // Apply admin edits first, then validate what will be published.
    let v = match &req.edits {
        Some(edits) => {
            let v = validate_submission(edits)?;
            write_fields(&state, &id, &v).await?;
            v
        }
        None => validate_submission(&SubmissionInput {
            name: s.name.clone(),
            description: s.description.clone(),
            category: s.category.clone(),
            icon: s.icon.clone(),
            compose_content: s.compose_content.clone(),
            env_hints: TemplateSubmissionResponse::from(s.clone()).env_hints,
        })?,
    };
    if name_taken(&state, &v.name).await? {
        return Err(ApiError::conflict(
            "A template with this name already exists; edit the name before approving",
        ));
    }

    let template_id = format!("community-{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().to_rfc3339();
    let mut tx = state.db.begin().await?;
    sqlx::query(
        "INSERT INTO service_templates \
         (id, name, description, category, icon, compose_template, env_schema, is_builtin, is_community, submission_id, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, 0, 1, ?, ?)",
    )
    .bind(&template_id)
    .bind(&v.name)
    .bind(&v.description)
    .bind(&v.category)
    .bind(&v.icon)
    .bind(&v.compose_content)
    .bind(&v.env_schema)
    .bind(&id)
    .bind(&now)
    .execute(&mut *tx)
    .await?;
    let updated = sqlx::query(
        "UPDATE community_template_submissions SET status = ?, reviewed_by = ?, reviewed_at = ?, \
         approved_template_id = ?, rejection_reason = NULL, \
         admin_notes = COALESCE(?, admin_notes), updated_at = ? WHERE id = ? AND status = 'pending'",
    )
    .bind(next)
    .bind(&user.id)
    .bind(&now)
    .bind(&template_id)
    .bind(&req.admin_notes)
    .bind(&now)
    .bind(&id)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(ApiError::conflict(
            "Submission was reviewed by someone else",
        ));
    }
    tx.commit().await?;

    audit_log(
        &state,
        "template_submission.approve",
        "template_submission",
        Some(&id),
        Some(&v.name),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "template_id": template_id })),
    )
    .await;
    Ok(Json(load(&state, &id).await?.into()))
}

/// POST /api/templates/submissions/{id}/reject
pub async fn reject_submission(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(id): Path<String>,
    Json(req): Json<RejectRequest>,
) -> Result<Json<TemplateSubmissionResponse>, ApiError> {
    require_admin(&user)?;
    let reason = req.reason.trim();
    if reason.is_empty() || reason.len() > 1000 {
        return Err(ApiError::validation_field(
            "reason",
            "Give a reason (up to 1000 characters)",
        ));
    }
    let s = load(&state, &id).await?;
    let next = transition(&s.status, ReviewAction::Reject).map_err(ApiError::conflict)?;
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE community_template_submissions SET status = ?, rejection_reason = ?, \
         reviewed_by = ?, reviewed_at = ?, updated_at = ? WHERE id = ? AND status = 'pending'",
    )
    .bind(next)
    .bind(reason)
    .bind(&user.id)
    .bind(&now)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await?;
    audit_log(
        &state,
        "template_submission.reject",
        "template_submission",
        Some(&id),
        Some(&s.name),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "reason": reason })),
    )
    .await;
    Ok(Json(load(&state, &id).await?.into()))
}

/// DELETE /api/templates/submissions/{id}
/// Submitters withdraw their pending or rejected submissions; admins can
/// delete any non-approved one. An approved template stays in the catalogue.
pub async fn delete_submission(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let s = load(&state, &id).await?;
    if !is_admin(&user) && s.submitted_by != user.id {
        return Err(ApiError::not_found("Submission not found"));
    }
    transition(&s.status, ReviewAction::Withdraw).map_err(ApiError::conflict)?;
    sqlx::query("DELETE FROM community_template_submissions WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPOSE: &str = "services:\n  web:\n    image: nginx:1.27\n";

    fn input() -> SubmissionInput {
        SubmissionInput {
            name: "  My App ".into(),
            description: "A thing".into(),
            category: "Development".into(),
            icon: Some("https://example.com/i.png".into()),
            compose_content: COMPOSE.into(),
            env_hints: vec![EnvSchemaEntry {
                name: "API_KEY".into(),
                label: String::new(),
                required: true,
                default: String::new(),
                secret: true,
            }],
        }
    }

    #[test]
    fn valid_submission_is_normalised() {
        let v = validate_submission(&input()).unwrap();
        assert_eq!(v.name, "My App");
        assert_eq!(v.category, "development");
        let hints: Vec<EnvSchemaEntry> = serde_json::from_str(&v.env_schema).unwrap();
        assert_eq!(hints[0].label, "API_KEY");
        assert!(hints[0].secret);
    }

    #[test]
    fn invalid_submissions_are_rejected() {
        type Mutator = Box<dyn Fn(&mut SubmissionInput)>;
        let cases: Vec<(&str, Mutator)> = vec![
            ("name", Box::new(|i| i.name = "x".into())),
            ("description", Box::new(|i| i.description = " ".into())),
            ("category", Box::new(|i| i.category = "games".into())),
            (
                "icon",
                Box::new(|i| i.icon = Some("javascript:alert(1)".into())),
            ),
            (
                "compose_content",
                Box::new(|i| i.compose_content = "".into()),
            ),
            (
                "compose_content",
                Box::new(|i| i.compose_content = "version: '3'\n".into()),
            ),
            (
                "compose_content",
                Box::new(|i| i.compose_content = "services: [".into()),
            ),
            (
                "compose_content",
                Box::new(|i| {
                    i.compose_content = format!("services: {{}}\n#{}", "x".repeat(70_000))
                }),
            ),
            (
                "env_hints",
                Box::new(|i| i.env_hints[0].name = "1BAD".into()),
            ),
            (
                "env_hints",
                Box::new(|i| {
                    let dup = i.env_hints[0].clone();
                    i.env_hints.push(dup)
                }),
            ),
        ];
        for (field, mutate) in cases {
            let mut i = input();
            mutate(&mut i);
            let err = validate_submission(&i).expect_err(field);
            let body = format!("{:?}", err);
            assert!(body.contains(field), "{field}: {body}");
        }
    }

    #[test]
    fn empty_icon_is_allowed() {
        let mut i = input();
        i.icon = Some("   ".into());
        assert_eq!(validate_submission(&i).unwrap().icon, None);
    }

    #[test]
    fn state_transitions() {
        use ReviewAction::*;
        assert_eq!(transition("pending", Approve), Ok("approved"));
        assert_eq!(transition("pending", Reject), Ok("rejected"));
        assert_eq!(transition("pending", Edit), Ok("pending"));
        assert_eq!(transition("rejected", Withdraw), Ok("withdrawn"));
        assert!(transition("approved", Approve).is_err());
        assert!(transition("approved", Reject).is_err());
        assert!(transition("approved", Withdraw).is_err());
        assert!(transition("rejected", Approve).is_err());
        assert!(transition("rejected", Edit).is_err());
    }

    async fn setup() -> (tempfile::TempDir, Arc<AppState>) {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init(dir.path()).await.unwrap();
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, name, role) VALUES \
             ('admin', 'a@x.io', 'x', 'A', 'admin'), ('dev', 'd@x.io', 'x', 'D', 'developer')",
        )
        .execute(&db)
        .await
        .unwrap();
        let config: crate::config::Config = toml::from_str("").unwrap();
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let state = Arc::new(AppState::new(
            config.clone(),
            db,
            tx,
            Arc::new(crate::runtime::NoopRuntime),
            Arc::new(arc_swap::ArcSwap::from_pointee(
                crate::proxy::RouteTable::new(),
            )),
            Arc::new(crate::engine::updater::UpdateChecker::new(
                config.auto_update.clone(),
            )),
        ));
        (dir, state)
    }

    async fn user(state: &AppState, id: &str) -> User {
        sqlx::query_as("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_one(&state.db)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn submit_approve_and_reject_flow() {
        let (_dir, state) = setup().await;
        let dev = user(&state, "dev").await;
        let admin = user(&state, "admin").await;

        let (code, Json(created)) = create_submission(
            State(state.clone()),
            dev.clone(),
            ClientIp(None),
            Json(input()),
        )
        .await
        .unwrap();
        assert_eq!(code, StatusCode::CREATED);
        assert_eq!(created.submission.status, "pending");
        let id = created.submission.id.clone();

        // Non-admins cannot review or see the queue.
        assert!(list_submissions(
            State(state.clone()),
            dev.clone(),
            Query(ListQuery { status: None })
        )
        .await
        .is_err());
        assert!(reject_submission(
            State(state.clone()),
            dev.clone(),
            ClientIp(None),
            Path(id.clone()),
            Json(RejectRequest {
                reason: "no".into()
            }),
        )
        .await
        .is_err());

        // Admin edits the name while approving.
        let mut edits = input();
        edits.name = "Renamed App".into();
        let Json(approved) = approve_submission(
            State(state.clone()),
            admin.clone(),
            ClientIp(None),
            Path(id.clone()),
            Some(Json(ApproveRequest {
                edits: Some(edits),
                admin_notes: None,
            })),
        )
        .await
        .unwrap();
        assert_eq!(approved.submission.status, "approved");
        let tid = approved.submission.approved_template_id.clone().unwrap();
        let t: crate::db::ServiceTemplate =
            sqlx::query_as("SELECT * FROM service_templates WHERE id = ?")
                .bind(&tid)
                .fetch_one(&state.db)
                .await
                .unwrap();
        assert_eq!(t.name, "Renamed App");
        assert_eq!(t.is_community, 1);
        assert_eq!(t.is_builtin, 0);
        assert_eq!(t.get_env_schema()[0].name, "API_KEY");

        // Approved submissions cannot be reviewed again.
        assert!(approve_submission(
            State(state.clone()),
            admin.clone(),
            ClientIp(None),
            Path(id.clone()),
            None
        )
        .await
        .is_err());

        // Same name is now taken.
        let mut dup = input();
        dup.name = "renamed app".into();
        assert!(
            create_submission(State(state.clone()), dev.clone(), ClientIp(None), Json(dup))
                .await
                .is_err()
        );

        // A second submission gets rejected with a reason; submitter sees it.
        let mut second = input();
        second.name = "Second".into();
        let (_, Json(s2)) = create_submission(
            State(state.clone()),
            dev.clone(),
            ClientIp(None),
            Json(second),
        )
        .await
        .unwrap();
        let Json(rejected) = reject_submission(
            State(state.clone()),
            admin.clone(),
            ClientIp(None),
            Path(s2.submission.id.clone()),
            Json(RejectRequest {
                reason: "Pin the image tag".into(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(rejected.submission.status, "rejected");
        assert_eq!(
            rejected.submission.rejection_reason.as_deref(),
            Some("Pin the image tag")
        );
        let Json(mine) = list_my_submissions(State(state.clone()), dev.clone())
            .await
            .unwrap();
        assert_eq!(mine.len(), 2);
        assert_eq!(
            mine[0].submission.submitter_email.as_deref(),
            Some("d@x.io")
        );

        // Submitter withdraws the rejected one.
        assert_eq!(
            delete_submission(
                State(state.clone()),
                dev.clone(),
                Path(s2.submission.id.clone())
            )
            .await
            .unwrap(),
            StatusCode::NO_CONTENT
        );
    }
}
