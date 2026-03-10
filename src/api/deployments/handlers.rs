use axum::{
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{
    actions, resource_types, App, Deployment, DeploymentLog, TeamAuditAction,
    TeamAuditResourceType, User,
};
use crate::engine::{detect_build_type, extract_zip_and_find_root, BuildDetectionResult};
use crate::runtime::ContainerStats;
use crate::AppState;

use crate::api::audit::{audit_log, extract_client_ip};
use crate::api::error::ApiError;
use crate::api::teams::log_team_audit;
use crate::api::validation::validate_uuid;

use super::freeze::check_freeze_windows;

/// Maximum upload size (100MB)
pub const MAX_UPLOAD_SIZE: usize = 100 * 1024 * 1024;

/// Query parameters for listing deployments with pagination
#[derive(Debug, Deserialize)]
pub struct DeploymentListQuery {
    /// Page number (1-indexed, default: 1)
    #[serde(default = "default_page")]
    pub page: i64,
    /// Items per page (default: 20, max: 100)
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    20
}

/// Paginated response for deployment list
#[derive(Debug, Serialize)]
pub struct DeploymentListResponse {
    /// List of deployments
    pub items: Vec<Deployment>,
    /// Total number of deployments
    pub total: i64,
    /// Current page number
    pub page: i64,
    /// Items per page
    pub per_page: i64,
    /// Total number of pages
    pub total_pages: i64,
}

/// Request body for deploy trigger with optional commit/tag targeting
#[derive(Debug, Deserialize, Default)]
pub struct TriggerDeployRequest {
    /// Deploy a specific commit SHA instead of branch HEAD
    pub commit_sha: Option<String>,
    /// Deploy a specific git tag instead of branch HEAD
    pub git_tag: Option<String>,
    /// Schedule the deployment for a specific time (ISO 8601 format)
    pub scheduled_at: Option<String>,
}

/// Response from upload deploy endpoint
#[derive(Debug, Serialize)]
pub struct UploadDeployResponse {
    pub deployment: Deployment,
    pub detected_build_type: BuildDetectionResult,
}

/// Query parameters for listing commits or tags
#[derive(Debug, Deserialize)]
pub struct CommitsTagsQuery {
    /// Maximum number of items to return (default: 20, max: 100)
    #[serde(default = "default_commits_limit")]
    pub limit: u32,
}

fn default_commits_limit() -> u32 {
    20
}

/// Commit info returned by the API
#[derive(Debug, Serialize)]
pub struct CommitInfo {
    pub sha: String,
    pub message: String,
    pub author: String,
    pub date: String,
}

/// Tag info returned by the API
#[derive(Debug, Serialize)]
pub struct TagInfo {
    pub name: String,
    pub sha: String,
}

pub async fn trigger_deploy(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Path(app_id): Path<String>,
    body: Option<Json<TriggerDeployRequest>>,
) -> Result<(StatusCode, Json<Deployment>), ApiError> {
    // Validate app_id format
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Check if app exists
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Check if there's already a deployment in progress
    let in_progress: Option<Deployment> = sqlx::query_as(
        "SELECT * FROM deployments WHERE app_id = ? AND status IN ('pending', 'cloning', 'building', 'starting', 'checking')"
    )
    .bind(&app_id)
    .fetch_optional(&state.db)
    .await?;

    if let Some(existing) = in_progress {
        return Err(ApiError::conflict(format!(
            "A deployment is already in progress (id: {})",
            existing.id
        )));
    }

    // Check if this is an upload-based app that needs special handling
    let is_upload_app = app.deployment_source.as_deref() == Some("upload");

    // For upload-based apps, we need to find a way to redeploy:
    // 1. If source directory still exists, rebuild from source
    // 2. If not but image exists, restart from existing image
    // 3. If neither, require new ZIP upload
    let (upload_source_path, existing_image_tag): (Option<String>, Option<String>) =
        if is_upload_app {
            // First, check if source directory from last deployment still exists
            let last_deployment: Option<(Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT commit_sha, image_tag FROM deployments WHERE app_id = ? AND status IN ('running', 'stopped') ORDER BY started_at DESC LIMIT 1"
        )
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await?;

            match last_deployment {
                Some((Some(path), image_tag)) if path.contains("rivetr-upload-") => {
                    // Check if the source directory still exists
                    let source_path = std::path::Path::new(&path);
                    if source_path.exists() {
                        // Rebuild from source
                        (Some(path), None)
                    } else if let Some(ref tag) = image_tag {
                        // Source cleaned up, but we have an image - restart from image
                        tracing::info!(
                            app_id = %app_id,
                            image_tag = %tag,
                            "Source directory cleaned up, will restart from existing image"
                        );
                        (None, image_tag)
                    } else {
                        // No source and no image
                        return Err(ApiError::bad_request(
                        "This app was deployed from a ZIP file, but the source files have been cleaned up and no image exists. \
                        Please upload a new ZIP file using the 'Deploy from ZIP file' option."
                    ));
                    }
                }
                Some((_, Some(image_tag))) => {
                    // No source path but have an image
                    tracing::info!(
                        app_id = %app_id,
                        image_tag = %image_tag,
                        "No source path found, will restart from existing image"
                    );
                    (None, Some(image_tag))
                }
                _ => {
                    // No previous deployment found
                    return Err(ApiError::bad_request(
                    "This app was configured for ZIP upload deployment but has no previous successful deployment. \
                    Please upload a ZIP file using the 'Deploy from ZIP file' option."
                ));
                }
            }
        } else {
            (None, None)
        };

    let deployment_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // Extract deploy options from request body
    let deploy_opts = body.map(|b| b.0).unwrap_or_default();

    // Check freeze windows before queuing (skip for upload apps — they are manual)
    if !is_upload_app {
        check_freeze_windows(&state, &app, &now).await?;
    }

    // Determine commit_sha and git_tag for the deployment record
    // For upload apps: commit_sha stores source path, git_tag is unused
    // For git apps: commit_sha/git_tag store the requested target
    let (deploy_commit_sha, deploy_git_tag) = if is_upload_app {
        (upload_source_path, None)
    } else {
        (deploy_opts.commit_sha.clone(), deploy_opts.git_tag.clone())
    };

    // Determine if this deployment needs approval
    // Approval is required if: app.require_approval is set AND user is not admin
    let needs_approval = app.require_approval != 0 && user.role != "admin";

    let approval_status: Option<&str> = if needs_approval {
        Some("pending")
    } else {
        None
    };

    // For upload apps with existing image, store the image tag to reuse
    // commit_sha stores source path for rebuild, image_tag stores image for restart
    sqlx::query(
        r#"
        INSERT INTO deployments (id, app_id, status, started_at, commit_sha, image_tag, git_tag, approval_status, scheduled_at)
        VALUES (?, ?, 'pending', ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&deployment_id)
    .bind(&app_id)
    .bind(&now)
    .bind(&deploy_commit_sha)
    .bind(&existing_image_tag)
    .bind(&deploy_git_tag)
    .bind(approval_status)
    .bind(&deploy_opts.scheduled_at)
    .execute(&state.db)
    .await?;

    // Only queue immediately if no approval needed and not scheduled for the future
    let should_queue_now = !needs_approval && deploy_opts.scheduled_at.is_none();

    if should_queue_now {
        // Queue the deployment job
        if let Err(e) = state
            .deploy_tx
            .send((deployment_id.clone(), app.clone()))
            .await
        {
            tracing::error!("Failed to queue deployment: {}", e);
            return Err(ApiError::internal("Failed to queue deployment job"));
        }
    } else if needs_approval {
        tracing::info!(
            deployment_id = %deployment_id,
            app_id = %app_id,
            "Deployment requires approval, awaiting approver action"
        );
    } else {
        tracing::info!(
            deployment_id = %deployment_id,
            scheduled_at = ?deploy_opts.scheduled_at,
            "Deployment scheduled for future execution"
        );
    }

    let deployment = sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = ?")
        .bind(&deployment_id)
        .fetch_one(&state.db)
        .await?;

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::DEPLOYMENT_TRIGGER,
        resource_types::APP,
        Some(&app.id),
        Some(&app.name),
        Some(&user.id),
        ip.as_deref(),
        Some(serde_json::json!({
            "deployment_id": deployment.id,
        })),
    )
    .await;

    // Log team audit event if app belongs to a team
    if let Some(ref team_id) = app.team_id {
        if let Err(e) = log_team_audit(
            &state.db,
            team_id,
            Some(&user.id),
            TeamAuditAction::DeploymentTriggered,
            TeamAuditResourceType::Deployment,
            Some(&deployment.id),
            Some(serde_json::json!({
                "app_id": app.id,
                "app_name": app.name,
            })),
        )
        .await
        {
            tracing::warn!("Failed to log team audit event: {}", e);
        }
    }

    Ok((StatusCode::ACCEPTED, Json(deployment)))
}

pub async fn list_deployments(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Query(query): Query<DeploymentListQuery>,
) -> Result<Json<DeploymentListResponse>, ApiError> {
    // Validate app_id format
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Verify the app exists
    let app_exists: Option<(String,)> = sqlx::query_as("SELECT id FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await?;

    if app_exists.is_none() {
        return Err(ApiError::not_found("App not found"));
    }

    // Normalize pagination parameters
    let page = query.page.max(1);
    let per_page = query.per_page.clamp(1, 100);
    let offset = (page - 1) * per_page;

    // Get total count
    let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM deployments WHERE app_id = ?")
        .bind(&app_id)
        .fetch_one(&state.db)
        .await?;

    // Calculate total pages
    let total_pages = (total + per_page - 1) / per_page;

    // Fetch paginated deployments
    let deployments = sqlx::query_as::<_, Deployment>(
        "SELECT * FROM deployments WHERE app_id = ? ORDER BY started_at DESC LIMIT ? OFFSET ?",
    )
    .bind(&app_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(DeploymentListResponse {
        items: deployments,
        total,
        page,
        per_page,
        total_pages,
    }))
}

pub async fn get_deployment(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Deployment>, ApiError> {
    // Validate deployment_id format
    if let Err(e) = validate_uuid(&id, "deployment_id") {
        return Err(ApiError::validation_field("deployment_id", e));
    }

    let deployment = sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Deployment not found"))?;

    Ok(Json(deployment))
}

pub async fn get_logs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<DeploymentLog>>, ApiError> {
    // Validate deployment_id format
    if let Err(e) = validate_uuid(&id, "deployment_id") {
        return Err(ApiError::validation_field("deployment_id", e));
    }

    // Verify the deployment exists
    let deployment_exists: Option<(String,)> =
        sqlx::query_as("SELECT id FROM deployments WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await?;

    if deployment_exists.is_none() {
        return Err(ApiError::not_found("Deployment not found"));
    }

    let logs = sqlx::query_as::<_, DeploymentLog>(
        "SELECT * FROM deployment_logs WHERE deployment_id = ? ORDER BY id ASC",
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(logs))
}

/// Get container resource stats for a running app
/// GET /api/apps/:id/stats
///
/// Returns current CPU, memory, and network statistics for the container.
/// Only available for apps with a running deployment.
pub async fn get_app_stats(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<ContainerStats>, ApiError> {
    // Validate app_id format
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Check if app exists
    let _app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Find the currently running deployment for this app
    let running_deployment: Option<Deployment> = sqlx::query_as(
        "SELECT * FROM deployments WHERE app_id = ? AND status = 'running' ORDER BY started_at DESC LIMIT 1"
    )
    .bind(&app_id)
    .fetch_optional(&state.db)
    .await?;

    let deployment = running_deployment
        .ok_or_else(|| ApiError::not_found("No running deployment found for this app"))?;

    let container_id = deployment
        .container_id
        .ok_or_else(|| ApiError::not_found("Running deployment has no container ID"))?;

    // Get stats from the container runtime
    let stats = state.runtime.stats(&container_id).await.map_err(|e| {
        tracing::warn!("Failed to get container stats for {}: {}", container_id, e);
        ApiError::internal(format!("Failed to get container stats: {}", e))
    })?;

    Ok(Json(stats))
}

/// Deploy an app from uploaded ZIP file
/// POST /api/apps/:id/deploy/upload
///
/// Accepts a multipart form with a ZIP file containing the project source.
/// Auto-detects build type (Dockerfile, Nixpacks, Static Site).
pub async fn upload_deploy(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Path(app_id): Path<String>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<UploadDeployResponse>), ApiError> {
    // Validate app_id format
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Check if app exists
    let _app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Check if there's already a deployment in progress
    let in_progress: Option<Deployment> = sqlx::query_as(
        "SELECT * FROM deployments WHERE app_id = ? AND status IN ('pending', 'cloning', 'building', 'starting', 'checking')"
    )
    .bind(&app_id)
    .fetch_optional(&state.db)
    .await?;

    if let Some(existing) = in_progress {
        return Err(ApiError::conflict(format!(
            "A deployment is already in progress (id: {})",
            existing.id
        )));
    }

    // Extract ZIP file from multipart
    let mut zip_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(format!("Failed to read multipart field: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" || name == "zip" {
            file_name = field.file_name().map(|s| s.to_string());

            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::bad_request(format!("Failed to read file: {}", e)))?;

            if data.len() > MAX_UPLOAD_SIZE {
                return Err(ApiError::bad_request(format!(
                    "File too large. Maximum size is {} MB",
                    MAX_UPLOAD_SIZE / 1024 / 1024
                )));
            }

            zip_data = Some(data.to_vec());
        }
    }

    let zip_data = zip_data.ok_or_else(|| {
        ApiError::bad_request(
            "No ZIP file provided. Include a 'file' or 'zip' field in the multipart form",
        )
    })?;

    tracing::info!(
        app_id = %app_id,
        file_name = ?file_name,
        size = zip_data.len(),
        "Processing ZIP upload for deployment"
    );

    // Create deployment ID and temp directory
    let deployment_id = Uuid::new_v4().to_string();
    let work_dir = std::env::temp_dir().join(format!("rivetr-upload-{}", deployment_id));

    // Extract ZIP and find project root
    let project_root = extract_zip_and_find_root(&zip_data, &work_dir)
        .await
        .map_err(|e| ApiError::bad_request(format!("Failed to extract ZIP: {}", e)))?;

    // Auto-detect build type
    let detection = detect_build_type(&project_root)
        .await
        .map_err(|e| ApiError::internal(format!("Build detection failed: {}", e)))?;

    tracing::info!(
        deployment_id = %deployment_id,
        build_type = ?detection.build_type,
        confidence = detection.confidence,
        detected_from = %detection.detected_from,
        "Build type detected for uploaded project"
    );

    // Create deployment record
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO deployments (id, app_id, status, started_at, commit_message)
        VALUES (?, ?, 'pending', ?, ?)
        "#,
    )
    .bind(&deployment_id)
    .bind(&app_id)
    .bind(&now)
    .bind(format!("Upload deployment: {:?}", detection.build_type))
    .execute(&state.db)
    .await?;

    // Store the upload source path for the deployment engine
    // We'll use a special mechanism to pass the source directory
    let source_path = project_root.to_string_lossy().to_string();

    // Update app with detected build type if not already set
    let build_type_str = match detection.build_type {
        crate::engine::BuildType::Dockerfile => "dockerfile",
        crate::engine::BuildType::Nixpacks => "nixpacks",
        crate::engine::BuildType::Railpack => "railpack",
        crate::engine::BuildType::Cnb => "cnb",
        crate::engine::BuildType::StaticSite => "static",
        crate::engine::BuildType::DockerCompose => "dockerfile", // Fallback for compose
        crate::engine::BuildType::DockerImage => "dockerfile",   // Fallback for image
    };

    // Update app's build_type and publish_directory if detected
    if let Some(ref publish_dir) = detection.publish_directory {
        sqlx::query("UPDATE apps SET build_type = ?, publish_directory = ?, deployment_source = 'upload' WHERE id = ?")
            .bind(build_type_str)
            .bind(publish_dir)
            .bind(&app_id)
            .execute(&state.db)
            .await?;
    } else {
        sqlx::query("UPDATE apps SET build_type = ?, deployment_source = 'upload' WHERE id = ?")
            .bind(build_type_str)
            .bind(&app_id)
            .execute(&state.db)
            .await?;
    }

    // Re-fetch app with updated fields
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_one(&state.db)
        .await?;

    // Queue the deployment job with source path
    // The engine will use this path instead of cloning from git
    // Store source path in deployment metadata
    sqlx::query("UPDATE deployments SET commit_sha = ? WHERE id = ?")
        .bind(&source_path) // Using commit_sha to store source path temporarily
        .bind(&deployment_id)
        .execute(&state.db)
        .await?;

    // Queue the deployment
    if let Err(e) = state
        .deploy_tx
        .send((deployment_id.clone(), app.clone()))
        .await
    {
        // Cleanup on error
        let _ = tokio::fs::remove_dir_all(&work_dir).await;
        tracing::error!("Failed to queue deployment: {}", e);
        return Err(ApiError::internal("Failed to queue deployment job"));
    }

    let deployment = sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = ?")
        .bind(&deployment_id)
        .fetch_one(&state.db)
        .await?;

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::DEPLOYMENT_TRIGGER,
        resource_types::APP,
        Some(&app.id),
        Some(&app.name),
        Some(&user.id),
        ip.as_deref(),
        Some(serde_json::json!({
            "deployment_id": deployment.id,
            "source": "upload",
            "build_type": build_type_str,
            "file_name": file_name,
        })),
    )
    .await;

    // Log team audit event if app belongs to a team
    if let Some(ref team_id) = app.team_id {
        if let Err(e) = log_team_audit(
            &state.db,
            team_id,
            Some(&user.id),
            TeamAuditAction::DeploymentTriggered,
            TeamAuditResourceType::Deployment,
            Some(&deployment.id),
            Some(serde_json::json!({
                "app_id": app.id,
                "app_name": app.name,
                "source": "upload",
                "build_type": build_type_str,
            })),
        )
        .await
        {
            tracing::warn!("Failed to log team audit event: {}", e);
        }
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(UploadDeployResponse {
            deployment,
            detected_build_type: detection,
        }),
    ))
}

/// Preview build type detection from uploaded ZIP
/// POST /api/build/detect
///
/// Upload a ZIP file to detect the build type without creating a deployment.
/// Useful for previewing detection results before deployment.
pub async fn detect_build_type_from_upload(
    mut multipart: Multipart,
) -> Result<Json<BuildDetectionResult>, ApiError> {
    // Extract ZIP file from multipart
    let mut zip_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(format!("Failed to read multipart field: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" || name == "zip" {
            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::bad_request(format!("Failed to read file: {}", e)))?;

            if data.len() > MAX_UPLOAD_SIZE {
                return Err(ApiError::bad_request(format!(
                    "File too large. Maximum size is {} MB",
                    MAX_UPLOAD_SIZE / 1024 / 1024
                )));
            }

            zip_data = Some(data.to_vec());
        }
    }

    let zip_data = zip_data.ok_or_else(|| ApiError::bad_request("No ZIP file provided"))?;

    // Create temp directory
    let temp_id = Uuid::new_v4().to_string();
    let work_dir = std::env::temp_dir().join(format!("rivetr-detect-{}", temp_id));

    // Extract ZIP and detect
    let result = async {
        let project_root = extract_zip_and_find_root(&zip_data, &work_dir).await?;
        detect_build_type(&project_root).await
    }
    .await;

    // Cleanup temp directory
    let _ = tokio::fs::remove_dir_all(&work_dir).await;

    let detection =
        result.map_err(|e| ApiError::bad_request(format!("Detection failed: {}", e)))?;

    Ok(Json(detection))
}

/// Helper to parse owner/repo from a git URL
pub fn parse_owner_repo(git_url: &str) -> Option<(String, String)> {
    // Handle HTTPS URLs: https://github.com/owner/repo.git
    if let Some(path) = git_url
        .strip_prefix("https://github.com/")
        .or_else(|| git_url.strip_prefix("http://github.com/"))
    {
        let path = path.trim_end_matches(".git").trim_end_matches('/');
        let parts: Vec<&str> = path.splitn(2, '/').collect();
        if parts.len() == 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }

    // Handle SSH URLs: git@github.com:owner/repo.git
    if let Some(path) = git_url.strip_prefix("git@github.com:") {
        let path = path.trim_end_matches(".git").trim_end_matches('/');
        let parts: Vec<&str> = path.splitn(2, '/').collect();
        if parts.len() == 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }

    None
}

/// Helper to get a GitHub API client for an app's repository
pub async fn get_github_client_for_app(
    state: &AppState,
    app: &App,
) -> Result<crate::github::GitHubClient, ApiError> {
    use crate::crypto;
    use crate::db::{GitHubApp, GitHubAppInstallation};
    use crate::github::get_installation_token;

    let installation_id_str = app
        .github_app_installation_id
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("App has no GitHub App installation configured"))?;

    let installation: GitHubAppInstallation =
        sqlx::query_as("SELECT * FROM github_app_installations WHERE id = ?")
            .bind(installation_id_str)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| ApiError::not_found("GitHub App installation not found"))?;

    let github_app: GitHubApp = sqlx::query_as("SELECT * FROM github_apps WHERE id = ?")
        .bind(&installation.github_app_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("GitHub App not found"))?;

    let encryption_key = state
        .config
        .auth
        .encryption_key
        .as_ref()
        .map(|k| crypto::derive_key(k));
    let private_key =
        crypto::decrypt_if_encrypted(&github_app.private_key, encryption_key.as_ref())
            .map_err(|e| ApiError::internal(format!("Failed to decrypt private key: {}", e)))?;

    let token_response = get_installation_token(
        github_app.app_id,
        &private_key,
        installation.installation_id,
    )
    .await
    .map_err(|e| ApiError::internal(format!("Failed to get installation token: {}", e)))?;

    Ok(crate::github::GitHubClient::new(token_response.token))
}

