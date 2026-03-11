mod build;
mod clone;
mod rollback;
mod start;

pub use rollback::run_rollback;

use anyhow::{Context, Result};
use std::sync::Arc;

use crate::db::App;
use crate::runtime::{ContainerRuntime, RegistryAuth};
use crate::DbPool;

use super::{add_deployment_log, update_deployment_status, BuildLimits, KEY_LENGTH};

/// Information about a successfully deployed container
pub struct DeploymentResult {
    pub container_id: String,
    pub image_tag: String,
    pub port: Option<u16>,
    /// If this deployment was an auto-rollback, this contains the ID of the failed deployment
    pub auto_rollback_from: Option<String>,
}

/// Error returned when health check fails and auto-rollback is triggered
#[derive(Debug)]
pub struct AutoRollbackTriggered {
    pub failed_deployment_id: String,
    pub rollback_deployment_id: String,
    pub target_deployment_id: String,
}

impl std::fmt::Display for AutoRollbackTriggered {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Health check failed, auto-rollback triggered to deployment {}",
            self.target_deployment_id
        )
    }
}

impl std::error::Error for AutoRollbackTriggered {}

/// Handle registry-based deployment (pull pre-built image)
async fn run_registry_deployment(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    deployment_id: &str,
    app: &App,
) -> Result<String> {
    let image_ref = app
        .get_full_image_reference()
        .ok_or_else(|| anyhow::anyhow!("Docker image not configured"))?;

    add_deployment_log(
        db,
        deployment_id,
        "info",
        &format!("Pulling image from registry: {}", image_ref),
    )
    .await?;
    update_deployment_status(db, deployment_id, "building", None).await?;

    // Set up registry authentication if provided
    let auth = if app.registry_username.is_some() || app.registry_password.is_some() {
        Some(RegistryAuth::new(
            app.registry_username.clone(),
            app.registry_password.clone(),
            app.registry_url.clone(),
        ))
    } else {
        None
    };

    runtime
        .pull_image(&image_ref, auth.as_ref())
        .await
        .context("Failed to pull image from registry")?;

    add_deployment_log(db, deployment_id, "info", "Image pulled successfully").await?;

    Ok(image_ref)
}

/// Handle upload-based deployment (source already extracted)
async fn run_upload_deployment(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    deployment_id: &str,
    app: &App,
    source_path: &str,
    build_limits: &BuildLimits,
) -> Result<String> {
    use std::path::PathBuf;

    let work_dir = PathBuf::from(source_path);

    add_deployment_log(db, deployment_id, "info", "Using uploaded source files...").await?;
    update_deployment_status(db, deployment_id, "building", None).await?;

    // Determine the actual build path (consider base_directory)
    let build_path = if let Some(ref base_dir) = app.base_directory {
        if !base_dir.is_empty() {
            work_dir.join(base_dir)
        } else {
            work_dir.clone()
        }
    } else {
        work_dir.clone()
    };

    let image_tag =
        build::build_upload_image(db, runtime, deployment_id, app, &build_path, build_limits)
            .await?;

    // Cleanup work directory after build
    let _ = tokio::fs::remove_dir_all(&work_dir).await;

    Ok(image_tag)
}

