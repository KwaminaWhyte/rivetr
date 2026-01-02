use axum::{
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::crypto;
use crate::db::{actions, resource_types, App, Deployment, DeploymentLog, User};
use crate::engine::{detect_build_type, extract_zip_and_find_root, run_rollback, BuildDetectionResult};
use crate::proxy::Backend;
use crate::runtime::ContainerStats;
use crate::AppState;

use super::audit::{audit_log, extract_client_ip};
use super::error::ApiError;
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
    let (upload_source_path, existing_image_tag): (Option<String>, Option<String>) = if is_upload_app {
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

    // For upload apps with existing image, store the image tag to reuse
    // commit_sha stores source path for rebuild, image_tag stores image for restart
    sqlx::query(
        r#"
        INSERT INTO deployments (id, app_id, status, started_at, commit_sha, image_tag)
        VALUES (?, ?, 'pending', ?, ?, ?)
        "#,
    )
    .bind(&deployment_id)
    .bind(&app_id)
    .bind(&now)
    .bind(&upload_source_path)
    .bind(&existing_image_tag)
    .execute(&state.db)
    .await?;

    // Queue the deployment job
    if let Err(e) = state.deploy_tx.send((deployment_id.clone(), app.clone())).await {
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
        })),
    )
    .await;

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
    let (total,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM deployments WHERE app_id = ?",
    )
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
    let current_deployment = sqlx::query_as::<_, Deployment>(
        "SELECT * FROM deployments WHERE id = ?"
    )
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
        match run_rollback(&db, runtime, &rollback_id_clone, &target_deployment_clone, &app_clone, encryption_key.as_ref()).await {
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
        "#
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
    let stats = state
        .runtime
        .stats(&container_id)
        .await
        .map_err(|e| {
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
        ApiError::bad_request("No ZIP file provided. Include a 'file' or 'zip' field in the multipart form")
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
    if let Err(e) = state.deploy_tx.send((deployment_id.clone(), app.clone())).await {
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

    Ok((StatusCode::ACCEPTED, Json(UploadDeployResponse {
        deployment,
        detected_build_type: detection,
    })))
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

    let zip_data = zip_data.ok_or_else(|| {
        ApiError::bad_request("No ZIP file provided")
    })?;

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

    let detection = result.map_err(|e| ApiError::bad_request(format!("Detection failed: {}", e)))?;

    Ok(Json(detection))
}
