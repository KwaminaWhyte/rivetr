use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::db::{App, GitHubApp, GitHubAppInstallation, GitProvider, SshKey};
use crate::DbPool;

/// Get SSH key for an app - checks app-specific key first, then falls back to global key
pub(super) async fn get_ssh_key_for_app(db: &DbPool, app: &App) -> Result<Option<SshKey>> {
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

/// Options controlling how git clone behaves for a deployment.
pub(super) struct CloneOptions {
    /// Use --depth 1 for a shallow clone (faster, default true).
    pub shallow: bool,
    /// Pass --recurse-submodules to git clone.
    pub submodules: bool,
    /// Run `git lfs pull` after clone (requires git-lfs on PATH).
    pub lfs: bool,
}

impl Default for CloneOptions {
    fn default() -> Self {
        Self {
            shallow: true,
            submodules: false,
            lfs: false,
        }
    }
}

pub(super) async fn clone_repository(
    url: &str,
    branch: &str,
    dest: &PathBuf,
    ssh_key: Option<&SshKey>,
    opts: &CloneOptions,
) -> Result<()> {
    use std::process::Stdio;
    use tokio::process::Command;

    // Create destination directory
    tokio::fs::create_dir_all(dest).await?;

    // If we have an SSH key and the URL is an SSH URL, set up SSH authentication
    if let Some(key) = ssh_key {
        if is_ssh_url(url) {
            return clone_with_ssh_key(url, branch, dest, key, opts).await;
        }
    }

    // Build git clone arguments
    let mut args: Vec<&str> = vec!["clone"];

    if opts.shallow {
        args.extend_from_slice(&["--depth", "1"]);
    }

    if opts.submodules {
        args.push("--recurse-submodules");
    }

    args.extend_from_slice(&["--branch", branch, url]);
    let dest_str = dest.to_string_lossy().into_owned();
    args.push(&dest_str);

    // Use git CLI for public repos or HTTPS URLs
    let output = Command::new("git")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to execute git clone")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git clone failed: {}", stderr);
    }

    // Run git lfs pull if requested
    if opts.lfs {
        run_git_lfs_pull(dest).await?;
    }

    Ok(())
}

/// Run `git lfs pull` inside the cloned repository directory.
/// Exposed as `pub(super)` so the pipeline orchestrator can call it after a full clone.
pub(super) async fn run_lfs_pull(dest: &PathBuf) -> Result<()> {
    run_git_lfs_pull(dest).await
}

async fn run_git_lfs_pull(dest: &PathBuf) -> Result<()> {
    use std::process::Stdio;
    use tokio::process::Command;

    let output = Command::new("git")
        .args(["lfs", "pull"])
        .current_dir(dest)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to execute git lfs pull")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git lfs pull failed: {}", stderr);
    }

    Ok(())
}

/// Check if a URL is an SSH URL (git@host:path or ssh://...)
pub(super) fn is_ssh_url(url: &str) -> bool {
    url.starts_with("git@") || url.starts_with("ssh://")
}

/// Inject OAuth/PAT credentials into an HTTPS git URL so cloning succeeds
/// without an interactive credential prompt.
///
/// Result formats:
///   GitHub  → `https://x-access-token:{token}@github.com/...`
///   GitLab  → `https://oauth2:{token}@gitlab.com/...`
///   Bitbucket → `https://x-token-auth:{token}@bitbucket.org/...`
///   Other   → `https://oauth2:{token}@host/...`
pub(super) fn inject_credentials_into_url(url: &str, provider: &GitProvider) -> String {
    if is_ssh_url(url) {
        return url.to_string();
    }

    let token = &provider.access_token;
    let userinfo = match provider.provider.as_str() {
        "github" => format!("x-access-token:{}", token),
        "gitlab" => format!("oauth2:{}", token),
        "bitbucket" => format!("x-token-auth:{}", token),
        _ => format!("oauth2:{}", token),
    };

    if let Some(rest) = url.strip_prefix("https://") {
        // If credentials are already embedded, replace them
        if let Some(at_pos) = rest.find('@') {
            format!("https://{}@{}", userinfo, &rest[at_pos + 1..])
        } else {
            format!("https://{}@{}", userinfo, rest)
        }
    } else if let Some(rest) = url.strip_prefix("http://") {
        format!("http://{}@{}", userinfo, rest)
    } else {
        url.to_string()
    }
}

