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
    /// Old container IDs that should be stopped AFTER proxy routes are updated (zero-downtime swap)
    pub old_container_ids: Vec<String>,
}

/// Error returned when health check fails and auto-rollback is triggered
#[derive(Debug)]
pub struct AutoRollbackTriggered {
    pub failed_deployment_id: String,
    pub rollback_deployment_id: String,
    pub target_deployment_id: String,
    /// Old container IDs to stop AFTER proxy routes are updated to the rollback container
    pub old_container_ids: Vec<String>,
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

/// Handle inline Dockerfile deployment (no git clone — Dockerfile content stored in DB)
async fn run_inline_dockerfile_deployment(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    deployment_id: &str,
    app: &App,
    dockerfile_content: &str,
    build_limits: &BuildLimits,
    encryption_key: Option<&[u8; KEY_LENGTH]>,
) -> Result<String> {
    add_deployment_log(
        db,
        deployment_id,
        "info",
        "Using inline Dockerfile (no git clone required)",
    )
    .await?;
    update_deployment_status(db, deployment_id, "building", None).await?;

    // Create a temporary directory and write the Dockerfile into it
    let temp_dir = tempfile::TempDir::new()
        .context("Failed to create temp directory for inline Dockerfile")?;
    let dockerfile_path = temp_dir.path().join("Dockerfile");
    tokio::fs::write(&dockerfile_path, dockerfile_content)
        .await
        .context("Failed to write inline Dockerfile to temp directory")?;

    add_deployment_log(
        db,
        deployment_id,
        "info",
        "Inline Dockerfile written to build context",
    )
    .await?;

    let image_tag = build::build_git_image(
        db,
        runtime,
        deployment_id,
        app,
        temp_dir.path(),
        build_limits,
        encryption_key,
    )
    .await?;

    // temp_dir is dropped here, cleaning up the temp directory
    Ok(image_tag)
}

/// Handle git-based deployment (clone and build)
async fn run_git_deployment(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    deployment_id: &str,
    app: &App,
    build_limits: &BuildLimits,
    encryption_key: Option<&[u8; KEY_LENGTH]>,
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

    // Build the effective clone URL (inject OAuth/PAT/GitHub App token for HTTPS repos)
    let clone_url = clone::get_authenticated_url(db, app, encryption_key).await?;

    let clone_opts = clone::CloneOptions {
        shallow: app.shallow_clone != 0,
        submodules: app.git_submodules != 0,
        lfs: app.git_lfs != 0,
    };

    if needs_full_clone {
        // Need full clone for specific commit/tag checkout
        clone::clone_repository_full(&clone_url, &app.branch, &work_dir, ssh_key.as_ref()).await?;
    } else {
        clone::clone_repository(
            &clone_url,
            &app.branch,
            &work_dir,
            ssh_key.as_ref(),
            &clone_opts,
        )
        .await?;
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

    // Step 1c: Apply deployment patches (file injection)
    {
        let patches = sqlx::query_as::<_, crate::db::AppPatch>(
            "SELECT id, app_id, file_path, content, operation, is_enabled, created_at, updated_at \
             FROM app_patches WHERE app_id = ? ORDER BY created_at ASC",
        )
        .bind(&app.id)
        .fetch_all(db)
        .await
        .unwrap_or_default();

        for patch in patches.iter().filter(|p| p.is_enabled()) {
            let target = work_dir.join(&patch.file_path);
            if let Some(parent) = target.parent() {
                if let Err(e) = tokio::fs::create_dir_all(parent).await {
                    add_deployment_log(
                        db,
                        deployment_id,
                        "warn",
                        &format!(
                            "Could not create parent dir for patch {}: {}",
                            patch.file_path, e
                        ),
                    )
                    .await?;
                    continue;
                }
            }

            match patch.operation.as_str() {
                "create" => {
                    if let Err(e) = tokio::fs::write(&target, &patch.content).await {
                        add_deployment_log(
                            db,
                            deployment_id,
                            "warn",
                            &format!("Patch write failed for {}: {}", patch.file_path, e),
                        )
                        .await?;
                    } else {
                        add_deployment_log(
                            db,
                            deployment_id,
                            "info",
                            &format!("Patch applied (create): {}", patch.file_path),
                        )
                        .await?;
                    }
                }
                "append" => {
                    use tokio::io::AsyncWriteExt;
                    match tokio::fs::OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open(&target)
                        .await
                    {
                        Ok(mut f) => {
                            if let Err(e) = f.write_all(patch.content.as_bytes()).await {
                                add_deployment_log(
                                    db,
                                    deployment_id,
                                    "warn",
                                    &format!("Patch append failed for {}: {}", patch.file_path, e),
                                )
                                .await?;
                            } else {
                                add_deployment_log(
                                    db,
                                    deployment_id,
                                    "info",
                                    &format!("Patch applied (append): {}", patch.file_path),
                                )
                                .await?;
                            }
                        }
                        Err(e) => {
                            add_deployment_log(
                                db,
                                deployment_id,
                                "warn",
                                &format!("Patch open failed for {}: {}", patch.file_path, e),
                            )
                            .await?;
                        }
                    }
                }
                "delete" => {
                    let _ = tokio::fs::remove_file(&target).await;
                    add_deployment_log(
                        db,
                        deployment_id,
                        "info",
                        &format!("Patch applied (delete): {}", patch.file_path),
                    )
                    .await?;
                }
                _ => {}
            }
        }
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

    let image_tag = build::build_git_image(
        db,
        runtime,
        deployment_id,
        app,
        &build_path,
        build_limits,
        encryption_key,
    )
    .await?;

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
    // Check if the deployment was cancelled while it was queued (before the pipeline picked it up)
    let current_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM deployments WHERE id = ?")
            .bind(deployment_id)
            .fetch_optional(db)
            .await?;
    if current_status.as_deref() == Some("cancelled") {
        anyhow::bail!("Deployment was cancelled before it could start");
    }

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

    // Determine the image to use based on deployment source.
    // `remote_image_tag` is Some(...) when the image was pushed to a registry —
    // after start_container stores the local image tag we overwrite it with the
    // remote reference so rollbacks can pull from registry.
    let (image_tag, remote_image_tag): (String, Option<String>) = if app.uses_registry_image() {
        // Registry-based deployment: pull pre-built image (no push needed)
        let tag = run_registry_deployment(db, runtime.clone(), deployment_id, app).await?;
        (tag, None)
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
        (existing_tag.clone(), None)
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
        // Optionally push to registry; capture remote tag for image_tag update
        let remote =
            match build::push_image_to_registry(db, deployment_id, app, &tag, encryption_key).await
            {
                Ok(rt) => rt,
                Err(e) => {
                    add_deployment_log(
                        db,
                        deployment_id,
                        "warn",
                        &format!("Registry push failed (non-fatal): {}", e),
                    )
                    .await?;
                    None
                }
            };
        (tag, remote)
    } else if let Some(ref dockerfile_content) =
        app.inline_dockerfile.clone().filter(|s| !s.is_empty())
    {
        // Inline Dockerfile deployment: write to temp dir and build (no git clone)
        let tag = run_inline_dockerfile_deployment(
            db,
            runtime.clone(),
            deployment_id,
            app,
            dockerfile_content,
            build_limits,
            encryption_key,
        )
        .await?;
        // Optionally push to registry; capture remote tag for image_tag update
        let remote =
            match build::push_image_to_registry(db, deployment_id, app, &tag, encryption_key).await
            {
                Ok(rt) => rt,
                Err(e) => {
                    add_deployment_log(
                        db,
                        deployment_id,
                        "warn",
                        &format!("Registry push failed (non-fatal): {}", e),
                    )
                    .await?;
                    None
                }
            };
        (tag, remote)
    } else {
        // Git-based deployment: clone and build
        let tag = run_git_deployment(
            db,
            runtime.clone(),
            deployment_id,
            app,
            build_limits,
            encryption_key,
        )
        .await?;
        // Optionally push to registry; capture remote tag for image_tag update
        let remote =
            match build::push_image_to_registry(db, deployment_id, app, &tag, encryption_key).await
            {
                Ok(rt) => rt,
                Err(e) => {
                    add_deployment_log(
                        db,
                        deployment_id,
                        "warn",
                        &format!("Registry push failed (non-fatal): {}", e),
                    )
                    .await?;
                    None
                }
            };
        (tag, remote)
    };

    // Start container, health check, and finalize
    let result =
        start::start_container(db, runtime, deployment_id, app, image_tag, encryption_key).await?;

    // If the image was pushed to a registry, update the deployment's image_tag to the remote
    // reference. start_container stored the local image name, but rollbacks need the registry URL.
    if let Some(ref rmt) = remote_image_tag {
        let _ = sqlx::query("UPDATE deployments SET image_tag = ? WHERE id = ?")
            .bind(rmt)
            .bind(deployment_id)
            .execute(db)
            .await;
    }

    Ok(result)
}
