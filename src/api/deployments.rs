use axum::{
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::crypto;
use crate::db::{
    actions, resource_types, App, Deployment, DeploymentFreezeWindow, DeploymentLog, GitHubApp,
    GitHubAppInstallation, TeamAuditAction, TeamAuditResourceType, User,
};
use crate::engine::{
    detect_build_type, extract_zip_and_find_root, run_rollback, BuildDetectionResult,
};
use crate::github::{get_installation_token, GitHubClient};
use crate::proxy::Backend;
use crate::runtime::ContainerStats;
use crate::AppState;

use super::audit::{audit_log, extract_client_ip};
use super::error::ApiError;
use super::teams::log_team_audit;
use super::validation::validate_uuid;

/// Key length for AES-256 encryption
const KEY_LENGTH: usize = 32;

/// Get the derived encryption key from the config if configured
fn get_encryption_key(state: &AppState) -> Option<[u8; KEY_LENGTH]> {
    state
        .config
        .auth
        .encryption_key
        .as_ref()
        .map(|secret| crypto::derive_key(secret))
}

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

/// Request body for rollback endpoint
#[derive(Debug, Deserialize)]
pub struct RollbackRequest {
    /// Optional: specify which deployment to roll back to.
    /// If not provided, rolls back to the previous successful deployment.
    pub target_deployment_id: Option<String>,
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

/// Rollback a deployment to a previous version
/// POST /api/deployments/:id/rollback
///
/// This endpoint allows rolling back to a previous successful deployment.
/// If no target_deployment_id is provided in the request body, it will
/// automatically roll back to the most recent successful deployment before
/// the current one.
pub async fn rollback_deployment(
    State(state): State<Arc<AppState>>,
    Path(deployment_id): Path<String>,
    Json(body): Json<Option<RollbackRequest>>,
) -> Result<(StatusCode, Json<Deployment>), ApiError> {
    // Validate deployment_id format
    if let Err(e) = validate_uuid(&deployment_id, "deployment_id") {
        return Err(ApiError::validation_field("deployment_id", e));
    }

    // Get the current deployment
    let current_deployment =
        sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = ?")
            .bind(&deployment_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| ApiError::not_found("Deployment not found"))?;

    // Get the app
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&current_deployment.app_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Check if there's already a deployment in progress
    let in_progress: Option<Deployment> = sqlx::query_as(
        "SELECT * FROM deployments WHERE app_id = ? AND status IN ('pending', 'cloning', 'building', 'starting', 'checking')"
    )
    .bind(&current_deployment.app_id)
    .fetch_optional(&state.db)
    .await?;

    if let Some(existing) = in_progress {
        return Err(ApiError::conflict(format!(
            "A deployment is already in progress (id: {})",
            existing.id
        )));
    }

    // Determine target deployment
    let target_deployment = if let Some(ref req) = body {
        if let Some(ref target_id) = req.target_deployment_id {
            // Validate target_deployment_id format
            if let Err(e) = validate_uuid(target_id, "target_deployment_id") {
                return Err(ApiError::validation_field("target_deployment_id", e));
            }

            // Fetch the specified target deployment (allow running, stopped, or replaced statuses)
            sqlx::query_as::<_, Deployment>(
                "SELECT * FROM deployments WHERE id = ? AND app_id = ? AND status IN ('running', 'stopped', 'replaced') AND image_tag IS NOT NULL"
            )
            .bind(target_id)
            .bind(&current_deployment.app_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| ApiError::not_found("Target deployment not found or has no image tag for rollback"))?
        } else {
            // Find the previous successful deployment
            find_previous_successful_deployment(&state, &current_deployment).await?
        }
    } else {
        // Find the previous successful deployment
        find_previous_successful_deployment(&state, &current_deployment).await?
    };

    // Verify target has an image_tag (required for rollback)
    if target_deployment.image_tag.is_none() {
        return Err(ApiError::bad_request(
            "Target deployment has no image tag - cannot rollback. This deployment may have been created before rollback support was added."
        ));
    }

    // Create a new deployment record for the rollback
    let rollback_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO deployments (id, app_id, commit_sha, commit_message, status, started_at)
        VALUES (?, ?, ?, ?, 'pending', ?)
        "#,
    )
    .bind(&rollback_id)
    .bind(&current_deployment.app_id)
    .bind(&target_deployment.commit_sha)
    .bind(format!("Rollback to deployment {}", target_deployment.id))
    .bind(&now)
    .execute(&state.db)
    .await?;

    // Run the rollback in a background task
    let db = state.db.clone();
    let runtime = state.runtime.clone();
    let routes = state.routes.clone();
    let rollback_id_clone = rollback_id.clone();
    let target_deployment_clone = target_deployment.clone();
    let app_clone = app.clone();
    let encryption_key = get_encryption_key(&state);

    tokio::spawn(async move {
        match run_rollback(
            &db,
            runtime,
            &rollback_id_clone,
            &target_deployment_clone,
            &app_clone,
            encryption_key.as_ref(),
        )
        .await
        {
            Ok(result) => {
                // Update proxy routes on successful rollback
                if let Some(domain) = &app_clone.domain {
                    if let Some(port) = result.port {
                        let backend = Backend::new(
                            result.container_id.clone(),
                            "127.0.0.1".to_string(),
                            port,
                        );
                        routes.load().add_route(domain.clone(), backend);
                        tracing::info!(
                            domain = %domain,
                            port = port,
                            "Proxy route updated after rollback for app {}",
                            app_clone.name
                        );
                    }
                }
            }
            Err(e) => {
                tracing::error!("Rollback {} failed: {}", rollback_id_clone, e);
                let _ = sqlx::query(
                    "UPDATE deployments SET status = 'failed', error_message = ?, finished_at = ? WHERE id = ?"
                )
                .bind(e.to_string())
                .bind(chrono::Utc::now().to_rfc3339())
                .bind(&rollback_id_clone)
                .execute(&db)
                .await;
            }
        }
    });

    // Return the new rollback deployment record
    let deployment = sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = ?")
        .bind(&rollback_id)
        .fetch_one(&state.db)
        .await?;

    // Log team audit event if app belongs to a team
    if let Some(ref team_id) = app.team_id {
        if let Err(e) = log_team_audit(
            &state.db,
            team_id,
            None, // User context not available in this function
            TeamAuditAction::DeploymentRolledBack,
            TeamAuditResourceType::Deployment,
            Some(&deployment.id),
            Some(serde_json::json!({
                "app_id": app.id,
                "app_name": app.name,
                "target_deployment_id": target_deployment.id,
            })),
        )
        .await
        {
            tracing::warn!("Failed to log team audit event: {}", e);
        }
    }

    Ok((StatusCode::ACCEPTED, Json(deployment)))
}

/// Find the previous successful deployment for an app
async fn find_previous_successful_deployment(
    state: &Arc<AppState>,
    current: &Deployment,
) -> Result<Deployment, ApiError> {
    // Find the most recent deployment with an image_tag that we can roll back to
    // Allow running, stopped, or replaced statuses (these all indicate a deployment that completed successfully)
    sqlx::query_as::<_, Deployment>(
        r#"
        SELECT * FROM deployments
        WHERE app_id = ?
          AND status IN ('running', 'stopped', 'replaced')
          AND id != ?
          AND image_tag IS NOT NULL
        ORDER BY started_at DESC
        LIMIT 1
        "#,
    )
    .bind(&current.app_id)
    .bind(&current.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("No previous successful deployment found to rollback to"))
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

/// Maximum upload size (100MB)
const MAX_UPLOAD_SIZE: usize = 100 * 1024 * 1024;

/// Response from upload deploy endpoint
#[derive(Debug, Serialize)]
pub struct UploadDeployResponse {
    pub deployment: Deployment,
    pub detected_build_type: BuildDetectionResult,
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

// -------------------------------------------------------------------------
// Git Commits and Tags List API
// -------------------------------------------------------------------------

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

/// Helper to parse owner/repo from a git URL
fn parse_owner_repo(git_url: &str) -> Option<(String, String)> {
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
async fn get_github_client_for_app(state: &AppState, app: &App) -> Result<GitHubClient, ApiError> {
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

    Ok(GitHubClient::new(token_response.token))
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

// -------------------------------------------------------------------------
// Freeze Window Helper
// -------------------------------------------------------------------------

/// Check if current time is within any active freeze window for this app/team.
/// Returns 409 Conflict if deployment is frozen.
async fn check_freeze_windows(
    state: &Arc<AppState>,
    app: &App,
    now: &str,
) -> Result<(), ApiError> {
    // Parse current time to get HH:MM and day-of-week
    let now_dt = chrono::DateTime::parse_from_rfc3339(now)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());

    let current_time = now_dt.format("%H:%M").to_string();
    // 0=Sun as per the schema convention
    let current_dow = now_dt.weekday().num_days_from_sunday().to_string();

    // Fetch active freeze windows for this app and/or team
    let windows: Vec<DeploymentFreezeWindow> = if let Some(ref team_id) = app.team_id {
        sqlx::query_as(
            r#"
            SELECT * FROM deployment_freeze_windows
            WHERE is_active = 1
              AND (app_id = ? OR team_id = ?)
            "#,
        )
        .bind(&app.id)
        .bind(team_id)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as(
            "SELECT * FROM deployment_freeze_windows WHERE is_active = 1 AND app_id = ?",
        )
        .bind(&app.id)
        .fetch_all(&state.db)
        .await?
    };

    for window in &windows {
        // Check if current day-of-week is in the window
        let days: Vec<&str> = window.days_of_week.split(',').collect();
        if !days.contains(&current_dow.as_str()) {
            continue;
        }

        // Check if current time is within start_time..end_time (HH:MM strings)
        let in_window = if window.start_time <= window.end_time {
            current_time >= window.start_time && current_time < window.end_time
        } else {
            // Wraps midnight
            current_time >= window.start_time || current_time < window.end_time
        };

        if in_window {
            return Err(ApiError::conflict(format!(
                "Deployment frozen: '{}' freeze window is active ({} - {} UTC)",
                window.name, window.start_time, window.end_time
            )));
        }
    }

    Ok(())
}

// -------------------------------------------------------------------------
// Deployment Approval Endpoints
// -------------------------------------------------------------------------

/// Request body for rejecting a deployment
#[derive(Debug, Deserialize, Default)]
pub struct RejectDeployRequest {
    pub reason: Option<String>,
}

/// Approve a pending deployment
/// POST /api/deployments/:id/approve
pub async fn approve_deployment(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(deployment_id): Path<String>,
) -> Result<Json<Deployment>, ApiError> {
    if let Err(e) = validate_uuid(&deployment_id, "deployment_id") {
        return Err(ApiError::validation_field("deployment_id", e));
    }

    // Only admins can approve deployments
    if user.role != "admin" {
        return Err(ApiError::forbidden("Only admins can approve deployments"));
    }

    // Get the deployment
    let deployment = sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = ?")
        .bind(&deployment_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Deployment not found"))?;

    // Must be in pending approval state
    if deployment.approval_status.as_deref() != Some("pending") {
        return Err(ApiError::bad_request("Deployment is not pending approval"));
    }

    // Must still have status 'pending'
    if deployment.status != "pending" {
        return Err(ApiError::bad_request(
            "Deployment is no longer in pending state",
        ));
    }

    let now = chrono::Utc::now().to_rfc3339();

    // Update approval status
    sqlx::query(
        "UPDATE deployments SET approval_status = 'approved', approved_by = ?, approved_at = ? WHERE id = ?",
    )
    .bind(&user.id)
    .bind(&now)
    .bind(&deployment_id)
    .execute(&state.db)
    .await?;

    // Get the app so we can queue the deployment
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&deployment.app_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Queue the deployment job now that it is approved
    if let Err(e) = state.deploy_tx.send((deployment_id.clone(), app)).await {
        tracing::error!("Failed to queue approved deployment: {}", e);
        return Err(ApiError::internal("Failed to queue deployment job"));
    }

    let updated = sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = ?")
        .bind(&deployment_id)
        .fetch_one(&state.db)
        .await?;

    tracing::info!(
        deployment_id = %deployment_id,
        approved_by = %user.id,
        "Deployment approved and queued"
    );

    Ok(Json(updated))
}

/// Reject a pending deployment
/// POST /api/deployments/:id/reject
pub async fn reject_deployment(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(deployment_id): Path<String>,
    body: Option<Json<RejectDeployRequest>>,
) -> Result<Json<Deployment>, ApiError> {
    if let Err(e) = validate_uuid(&deployment_id, "deployment_id") {
        return Err(ApiError::validation_field("deployment_id", e));
    }

    // Only admins can reject deployments
    if user.role != "admin" {
        return Err(ApiError::forbidden("Only admins can reject deployments"));
    }

    // Get the deployment
    let deployment = sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = ?")
        .bind(&deployment_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Deployment not found"))?;

    // Must be in pending approval state
    if deployment.approval_status.as_deref() != Some("pending") {
        return Err(ApiError::bad_request("Deployment is not pending approval"));
    }

    let reason = body
        .and_then(|b| b.0.reason)
        .unwrap_or_else(|| "No reason provided".to_string());

    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"UPDATE deployments
           SET approval_status = 'rejected',
               approved_by = ?,
               approved_at = ?,
               rejection_reason = ?,
               status = 'failed',
               error_message = ?,
               finished_at = ?
           WHERE id = ?"#,
    )
    .bind(&user.id)
    .bind(&now)
    .bind(&reason)
    .bind(format!("Deployment rejected: {}", reason))
    .bind(&now)
    .bind(&deployment_id)
    .execute(&state.db)
    .await?;

    let updated = sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = ?")
        .bind(&deployment_id)
        .fetch_one(&state.db)
        .await?;

    tracing::info!(
        deployment_id = %deployment_id,
        rejected_by = %user.id,
        reason = %reason,
        "Deployment rejected"
    );

    Ok(Json(updated))
}

/// List pending-approval deployments for an app
/// GET /api/apps/:id/deployments/pending
pub async fn list_pending_deployments(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<Deployment>>, ApiError> {
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    let app_exists: Option<(String,)> = sqlx::query_as("SELECT id FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await?;

    if app_exists.is_none() {
        return Err(ApiError::not_found("App not found"));
    }

    let deployments = sqlx::query_as::<_, Deployment>(
        "SELECT * FROM deployments WHERE app_id = ? AND approval_status = 'pending' ORDER BY started_at DESC",
    )
    .bind(&app_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(deployments))
}

// -------------------------------------------------------------------------
// Freeze Window Endpoints
// -------------------------------------------------------------------------

/// Request body for creating a freeze window
#[derive(Debug, Deserialize)]
pub struct CreateFreezeWindowRequest {
    pub name: String,
    /// Start time in HH:MM UTC format
    pub start_time: String,
    /// End time in HH:MM UTC format
    pub end_time: String,
    /// Comma-separated days of week (0=Sun, ..., 6=Sat). Default: all days
    pub days_of_week: Option<String>,
    #[serde(default = "default_is_active")]
    pub is_active: bool,
}

fn default_is_active() -> bool {
    true
}

/// List freeze windows for an app
/// GET /api/apps/:id/freeze-windows
pub async fn list_freeze_windows(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<DeploymentFreezeWindow>>, ApiError> {
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    let app_exists: Option<(String,)> = sqlx::query_as("SELECT id FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await?;

    if app_exists.is_none() {
        return Err(ApiError::not_found("App not found"));
    }

    let windows = sqlx::query_as::<_, DeploymentFreezeWindow>(
        "SELECT * FROM deployment_freeze_windows WHERE app_id = ? ORDER BY created_at DESC",
    )
    .bind(&app_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(windows))
}

/// Create a freeze window for an app
/// POST /api/apps/:id/freeze-windows
pub async fn create_freeze_window(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(app_id): Path<String>,
    Json(req): Json<CreateFreezeWindowRequest>,
) -> Result<(StatusCode, Json<DeploymentFreezeWindow>), ApiError> {
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Only admins can create freeze windows
    if user.role != "admin" {
        return Err(ApiError::forbidden("Only admins can create freeze windows"));
    }

    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Validate time format (HH:MM)
    let time_re = regex::Regex::new(r"^\d{2}:\d{2}$").unwrap();
    if !time_re.is_match(&req.start_time) || !time_re.is_match(&req.end_time) {
        return Err(ApiError::bad_request(
            "start_time and end_time must be in HH:MM format (UTC)",
        ));
    }

    let window_id = Uuid::new_v4().to_string();
    let days_of_week = req
        .days_of_week
        .unwrap_or_else(|| "0,1,2,3,4,5,6".to_string());
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO deployment_freeze_windows
          (id, app_id, team_id, name, start_time, end_time, days_of_week, is_active, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&window_id)
    .bind(&app_id)
    .bind(&app.team_id)
    .bind(&req.name)
    .bind(&req.start_time)
    .bind(&req.end_time)
    .bind(&days_of_week)
    .bind(req.is_active as i32)
    .bind(&now)
    .execute(&state.db)
    .await?;

    let window = sqlx::query_as::<_, DeploymentFreezeWindow>(
        "SELECT * FROM deployment_freeze_windows WHERE id = ?",
    )
    .bind(&window_id)
    .fetch_one(&state.db)
    .await?;

    Ok((StatusCode::CREATED, Json(window)))
}

/// Delete a freeze window
/// DELETE /api/apps/:id/freeze-windows/:window_id
pub async fn delete_freeze_window(
    State(state): State<Arc<AppState>>,
    user: User,
    Path((app_id, window_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }
    if let Err(e) = validate_uuid(&window_id, "window_id") {
        return Err(ApiError::validation_field("window_id", e));
    }

    // Only admins can delete freeze windows
    if user.role != "admin" {
        return Err(ApiError::forbidden("Only admins can delete freeze windows"));
    }

    let result =
        sqlx::query("DELETE FROM deployment_freeze_windows WHERE id = ? AND app_id = ?")
            .bind(&window_id)
            .bind(&app_id)
            .execute(&state.db)
            .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Freeze window not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}