/// Fetch the git provider linked to an app and return the authenticated clone URL.
/// Returns the original URL unchanged if no provider is linked or the URL is SSH.
pub(super) async fn get_authenticated_url(
    db: &DbPool,
    app: &App,
    encryption_key: Option<&[u8; 32]>,
) -> Result<String> {
    if is_ssh_url(&app.git_url) {
        return Ok(app.git_url.clone());
    }

    // Check for GitHub App installation first (takes precedence over OAuth)
    if let Some(ref installation_id_str) = app.github_app_installation_id {
        let installation = sqlx::query_as::<_, GitHubAppInstallation>(
            "SELECT * FROM github_app_installations WHERE id = ?",
        )
        .bind(installation_id_str)
        .fetch_optional(db)
        .await?;

        if let Some(installation) = installation {
            let github_app =
                sqlx::query_as::<_, GitHubApp>("SELECT * FROM github_apps WHERE id = ?")
                    .bind(&installation.github_app_id)
                    .fetch_optional(db)
                    .await?;

            if let Some(github_app) = github_app {
                let private_key =
                    crate::crypto::decrypt_if_encrypted(&github_app.private_key, encryption_key)
                        .unwrap_or_else(|_| github_app.private_key.clone());

                match crate::github::get_installation_token(
                    github_app.app_id,
                    &private_key,
                    installation.installation_id,
                )
                .await
                {
                    Ok(token_response) => {
                        let token = token_response.token;
                        return Ok(inject_github_token_into_url(&app.git_url, &token));
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to get GitHub App installation token: {}. Falling through.",
                            e
                        );
                    }
                }
            }
        }
    }

    // Fall back to OAuth/PAT token via git_provider_id
    let provider_id: Option<Option<String>> =
        sqlx::query_scalar("SELECT git_provider_id FROM apps WHERE id = ?")
            .bind(&app.id)
            .fetch_optional(db)
            .await?;

    let provider_id = match provider_id.flatten() {
        Some(id) => id,
        None => return Ok(app.git_url.clone()),
    };

    let provider = sqlx::query_as::<_, GitProvider>("SELECT * FROM git_providers WHERE id = ?")
        .bind(&provider_id)
        .fetch_optional(db)
        .await?;

    match provider {
        Some(p) => Ok(inject_credentials_into_url(&app.git_url, &p)),
        None => Ok(app.git_url.clone()),
    }
}

/// Inject a GitHub installation token into an HTTPS URL.
fn inject_github_token_into_url(url: &str, token: &str) -> String {
    let userinfo = format!("x-access-token:{}", token);
    if let Some(rest) = url.strip_prefix("https://") {
        if let Some(at_pos) = rest.find('@') {
            format!("https://{}@{}", userinfo, &rest[at_pos + 1..])
        } else {
            format!("https://{}@{}", userinfo, rest)
        }
    } else if let Some(rest) = url.strip_prefix("http://") {
        format!("http://{}@{}", userinfo, rest)
    } else {
        url.to_string()
    }
}

