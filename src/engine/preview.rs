//! Preview deployment engine for PR-based preview environments.
//!
//! This module handles the full lifecycle of preview deployments:
//! - Generating unique preview domains
//! - Building and deploying preview containers
//! - Cleaning up previews when PRs are closed

use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::crypto;
use crate::db::{App, PreviewDeployment, PreviewDeploymentStatus, SshKey};
use crate::proxy::{Backend, RouteTable};
use crate::runtime::{BuildContext, ContainerRuntime, RunConfig};
use crate::DbPool;

/// Key length for AES-256 encryption
const KEY_LENGTH: usize = 32;

/// Information about a preview deployment for webhook processing
#[derive(Debug, Clone)]
pub struct PreviewDeploymentInfo {
    pub app_id: String,
    pub pr_number: i64,
    pub pr_title: Option<String>,
    pub pr_source_branch: String,
    pub pr_target_branch: String,
    pub pr_author: Option<String>,
    pub pr_url: Option<String>,
    pub commit_sha: Option<String>,
    pub commit_message: Option<String>,
    pub provider_type: String,
    pub repo_full_name: String,
}

/// Generate a unique preview domain for a PR
///
/// Format: pr-{pr_number}.{app_name}.{base_domain}
/// Example: pr-123.myapp.preview.example.com
pub fn generate_preview_domain(app_name: &str, pr_number: i64, base_domain: &str) -> String {
    // Sanitize app name for DNS (lowercase, alphanumeric and hyphens only)
    let sanitized_name: String = app_name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    // Limit app name length to avoid DNS label length issues (max 63 chars per label)
    let truncated_name = if sanitized_name.len() > 30 {
        &sanitized_name[..30]
    } else {
        &sanitized_name
    };

    format!("pr-{}.{}.{}", pr_number, truncated_name, base_domain)
}

/// Generate a unique container name for a preview deployment
pub fn generate_preview_container_name(app_name: &str, pr_number: i64) -> String {
    let sanitized_name: String = app_name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    format!("rivetr-preview-{}-pr-{}", sanitized_name, pr_number)
}

/// Update preview deployment status in the database
pub async fn update_preview_status(
    db: &DbPool,
    preview_id: &str,
    status: PreviewDeploymentStatus,
    error: Option<&str>,
) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let status_str = status.to_string();

    if status == PreviewDeploymentStatus::Running
        || status == PreviewDeploymentStatus::Failed
        || status == PreviewDeploymentStatus::Closed
    {
        let closed_at = if status == PreviewDeploymentStatus::Closed {
            Some(now.clone())
        } else {
            None
        };

        sqlx::query(
            "UPDATE preview_deployments SET status = ?, error_message = ?, updated_at = ?, closed_at = ? WHERE id = ?",
        )
        .bind(&status_str)
        .bind(error)
        .bind(&now)
        .bind(closed_at)
        .bind(preview_id)
        .execute(db)
        .await?;
    } else {
        sqlx::query(
            "UPDATE preview_deployments SET status = ?, error_message = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&status_str)
        .bind(error)
        .bind(&now)
        .bind(preview_id)
        .execute(db)
        .await?;
    }

    Ok(())
}