/// List recent commits for an app's repository
/// GET /api/apps/:id/commits?limit=20
pub async fn list_commits(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Query(query): Query<CommitsTagsQuery>,
) -> Result<Json<Vec<CommitInfo>>, ApiError> {
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    let limit = query.limit.min(100);

    // Try GitHub App integration first
    if app.github_app_installation_id.is_some() {
        if let Some((owner, repo)) = parse_owner_repo(&app.git_url) {
            let client = get_github_client_for_app(&state, &app).await?;
            let commits = client
                .list_commits(&owner, &repo, Some(&app.branch), limit)
                .await
                .map_err(|e| {
                    ApiError::internal(format!("Failed to fetch commits from GitHub: {}", e))
                })?;

            let result: Vec<CommitInfo> = commits
                .into_iter()
                .map(|c| {
                    let first_line = c.commit.message.lines().next().unwrap_or("").to_string();
                    CommitInfo {
                        sha: c.sha,
                        message: first_line,
                        author: c
                            .author
                            .map(|a| a.login)
                            .unwrap_or_else(|| c.commit.author.name),
                        date: c.commit.author.date,
                    }
                })
                .collect();

            return Ok(Json(result));
        }
    }

    // Fallback: return empty list for non-GitHub repos
    Ok(Json(vec![]))
}