/// Clone a repository using SSH key authentication
pub(super) async fn clone_with_ssh_key(
    url: &str,
    branch: &str,
    dest: &Path,
    ssh_key: &SshKey,
    opts: &CloneOptions,
) -> Result<()> {
    use std::process::Stdio;
    use tokio::process::Command;

    // Create a temporary file for the SSH key
    let temp_dir = std::env::temp_dir();
    let key_file = temp_dir.join(format!("rivetr-ssh-{}", uuid::Uuid::new_v4()));

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

    // Build git clone arguments
    let mut args: Vec<String> = vec!["clone".to_string()];

    if opts.shallow {
        args.push("--depth".to_string());
        args.push("1".to_string());
    }

    if opts.submodules {
        args.push("--recurse-submodules".to_string());
    }

    args.push("--branch".to_string());
    args.push(branch.to_string());
    args.push(url.to_string());
    args.push(dest.to_string_lossy().to_string());

    let output = Command::new("git")
        .env("GIT_SSH_COMMAND", &git_ssh_command)
        .args(&args)
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

    // Run git lfs pull if requested
    if opts.lfs {
        let dest_buf = dest.to_path_buf();
        run_git_lfs_pull(&dest_buf).await?;
    }

    Ok(())
}

/// Clone a repository without --depth 1 (full history needed for specific commit checkout)
pub(super) async fn clone_repository_full(
    url: &str,
    branch: &str,
    dest: &PathBuf,
    ssh_key: Option<&SshKey>,
) -> Result<()> {
    use std::process::Stdio;
    use tokio::process::Command;

    // Create destination directory
    tokio::fs::create_dir_all(dest).await?;

    if let Some(key) = ssh_key {
        if is_ssh_url(url) {
            return clone_with_ssh_key_full(url, branch, dest, key).await;
        }
    }

    let output = Command::new("git")
        .args(["clone", "--branch", branch, url, &dest.to_string_lossy()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to execute git clone (full)")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git clone (full) failed: {}", stderr);
    }

    Ok(())
}

/// Clone a repository with SSH key authentication (full history)
pub(super) async fn clone_with_ssh_key_full(
    url: &str,
    branch: &str,
    dest: &Path,
    ssh_key: &SshKey,
) -> Result<()> {
    use std::process::Stdio;
    use tokio::process::Command;

    let temp_dir = std::env::temp_dir();
    let key_file = temp_dir.join(format!("rivetr-ssh-{}", uuid::Uuid::new_v4()));

    tokio::fs::write(&key_file, &ssh_key.private_key).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = tokio::fs::metadata(&key_file).await?.permissions();
        perms.set_mode(0o600);
        tokio::fs::set_permissions(&key_file, perms).await?;
    }

    let git_ssh_command = format!(
        "ssh -i {} -o StrictHostKeyChecking=accept-new -o UserKnownHostsFile=/dev/null",
        key_file.display()
    );

    let output = Command::new("git")
        .env("GIT_SSH_COMMAND", &git_ssh_command)
        .args(["clone", "--branch", branch, url, &dest.to_string_lossy()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to execute git clone with SSH key (full)")?;

    let _ = tokio::fs::remove_file(&key_file).await;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git clone with SSH (full) failed: {}", stderr);
    }

    Ok(())
}

/// Checkout a specific git ref (commit SHA or tag) in a cloned repository
pub(super) async fn git_checkout(work_dir: &PathBuf, ref_name: &str) -> Result<()> {
    use std::process::Stdio;
    use tokio::process::Command;

    let output = Command::new("git")
        .args(["checkout", ref_name])
        .current_dir(work_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to execute git checkout")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git checkout '{}' failed: {}", ref_name, stderr);
    }

    Ok(())
}

/// Get commit SHA and message from HEAD in a git repository
pub(super) async fn get_git_commit_info(work_dir: &PathBuf) -> Result<(String, String)> {
    use std::process::Stdio;
    use tokio::process::Command;

    let sha_output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(work_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to get git commit SHA")?;

    if !sha_output.status.success() {
        anyhow::bail!("Failed to get commit SHA");
    }

    let sha = String::from_utf8_lossy(&sha_output.stdout)
        .trim()
        .to_string();

    let msg_output = Command::new("git")
        .args(["log", "-1", "--format=%s"])
        .current_dir(work_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to get git commit message")?;

    let message = if msg_output.status.success() {
        String::from_utf8_lossy(&msg_output.stdout)
            .trim()
            .to_string()
    } else {
        String::new()
    };

    Ok((sha, message))
}