/// Run a preview deployment for a PR
///
/// This function:
/// 1. Clones the PR branch
/// 2. Builds the Docker image
/// 3. Starts the container with resource limits
/// 4. Updates the proxy route table
pub async fn run_preview_deployment(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    routes: Arc<ArcSwap<RouteTable>>,
    preview: &PreviewDeployment,
    app: &App,
    encryption_key: Option<&[u8; KEY_LENGTH]>,
) -> Result<()> {
    let preview_id = &preview.id;
    info!(
        preview_id = %preview_id,
        app = %app.name,
        pr = preview.pr_number,
        "Starting preview deployment"
    );

    // Step 1: Clone the PR source branch
    update_preview_status(db, preview_id, PreviewDeploymentStatus::Cloning, None).await?;

    let work_dir = std::env::temp_dir().join(format!("rivetr-preview-{}", preview_id));

    // Get SSH key if configured
    let ssh_key = get_ssh_key_for_app(db, app).await?;

    if let Err(e) = clone_repository(
        &app.git_url,
        &preview.pr_source_branch,
        &work_dir,
        ssh_key.as_ref(),
    )
    .await
    {
        error!(error = %e, "Failed to clone repository for preview");
        update_preview_status(
            db,
            preview_id,
            PreviewDeploymentStatus::Failed,
            Some(&format!("Clone failed: {}", e)),
        )
        .await?;
        return Err(e);
    }

    // Step 2: Build the Docker image
    update_preview_status(db, preview_id, PreviewDeploymentStatus::Building, None).await?;

    let build_path = if let Some(ref base_dir) = app.base_directory {
        if !base_dir.is_empty() {
            work_dir.join(base_dir)
        } else {
            work_dir.clone()
        }
    } else {
        work_dir.clone()
    };

    let dockerfile = app
        .dockerfile_path
        .as_ref()
        .filter(|p| !p.is_empty())
        .cloned()
        .unwrap_or_else(|| app.dockerfile.clone());

    let image_tag = format!(
        "rivetr-preview-{}-pr-{}:{}",
        app.name, preview.pr_number, preview_id
    );

    let build_ctx = BuildContext {
        path: build_path.to_string_lossy().to_string(),
        dockerfile,
        tag: image_tag.clone(),
        build_args: vec![],
        build_target: app.build_target.clone(),
        custom_options: app.custom_docker_options.clone(),
        // Use reduced resource limits for preview builds
        cpu_limit: Some("1".to_string()),
        memory_limit: Some("1g".to_string()),
    };

    if let Err(e) = runtime.build(&build_ctx).await {
        error!(error = %e, "Failed to build preview image");
        update_preview_status(
            db,
            preview_id,
            PreviewDeploymentStatus::Failed,
            Some(&format!("Build failed: {}", e)),
        )
        .await?;
        let _ = tokio::fs::remove_dir_all(&work_dir).await;
        return Err(e.into());
    }

    // Cleanup work directory
    let _ = tokio::fs::remove_dir_all(&work_dir).await;

    // Step 3: Stop old preview container if exists
    let container_name = generate_preview_container_name(&app.name, preview.pr_number);
    let _ = runtime.stop(&container_name).await;
    let _ = runtime.remove(&container_name).await;

    // Step 4: Start the preview container
    update_preview_status(db, preview_id, PreviewDeploymentStatus::Starting, None).await?;

    // Get env vars from database (same as production app)
    let raw_env_vars =
        sqlx::query_as::<_, (String, String)>("SELECT key, value FROM env_vars WHERE app_id = ?")
            .bind(&app.id)
            .fetch_all(db)
            .await
            .unwrap_or_default();

    // Decrypt env var values
    let mut env_vars: Vec<(String, String)> = raw_env_vars
        .into_iter()
        .map(|(key, value)| {
            let decrypted =
                crypto::decrypt_if_encrypted(&value, encryption_key).unwrap_or_else(|e| {
                    warn!("Failed to decrypt env var {}: {}", key, e);
                    value
                });
            (key, decrypted)
        })
        .collect();

    // Automatically set PORT environment variable if not already set
    // This is a common pattern in PaaS systems (Heroku, Railway, etc.)
    if !env_vars.iter().any(|(k, _)| k == "PORT") {
        env_vars.push(("PORT".to_string(), app.port.to_string()));
    }

    // Use preview-specific resource limits (lower than production)
    let memory_limit = preview
        .memory_limit
        .clone()
        .or_else(|| Some("256m".to_string()));
    let cpu_limit = preview
        .cpu_limit
        .clone()
        .or_else(|| Some("0.5".to_string()));

    let run_config = RunConfig {
        image: image_tag.clone(),
        name: container_name.clone(),
        port: app.port as u16,
        env: env_vars,
        memory_limit,
        cpu_limit,
        port_mappings: vec![],
        network_aliases: vec![],
        extra_hosts: vec![],
        labels: std::collections::HashMap::from([
            ("rivetr.preview".to_string(), "true".to_string()),
            ("rivetr.app".to_string(), app.id.clone()),
            ("rivetr.pr".to_string(), preview.pr_number.to_string()),
        ]),
        binds: vec![],
    };

    let container_id = match runtime.run(&run_config).await {
        Ok(id) => id,
        Err(e) => {
            error!(error = %e, "Failed to start preview container");
            update_preview_status(
                db,
                preview_id,
                PreviewDeploymentStatus::Failed,
                Some(&format!("Container start failed: {}", e)),
            )
            .await?;
            return Err(e.into());
        }
    };

    // Get container port
    let container_info = runtime.inspect(&container_id).await?;
    let port = container_info.port;

    // Update preview deployment with container info
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        UPDATE preview_deployments
        SET container_id = ?, container_name = ?, image_tag = ?, port = ?,
            status = 'running', updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&container_id)
    .bind(&container_name)
    .bind(&image_tag)
    .bind(port.map(|p| p as i64))
    .bind(&now)
    .bind(preview_id)
    .execute(db)
    .await?;

    // Step 5: Update proxy routes
    if let Some(p) = port {
        let route_table = routes.load();
        let backend = Backend::new(container_id.clone(), "127.0.0.1".to_string(), p)
            .with_healthcheck(app.healthcheck.clone());

        route_table.add_route(preview.preview_domain.clone(), backend);

        info!(
            preview_id = %preview_id,
            domain = %preview.preview_domain,
            port = p,
            "Preview deployment running"
        );
    }

    Ok(())
}