/// List tags for an app's repository
/// GET /api/apps/:id/tags?limit=20
pub async fn list_tags(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Query(query): Query<CommitsTagsQuery>,
) -> Result<Json<Vec<TagInfo>>, ApiError> {
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    let limit = query.limit.min(100);

    // Try GitHub App integration first
    if app.github_app_installation_id.is_some() {
        if let Some((owner, repo)) = parse_owner_repo(&app.git_url) {
            let client = get_github_client_for_app(&state, &app).await?;
            let tags = client.list_tags(&owner, &repo, limit).await.map_err(|e| {
                ApiError::internal(format!("Failed to fetch tags from GitHub: {}", e))
            })?;

            let result: Vec<TagInfo> = tags
                .into_iter()
                .map(|t| TagInfo {
                    name: t.name,
                    sha: t.commit.sha,
                })
                .collect();

            return Ok(Json(result));
        }
    }

    // Fallback: return empty list for non-GitHub repos
    Ok(Json(vec![]))
}

/// Deployment diff response showing what changed between deployments
#[derive(Debug, Serialize)]
pub struct DeploymentDiff {
    pub deployment_id: String,
    pub previous_deployment_id: Option<String>,
    pub current_sha: Option<String>,
    pub previous_sha: Option<String>,
    /// Number of commits between the two deployments (if calculable)
    pub commits_count: i64,
    /// Human-readable summary
    pub summary: String,
    /// File paths changed (if available via git provider API)
    pub files_changed: Vec<String>,
    /// Commit messages in the range
    pub commit_messages: Vec<String>,
}

