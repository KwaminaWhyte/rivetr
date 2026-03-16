//! Community template submission API endpoints.
//!
//! Allows users to submit Docker Compose templates for admin review.
//! Admins can approve (which inserts into service_templates) or reject.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{
    CommunityTemplateSubmission, ReviewSubmissionRequest, SubmitTemplateRequest, User,
};
use crate::AppState;

use super::error::ApiError;

/// POST /api/templates/submit
/// Any authenticated user can submit a new template for admin review.
pub async fn submit_template(
    State(state): State<Arc<AppState>>,
    user: User,
    Json(req): Json<SubmitTemplateRequest>,
) -> Result<(StatusCode, Json<CommunityTemplateSubmission>), ApiError> {
    if req.name.trim().is_empty() {
        return Err(ApiError::validation_field("name", "Name is required"));
    }
    if req.description.trim().is_empty() {
        return Err(ApiError::validation_field(
            "description",
            "Description is required",
        ));
    }
    if req.compose_content.trim().is_empty() {
        return Err(ApiError::validation_field(
            "compose_content",
            "Compose content is required",
        ));
    }

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO community_template_submissions
            (id, name, description, category, icon, compose_content, submitted_by, status, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.category)
    .bind(&req.icon)
    .bind(&req.compose_content)
    .bind(&user.id)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to submit community template: {}", e);
        ApiError::database("Failed to submit template")
    })?;

    let submission = sqlx::query_as::<_, CommunityTemplateSubmission>(
        "SELECT * FROM community_template_submissions WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await?;

    Ok((StatusCode::CREATED, Json(submission)))
}

/// GET /api/templates/submissions
/// Admin only — list all submissions (optionally filtered by status).
pub async fn list_submissions(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<Vec<CommunityTemplateSubmission>>, ApiError> {
    if user.role != "admin" {
        return Err(ApiError::forbidden("Admin access required"));
    }

    let submissions = sqlx::query_as::<_, CommunityTemplateSubmission>(
        "SELECT * FROM community_template_submissions ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(submissions))
}

/// GET /api/templates/submissions/:id
/// Admin or the original submitter can view the submission.
pub async fn get_submission(
    State(state): State<Arc<AppState>>,
    Path(submission_id): Path<String>,
    user: User,
) -> Result<Json<CommunityTemplateSubmission>, ApiError> {
    let submission = sqlx::query_as::<_, CommunityTemplateSubmission>(
        "SELECT * FROM community_template_submissions WHERE id = ?",
    )
    .bind(&submission_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Submission not found"))?;

    if user.role != "admin" && submission.submitted_by != user.id {
        return Err(ApiError::forbidden(
            "You can only view your own submissions",
        ));
    }

    Ok(Json(submission))
}

/// PUT /api/templates/submissions/:id/review
/// Admin only — approve or reject a submission.
/// On approval, inserts a new row into service_templates.
pub async fn review_submission(
    State(state): State<Arc<AppState>>,
    Path(submission_id): Path<String>,
    user: User,
    Json(req): Json<ReviewSubmissionRequest>,
) -> Result<Json<CommunityTemplateSubmission>, ApiError> {
    if user.role != "admin" {
        return Err(ApiError::forbidden("Admin access required"));
    }

    if req.action != "approve" && req.action != "reject" {
        return Err(ApiError::validation_field(
            "action",
            "Action must be 'approve' or 'reject'",
        ));
    }

    let submission = sqlx::query_as::<_, CommunityTemplateSubmission>(
        "SELECT * FROM community_template_submissions WHERE id = ?",
    )
    .bind(&submission_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Submission not found"))?;

    if submission.status != "pending" {
        return Err(ApiError::bad_request(
            "Only pending submissions can be reviewed",
        ));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let new_status = if req.action == "approve" {
        "approved"
    } else {
        "rejected"
    };

    // Update submission status
    sqlx::query(
        "UPDATE community_template_submissions SET status = ?, admin_notes = ?, reviewed_by = ?, reviewed_at = ?, updated_at = ? WHERE id = ?",
    )
    .bind(new_status)
    .bind(&req.notes)
    .bind(&user.id)
    .bind(&now)
    .bind(&now)
    .bind(&submission_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to review submission: {}", e);
        ApiError::database("Failed to update submission")
    })?;

    // On approval, insert into service_templates
    if req.action == "approve" {
        let template_id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO service_templates (id, name, description, category, icon, compose_template, is_builtin, created_at)
            VALUES (?, ?, ?, ?, ?, ?, 0, ?)
            "#,
        )
        .bind(&template_id)
        .bind(&submission.name)
        .bind(&submission.description)
        .bind(&submission.category)
        .bind(&submission.icon)
        .bind(&submission.compose_content)
        .bind(&now)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to insert approved template: {}", e);
            ApiError::database("Failed to create service template from submission")
        })?;

        tracing::info!(
            "Community template '{}' approved by {} — inserted as service_template {}",
            submission.name,
            user.email,
            template_id
        );
    }

    let updated = sqlx::query_as::<_, CommunityTemplateSubmission>(
        "SELECT * FROM community_template_submissions WHERE id = ?",
    )
    .bind(&submission_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(updated))
}

/// GET /api/templates/my-submissions
/// List the current user's own submissions.
pub async fn my_submissions(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<Vec<CommunityTemplateSubmission>>, ApiError> {
    let submissions = sqlx::query_as::<_, CommunityTemplateSubmission>(
        "SELECT * FROM community_template_submissions WHERE submitted_by = ? ORDER BY created_at DESC",
    )
    .bind(&user.id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(submissions))
}

/// DELETE /api/templates/submissions/:id
/// The submitter or an admin can delete a pending submission.
pub async fn delete_submission(
    State(state): State<Arc<AppState>>,
    Path(submission_id): Path<String>,
    user: User,
) -> Result<StatusCode, ApiError> {
    let submission = sqlx::query_as::<_, CommunityTemplateSubmission>(
        "SELECT * FROM community_template_submissions WHERE id = ?",
    )
    .bind(&submission_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Submission not found"))?;

    if user.role != "admin" && submission.submitted_by != user.id {
        return Err(ApiError::forbidden(
            "You can only delete your own submissions",
        ));
    }

    // Admins can delete any; submitters can only delete pending ones
    if user.role != "admin" && submission.status != "pending" {
        return Err(ApiError::bad_request(
            "You can only delete pending submissions",
        ));
    }

    sqlx::query("DELETE FROM community_template_submissions WHERE id = ?")
        .bind(&submission_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete submission: {}", e);
            ApiError::database("Failed to delete submission")
        })?;

    Ok(StatusCode::NO_CONTENT)
}
