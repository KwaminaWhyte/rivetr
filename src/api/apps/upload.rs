use axum::{
    extract::{Multipart, Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{actions, resource_types, App, TeamAuditAction, TeamAuditResourceType, User};
use crate::engine::{detect_build_type, extract_zip_and_find_root};
use crate::AppState;

use super::super::audit::{audit_log, extract_client_ip};
use super::super::error::ApiError;
use super::super::teams::log_team_audit;
use super::super::validation::{validate_app_name, validate_uuid};
use super::{UploadAppConfig, UploadAppResponse};

/// Create an app and deploy from uploaded ZIP file
/// POST /api/projects/:project_id/apps/upload
pub async fn upload_create_app(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<UploadAppResponse>), ApiError> {
    // Validate project_id format
    if let Err(e) = validate_uuid(&project_id, "project_id") {
        return Err(ApiError::validation_field("project_id", e));
    }

    // Verify project exists
    let _project: Option<(String,)> = sqlx::query_as("SELECT id FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_optional(&state.db)
        .await?;

    if _project.is_none() {
        return Err(ApiError::not_found("Project not found"));
    }

    // Parse multipart form data
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut config: Option<UploadAppConfig> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(format!("Failed to read multipart: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            file_name = field.file_name().map(|s| s.to_string());
            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::bad_request(format!("Failed to read file: {}", e)))?;
            file_data = Some(data.to_vec());
        } else if name == "config" {
            let text = field
                .text()
                .await
                .map_err(|e| ApiError::bad_request(format!("Failed to read config: {}", e)))?;
            config = Some(
                serde_json::from_str(&text)
                    .map_err(|e| ApiError::bad_request(format!("Invalid config JSON: {}", e)))?,
            );
        }
    }

    let file_data = file_data.ok_or_else(|| ApiError::bad_request("No file uploaded"))?;
    let config = config.ok_or_else(|| ApiError::bad_request("No config provided"))?;

    // Validate file is a ZIP
    if let Some(ref name) = file_name {
        if !name.to_lowercase().ends_with(".zip") {
            return Err(ApiError::bad_request("Only ZIP files are supported"));
        }
    }

    // Validate app name
    if let Err(e) = validate_app_name(&config.name) {
        return Err(ApiError::validation_field("name", e));
    }

    // Create a unique deployment ID for the temp directory
    let deployment_id = Uuid::new_v4().to_string();
    let work_dir = std::env::temp_dir().join(format!("rivetr-upload-{}", deployment_id));

    // Extract ZIP and find project root
    let project_root = extract_zip_and_find_root(&file_data, &work_dir)
        .await
        .map_err(|e| ApiError::bad_request(format!("Failed to extract ZIP: {}", e)))?;

    // Auto-detect build type
    let detected = detect_build_type(&project_root)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to detect build type: {}", e)))?;
    tracing::info!(
        build_type = %detected.build_type,
        confidence = %detected.confidence,
        detected_from = %detected.detected_from,
        "Build type detected for uploaded project"
    );

    // Determine final build type (use override or detected)
    let build_type = config
        .build_type
        .clone()
        .unwrap_or_else(|| detected.build_type.to_string());
    let publish_directory = config
        .publish_directory
        .clone()
        .or_else(|| detected.publish_directory.clone());

    // Clone detected for audit log before moving
    let detected_from_log = detected.detected_from.clone();

    // Create the app
    let app_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO apps (
            id, name, git_url, branch, dockerfile, domain, port, healthcheck,
            memory_limit, cpu_limit, environment, project_id, build_type,
            publish_directory, deployment_source, created_at, updated_at
        ) VALUES (?, ?, '', 'main', 'Dockerfile', ?, ?, ?, ?, ?, ?, ?, ?, ?, 'upload', ?, ?)
        "#,
    )
    .bind(&app_id)
    .bind(&config.name)
    .bind(&config.domain)
    .bind(config.port as i32)
    .bind(&config.healthcheck)
    .bind(&config.memory_limit)
    .bind(&config.cpu_limit)
    .bind(&config.environment)
    .bind(&project_id)
    .bind(&build_type)
    .bind(&publish_directory)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create app: {}", e);
        if e.to_string().contains("UNIQUE constraint failed") {
            ApiError::conflict("An app with this name already exists")
        } else {
            ApiError::database("Failed to create app")
        }
    })?;

    // Create deployment record
    sqlx::query(
        r#"
        INSERT INTO deployments (id, app_id, status, started_at, commit_sha)
        VALUES (?, ?, 'pending', ?, ?)
        "#,
    )
    .bind(&deployment_id)
    .bind(&app_id)
    .bind(&now)
    .bind(project_root.to_string_lossy().to_string()) // Store source path in commit_sha
    .execute(&state.db)
    .await?;

    // Fetch the created app
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_one(&state.db)
        .await?;

    // Queue the deployment
    if let Err(e) = state
        .deploy_tx
        .send((deployment_id.clone(), app.clone()))
        .await
    {
        tracing::error!("Failed to queue deployment: {}", e);
        return Err(ApiError::internal("Failed to queue deployment"));
    }

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::APP_CREATE,
        resource_types::APP,
        Some(&app.id),
        Some(&app.name),
        Some(&user.id),
        ip.as_deref(),
        Some(serde_json::json!({
            "source": "upload",
            "build_type": build_type,
            "detected_from": detected_from_log
        })),
    )
    .await;

    // Log team audit event if app belongs to a team
    if let Some(ref team_id) = app.team_id {
        if let Err(e) = log_team_audit(
            &state.db,
            team_id,
            Some(&user.id),
            TeamAuditAction::AppCreated,
            TeamAuditResourceType::App,
            Some(&app.id),
            Some(serde_json::json!({
                "app_name": app.name,
                "source": "upload",
                "build_type": build_type,
            })),
        )
        .await
        {
            tracing::warn!("Failed to log team audit event: {}", e);
        }
    }

    tracing::info!(
        app_id = %app_id,
        deployment_id = %deployment_id,
        build_type = %build_type,
        "App created from upload and deployment queued"
    );

    Ok((
        StatusCode::CREATED,
        Json(UploadAppResponse {
            app,
            deployment_id,
            detected_build_type: detected,
        }),
    ))
}