/// Get the diff between a deployment and the previous successful one
/// GET /api/deployments/:id/diff
#[allow(unused_assignments)]
pub async fn get_deployment_diff(
    State(state): State<Arc<AppState>>,
    Path(deployment_id): Path<String>,
    _user: User,
) -> Result<Json<DeploymentDiff>, ApiError> {
    if let Err(e) = validate_uuid(&deployment_id, "deployment_id") {
        return Err(ApiError::validation_field("deployment_id", e));
    }

    let deployment = sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = ?")
        .bind(&deployment_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Deployment not found"))?;

    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&deployment.app_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    let previous: Option<Deployment> = sqlx::query_as(
        r#"SELECT * FROM deployments
           WHERE app_id = ?
             AND id != ?
             AND status IN ('running', 'stopped', 'replaced')
           ORDER BY started_at DESC
           LIMIT 1"#,
    )
    .bind(&deployment.app_id)
    .bind(&deployment_id)
    .fetch_optional(&state.db)
    .await?;

    let current_sha = deployment.commit_sha.clone();
    let previous_sha = previous.as_ref().and_then(|p| p.commit_sha.clone());
    let previous_deployment_id = previous.as_ref().map(|p| p.id.clone());

    let mut commits_count: i64 = 0;
    let mut summary = String::new();
    let mut files_changed: Vec<String> = vec![];
    let mut commit_messages: Vec<String> = vec![];

    match (&current_sha, &previous_sha) {
        (Some(cur), Some(prev)) if cur != prev => {
            if app.github_app_installation_id.is_some() {
                if let Some((owner, repo)) = parse_owner_repo(&app.git_url) {
                    match get_github_client_for_app(&state, &app).await {
                        Ok(client) => {
                            match client.compare_commits(&owner, &repo, prev, cur).await {
                                Ok(comparison) => {
                                    commits_count = comparison.commits.len() as i64;
                                    commit_messages = comparison
                                        .commits
                                        .iter()
                                        .map(|c| {
                                            c.commit
                                                .message
                                                .lines()
                                                .next()
                                                .unwrap_or("")
                                                .to_string()
                                        })
                                        .collect();
                                    files_changed = comparison
                                        .files
                                        .iter()
                                        .map(|f| f.filename.clone())
                                        .collect();
                                    summary = format!(
                                        "{} commit{}, {} file{} changed",
                                        commits_count,
                                        if commits_count == 1 { "" } else { "s" },
                                        files_changed.len(),
                                        if files_changed.len() == 1 { "" } else { "s" },
                                    );
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Failed to get GitHub comparison for deployment {}: {}",
                                        deployment_id,
                                        e
                                    );
                                    summary = format!(
                                        "{} -> {} (diff unavailable)",
                                        &prev[..7.min(prev.len())],
                                        &cur[..7.min(cur.len())]
                                    );
                                }
                            }
                        }
                        Err(_) => {
                            summary = format!(
                                "{} -> {}",
                                &prev[..7.min(prev.len())],
                                &cur[..7.min(cur.len())]
                            );
                        }
                    }
                } else {
                    summary = format!(
                        "{} -> {}",
                        &prev[..7.min(prev.len())],
                        &cur[..7.min(cur.len())]
                    );
                }
            } else {
                summary = format!(
                    "{} -> {}",
                    &prev[..7.min(prev.len())],
                    &cur[..7.min(cur.len())]
                );
            }
        }
        (Some(cur), None) => {
            summary = format!("First deployment ({})", &cur[..7.min(cur.len())]);
        }
        (Some(cur), Some(_prev)) => {
            summary = format!("Same commit ({})", &cur[..7.min(cur.len())]);
        }
        _ => {
            summary = if previous_deployment_id.is_some() {
                "No commit SHAs available for comparison".to_string()
            } else {
                "No previous deployment".to_string()
            };
        }
    }

    Ok(Json(DeploymentDiff {
        deployment_id,
        previous_deployment_id,
        current_sha,
        previous_sha,
        commits_count,
        summary,
        files_changed,
        commit_messages,
    }))
}