/// Clean up a preview deployment
///
/// This function:
/// 1. Stops and removes the container
/// 2. Removes the proxy route
/// 3. Optionally removes the Docker image
/// 4. Updates the database status to 'closed'
pub async fn cleanup_preview(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    routes: Arc<ArcSwap<RouteTable>>,
    preview: &PreviewDeployment,
) -> Result<()> {
    info!(
        preview_id = %preview.id,
        pr = preview.pr_number,
        "Cleaning up preview deployment"
    );

    // Stop and remove container
    if let Some(ref container_name) = preview.container_name {
        if let Err(e) = runtime.stop(container_name).await {
            warn!(error = %e, "Failed to stop preview container (may already be stopped)");
        }
        if let Err(e) = runtime.remove(container_name).await {
            warn!(error = %e, "Failed to remove preview container");
        }
    }

    // Remove proxy route
    let route_table = routes.load();
    route_table.remove_route(&preview.preview_domain);

    // Optionally remove the Docker image to save disk space
    if let Some(ref image_tag) = preview.image_tag {
        if let Err(e) = runtime.remove_image(image_tag).await {
            warn!(error = %e, "Failed to remove preview image (may be in use)");
        }
    }

    // Update status to closed
    update_preview_status(db, &preview.id, PreviewDeploymentStatus::Closed, None).await?;

    info!(
        preview_id = %preview.id,
        "Preview deployment cleaned up"
    );

    Ok(())
}

/// Get SSH key for an app - checks app-specific key first, then falls back to global key
async fn get_ssh_key_for_app(db: &DbPool, app: &App) -> Result<Option<SshKey>> {
    // First, check if app has a specific SSH key configured
    if let Some(ref ssh_key_id) = app.ssh_key_id {
        let key = sqlx::query_as::<_, SshKey>("SELECT * FROM ssh_keys WHERE id = ?")
            .bind(ssh_key_id)
            .fetch_optional(db)
            .await?;
        if key.is_some() {
            return Ok(key);
        }
    }

    // Check for an app-specific SSH key (linked via app_id)
    let app_key = sqlx::query_as::<_, SshKey>("SELECT * FROM ssh_keys WHERE app_id = ?")
        .bind(&app.id)
        .fetch_optional(db)
        .await?;
    if app_key.is_some() {
        return Ok(app_key);
    }

    // Fall back to global SSH key
    let global_key = sqlx::query_as::<_, SshKey>(
        "SELECT * FROM ssh_keys WHERE is_global = 1 ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_optional(db)
    .await?;

    Ok(global_key)
}

/// Clone a repository to a destination directory
async fn clone_repository(
    url: &str,
    branch: &str,
    dest: &PathBuf,
    ssh_key: Option<&SshKey>,
) -> Result<()> {
    use std::process::Stdio;
    use tokio::process::Command;

    // Create destination directory
    tokio::fs::create_dir_all(dest).await?;

    // If we have an SSH key and the URL is an SSH URL, set up SSH authentication
    if let Some(key) = ssh_key {
        if is_ssh_url(url) {
            return clone_with_ssh_key(url, branch, dest, key).await;
        }
    }

    // Use git CLI for public repos or HTTPS URLs
    let output = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "--branch",
            branch,
            url,
            &dest.to_string_lossy(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to execute git clone")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git clone failed: {}", stderr);
    }

    Ok(())
}

/// Check if a URL is an SSH URL
fn is_ssh_url(url: &str) -> bool {
    url.starts_with("git@") || url.starts_with("ssh://")
}

