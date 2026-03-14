//! Community template submission models.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A user-submitted community service template awaiting admin review
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CommunityTemplateSubmission {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub icon: Option<String>,
    pub compose_content: String,
    pub submitted_by: String,
    pub status: String, // "pending", "approved", "rejected"
    pub admin_notes: Option<String>,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Request body for submitting a new community template
#[derive(Debug, Deserialize)]
pub struct SubmitTemplateRequest {
    pub name: String,
    pub description: String,
    pub category: String,
    pub icon: Option<String>,
    pub compose_content: String,
}

/// Request body for reviewing (approving/rejecting) a submission
#[derive(Debug, Deserialize)]
pub struct ReviewSubmissionRequest {
    pub action: String, // "approve" or "reject"
    pub notes: Option<String>,
}