/// Handle git-based deployment (clone and build)
async fn run_git_deployment(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    deployment_id: &str,
    app: &App,
    build_limits: &BuildLimits,
) -> Result<String> {
    use std::path::PathBuf;

    let work_dir = std::env::temp_dir().join(format!("rivetr-{}", deployment_id));

    // Check if this deployment targets a specific commit or tag
    let deployment_target: Option<(Option<String>, Option<String>)> =
        sqlx::query_as("SELECT commit_sha, git_tag FROM deployments WHERE id = ?")
            .bind(deployment_id)
            .fetch_optional(db)
            .await?;

    let (target_commit_sha, target_git_tag) = deployment_target
        .map(|(sha, tag)| {
            // Filter out upload source paths from commit_sha
            let sha = sha.filter(|s| !s.contains("rivetr-upload-"));
            (sha, tag)
        })
        .unwrap_or((None, None));

    let needs_full_clone = target_commit_sha.is_some() || target_git_tag.is_some();

    // Step 1: Clone
    add_deployment_log(
        db,
        deployment_id,
        "info",
        &format!("Cloning repository: {}", app.git_url),
    )
    .await?;
    update_deployment_status(db, deployment_id, "cloning", None).await?;

    // Get SSH key if configured for this app
    let ssh_key = clone::get_ssh_key_for_app(db, app).await?;

    // Build the effective clone URL (inject OAuth/PAT token for HTTPS repos)
    let clone_url = clone::get_authenticated_url(db, app).await?;

    if needs_full_clone {
        // Need full clone for specific commit/tag checkout
        clone::clone_repository_full(&clone_url, &app.branch, &work_dir, ssh_key.as_ref())
            .await?;
    } else {
        clone::clone_repository(&clone_url, &app.branch, &work_dir, ssh_key.as_ref()).await?;
    }
    add_deployment_log(db, deployment_id, "info", "Repository cloned successfully").await?;

    // Step 1b: Checkout specific commit or tag if requested
    if let Some(ref sha) = target_commit_sha {
        add_deployment_log(
            db,
            deployment_id,
            "info",
            &format!("Checking out commit: {}", sha),
        )
        .await?;
        clone::git_checkout(&work_dir, sha).await?;
        add_deployment_log(db, deployment_id, "info", "Commit checked out successfully").await?;
    } else if let Some(ref tag) = target_git_tag {
        add_deployment_log(
            db,
            deployment_id,
            "info",
            &format!("Checking out tag: {}", tag),
        )
        .await?;
        clone::git_checkout(&work_dir, &format!("tags/{}", tag)).await?;
        add_deployment_log(db, deployment_id, "info", "Tag checked out successfully").await?;
    }

    // Update deployment record with actual commit SHA and message from the checked-out HEAD
    if let Ok(commit_info) = clone::get_git_commit_info(&work_dir).await {
        sqlx::query("UPDATE deployments SET commit_sha = ?, commit_message = ? WHERE id = ?")
            .bind(&commit_info.0)
            .bind(&commit_info.1)
            .bind(deployment_id)
            .execute(db)
            .await?;
    }

    // Step 2: Build
    update_deployment_status(db, deployment_id, "building", None).await?;

    // Determine the actual build path (consider base_directory)
    let build_path: PathBuf = if let Some(ref base_dir) = app.base_directory {
        if !base_dir.is_empty() {
            work_dir.join(base_dir)
        } else {
            work_dir.clone()
        }
    } else {
        work_dir.clone()
    };

    let image_tag =
        build::build_git_image(db, runtime, deployment_id, app, &build_path, build_limits).await?;

    // Cleanup work directory
    let _ = tokio::fs::remove_dir_all(&work_dir).await;

    Ok(image_tag)
}

pub async fn run_deployment(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    deployment_id: &str,
    app: &App,
    build_limits: &BuildLimits,
    encryption_key: Option<&[u8; KEY_LENGTH]>,
) -> Result<DeploymentResult> {
    // Log remote deployment intent if a server is assigned to this app
    if let Some(ref server_id) = app.server_id {
        add_deployment_log(
            db,
            deployment_id,
            "info",
            &format!(
                "Remote deployment to server {} requested. Falling back to local deployment for MVP.",
                server_id
            ),
        )
        .await?;
    }

    // Check if this is an upload-based deployment by looking at the deployment record
    // Upload deployments store the source path in commit_sha, or existing image_tag for restart
    let deployment: Option<(Option<String>, Option<String>)> =
        sqlx::query_as("SELECT commit_sha, image_tag FROM deployments WHERE id = ?")
            .bind(deployment_id)
            .fetch_optional(db)
            .await?;

    let (upload_source_path, existing_image_tag) = deployment
        .map(|(commit_sha, image_tag)| {
            let source_path = commit_sha.filter(|path| path.contains("rivetr-upload-"));
            (source_path, image_tag)
        })
        .unwrap_or((None, None));

    // Determine the image to use based on deployment source
    let image_tag = if app.uses_registry_image() {
        // Registry-based deployment: pull pre-built image
        run_registry_deployment(db, runtime.clone(), deployment_id, app).await?
    } else if let Some(ref existing_tag) = existing_image_tag {
        // Restart from existing image (for upload apps without source)
        add_deployment_log(
            db,
            deployment_id,
            "info",
            &format!("Restarting from existing image: {}", existing_tag),
        )
        .await?;
        update_deployment_status(db, deployment_id, "building", None).await?;
        add_deployment_log(
            db,
            deployment_id,
            "info",
            "Skipping build - using existing image",
        )
        .await?;
        existing_tag.clone()
    } else if let Some(source_path) = upload_source_path {
        // Upload-based deployment: use pre-extracted source
        let tag = run_upload_deployment(
            db,
            runtime.clone(),
            deployment_id,
            app,
            &source_path,
            build_limits,
        )
        .await?;
        // Optionally push to registry after upload build
        if let Err(e) =
            build::push_image_to_registry(db, deployment_id, app, &tag, encryption_key).await
        {
            add_deployment_log(
                db,
                deployment_id,
                "warn",
                &format!("Registry push failed (non-fatal): {}", e),
            )
            .await?;
        }
        tag
    } else {
        // Git-based deployment: clone and build
        let tag = run_git_deployment(db, runtime.clone(), deployment_id, app, build_limits).await?;
        // Optionally push to registry after git build
        if let Err(e) =
            build::push_image_to_registry(db, deployment_id, app, &tag, encryption_key).await
        {
            add_deployment_log(
                db,
                deployment_id,
                "warn",
                &format!("Registry push failed (non-fatal): {}", e),
            )
            .await?;
        }
        tag
    };

    // Start container, health check, and finalize
    start::start_container(db, runtime, deployment_id, app, image_tag, encryption_key).await
}