/// Clone a repository using SSH key authentication
async fn clone_with_ssh_key(
    url: &str,
    branch: &str,
    dest: &PathBuf,
    ssh_key: &SshKey,
) -> Result<()> {
    use std::process::Stdio;
    use tokio::process::Command;

    // Create a temporary file for the SSH key
    let temp_dir = std::env::temp_dir();
    let key_file = temp_dir.join(format!("rivetr-preview-ssh-{}", uuid::Uuid::new_v4()));

    // Write the private key to the temp file
    tokio::fs::write(&key_file, &ssh_key.private_key).await?;

    // Set proper permissions on the key file (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = tokio::fs::metadata(&key_file).await?.permissions();
        perms.set_mode(0o600);
        tokio::fs::set_permissions(&key_file, perms).await?;
    }

    // Build GIT_SSH_COMMAND to use our key file
    let git_ssh_command = format!(
        "ssh -i {} -o StrictHostKeyChecking=accept-new -o UserKnownHostsFile=/dev/null",
        key_file.display()
    );

    let output = Command::new("git")
        .env("GIT_SSH_COMMAND", &git_ssh_command)
        .args([
            "clone",
            "--depth",
            "1",
            "--branch",
            branch,
            url,
            &dest.to_string_lossy(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to execute git clone with SSH key")?;

    // Clean up the temporary key file
    let _ = tokio::fs::remove_file(&key_file).await;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git clone with SSH failed: {}", stderr);
    }

    Ok(())
}

/// Find or create a preview deployment for a PR
pub async fn find_or_create_preview(
    db: &DbPool,
    app: &App,
    info: &PreviewDeploymentInfo,
    base_domain: &str,
) -> Result<PreviewDeployment> {
    // Check if preview already exists for this PR
    let existing: Option<PreviewDeployment> =
        sqlx::query_as("SELECT * FROM preview_deployments WHERE app_id = ? AND pr_number = ?")
            .bind(&app.id)
            .bind(info.pr_number)
            .fetch_optional(db)
            .await?;

    if let Some(mut preview) = existing {
        // Update existing preview with new commit info
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE preview_deployments
            SET commit_sha = ?, commit_message = ?, pr_title = ?, status = 'pending',
                error_message = NULL, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&info.commit_sha)
        .bind(&info.commit_message)
        .bind(&info.pr_title)
        .bind(&now)
        .bind(&preview.id)
        .execute(db)
        .await?;

        preview.commit_sha = info.commit_sha.clone();
        preview.commit_message = info.commit_message.clone();
        preview.pr_title = info.pr_title.clone();
        preview.status = "pending".to_string();

        return Ok(preview);
    }

    // Create new preview deployment
    let preview_id = uuid::Uuid::new_v4().to_string();
    let preview_domain = generate_preview_domain(&app.name, info.pr_number, base_domain);
    let container_name = generate_preview_container_name(&app.name, info.pr_number);
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO preview_deployments (
            id, app_id, pr_number, pr_title, pr_source_branch, pr_target_branch,
            pr_author, pr_url, provider_type, repo_full_name, preview_domain,
            container_name, commit_sha, commit_message, status, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?)
        "#,
    )
    .bind(&preview_id)
    .bind(&app.id)
    .bind(info.pr_number)
    .bind(&info.pr_title)
    .bind(&info.pr_source_branch)
    .bind(&info.pr_target_branch)
    .bind(&info.pr_author)
    .bind(&info.pr_url)
    .bind(&info.provider_type)
    .bind(&info.repo_full_name)
    .bind(&preview_domain)
    .bind(&container_name)
    .bind(&info.commit_sha)
    .bind(&info.commit_message)
    .bind(&now)
    .bind(&now)
    .execute(db)
    .await?;

    let preview = PreviewDeployment {
        id: preview_id,
        app_id: app.id.clone(),
        pr_number: info.pr_number,
        pr_title: info.pr_title.clone(),
        pr_source_branch: info.pr_source_branch.clone(),
        pr_target_branch: info.pr_target_branch.clone(),
        pr_author: info.pr_author.clone(),
        pr_url: info.pr_url.clone(),
        provider_type: info.provider_type.clone(),
        repo_full_name: info.repo_full_name.clone(),
        preview_domain,
        container_id: None,
        container_name: Some(container_name),
        image_tag: None,
        port: None,
        commit_sha: info.commit_sha.clone(),
        commit_message: info.commit_message.clone(),
        status: "pending".to_string(),
        error_message: None,
        github_comment_id: None,
        memory_limit: Some("256m".to_string()),
        cpu_limit: Some("0.5".to_string()),
        created_at: now.clone(),
        updated_at: now,
        closed_at: None,
    };

    Ok(preview)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_preview_domain() {
        assert_eq!(
            generate_preview_domain("my-app", 123, "preview.example.com"),
            "pr-123.my-app.preview.example.com"
        );

        // Test sanitization
        assert_eq!(
            generate_preview_domain("My App!", 42, "preview.example.com"),
            "pr-42.my-app-.preview.example.com"
        );

        // Test truncation of long names
        let long_name = "a".repeat(50);
        let domain = generate_preview_domain(&long_name, 1, "preview.example.com");
        assert!(domain.len() < 100);
    }

    #[test]
    fn test_generate_preview_container_name() {
        assert_eq!(
            generate_preview_container_name("my-app", 123),
            "rivetr-preview-my-app-pr-123"
        );
    }
}
