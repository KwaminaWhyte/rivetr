//! Auto-update system for Rivetr
//!
//! Checks for new releases on GitHub and optionally downloads/applies updates.
//! Updates are atomic - the binary is replaced and requires a service restart.

use anyhow::{anyhow, Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::config::AutoUpdateConfig;

/// Current version from Cargo.toml
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub API response for a release
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub prerelease: bool,
    pub draft: bool,
    pub published_at: String,
    pub html_url: String,
    pub body: Option<String>,
    pub assets: Vec<GitHubAsset>,
}

/// GitHub release asset
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub size: u64,
    pub browser_download_url: String,
    pub content_type: String,
}

/// Update status information
#[derive(Debug, Clone, Serialize)]
pub struct UpdateStatus {
    /// Current running version
    pub current_version: String,
    /// Latest available version (None if check failed or up-to-date)
    pub latest_version: Option<String>,
    /// Whether an update is available
    pub update_available: bool,
    /// URL to download the update
    pub download_url: Option<String>,
    /// Release notes/changelog
    pub release_notes: Option<String>,
    /// Release page URL
    pub release_url: Option<String>,
    /// When the last check was performed (ISO 8601)
    pub last_checked: Option<String>,
    /// Error message if the last check failed
    pub last_error: Option<String>,
    /// Whether auto-update is enabled
    pub auto_update_enabled: bool,
    /// Whether auto-apply is enabled
    pub auto_apply_enabled: bool,
}

/// Shared update state
#[derive(Debug, Default)]
pub struct UpdateState {
    pub latest_release: Option<GitHubRelease>,
    pub last_checked: Option<chrono::DateTime<chrono::Utc>>,
    pub last_error: Option<String>,
    pub update_in_progress: bool,
}

/// Update checker service
pub struct UpdateChecker {
    config: AutoUpdateConfig,
    state: Arc<RwLock<UpdateState>>,
    http_client: reqwest::Client,
}

impl UpdateChecker {
    /// Create a new update checker
    pub fn new(config: AutoUpdateConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .user_agent(format!("rivetr/{}", CURRENT_VERSION))
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            config,
            state: Arc::new(RwLock::new(UpdateState::default())),
            http_client,
        }
    }

    /// Get the shared state for API access
    pub fn state(&self) -> Arc<RwLock<UpdateState>> {
        self.state.clone()
    }

    /// Check for updates once
    pub async fn check_for_updates(&self) -> Result<Option<GitHubRelease>> {
        let url = format!(
            "https://api.github.com/repos/{}/releases/latest",
            self.config.github_repo
        );

        debug!("Checking for updates from: {}", url);

        let response = self
            .http_client
            .get(&url)
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .context("Failed to fetch release info from GitHub")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "GitHub API returned error {}: {}",
                status,
                body
            ));
        }

        let release: GitHubRelease = response
            .json()
            .await
            .context("Failed to parse GitHub release response")?;

        // Skip if prerelease and we don't want those
        if release.prerelease && !self.config.include_prereleases {
            debug!("Skipping prerelease: {}", release.tag_name);
            return Ok(None);
        }

        // Skip drafts
        if release.draft {
            debug!("Skipping draft release: {}", release.tag_name);
            return Ok(None);
        }

        // Compare versions
        let latest_version = release.tag_name.trim_start_matches('v');
        let current_version = CURRENT_VERSION;

        if is_newer_version(latest_version, current_version) {
            info!(
                "Update available: {} -> {}",
                current_version, latest_version
            );
            Ok(Some(release))
        } else {
            debug!(
                "Already at latest version: {} (latest: {})",
                current_version, latest_version
            );
            Ok(None)
        }
    }

    /// Run the update check and update state
    pub async fn run_check(&self) {
        let result = self.check_for_updates().await;
        let mut state = self.state.write();
        state.last_checked = Some(chrono::Utc::now());

        match result {
            Ok(release) => {
                state.latest_release = release;
                state.last_error = None;
            }
            Err(e) => {
                warn!("Update check failed: {}", e);
                state.last_error = Some(e.to_string());
            }
        }
    }

    /// Get current update status
    pub fn get_status(&self) -> UpdateStatus {
        let state = self.state.read();
        let update_available = state
            .latest_release
            .as_ref()
            .map(|r| is_newer_version(r.tag_name.trim_start_matches('v'), CURRENT_VERSION))
            .unwrap_or(false);

        UpdateStatus {
            current_version: format!("v{}", CURRENT_VERSION),
            latest_version: state
                .latest_release
                .as_ref()
                .map(|r| r.tag_name.clone()),
            update_available,
            download_url: state.latest_release.as_ref().and_then(|r| {
                r.assets
                    .iter()
                    .find(|a| a.name.contains("linux") && a.name.contains("x86_64"))
                    .map(|a| a.browser_download_url.clone())
            }),
            release_notes: state
                .latest_release
                .as_ref()
                .and_then(|r| r.body.clone()),
            release_url: state
                .latest_release
                .as_ref()
                .map(|r| r.html_url.clone()),
            last_checked: state
                .last_checked
                .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
            last_error: state.last_error.clone(),
            auto_update_enabled: self.config.enabled,
            auto_apply_enabled: self.config.auto_apply,
        }
    }

    /// Get the download URL for the current platform
    pub fn get_download_url(&self) -> Option<String> {
        let state = self.state.read();
        state.latest_release.as_ref().and_then(|release| {
            // Determine platform
            let os = std::env::consts::OS;
            let arch = std::env::consts::ARCH;

            let os_part = match os {
                "linux" => "linux",
                "macos" => "darwin",
                "windows" => "windows",
                _ => return None,
            };

            let arch_part = match arch {
                "x86_64" | "amd64" => "x86_64",
                "aarch64" | "arm64" => "aarch64",
                _ => return None,
            };

            // Find matching asset
            release
                .assets
                .iter()
                .find(|asset| {
                    let name = asset.name.to_lowercase();
                    name.contains(os_part) && name.contains(arch_part)
                })
                .map(|a| a.browser_download_url.clone())
        })
    }

    /// Download the update binary to a temporary file
    pub async fn download_update(&self) -> Result<std::path::PathBuf> {
        let download_url = self
            .get_download_url()
            .ok_or_else(|| anyhow!("No download URL available for current platform"))?;

        info!("Downloading update from: {}", download_url);

        let response = self
            .http_client
            .get(&download_url)
            .send()
            .await
            .context("Failed to download update")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Download failed with status: {}",
                response.status()
            ));
        }

        let bytes = response.bytes().await.context("Failed to read update file")?;

        // Write to temp file
        let temp_path = std::env::temp_dir().join("rivetr-update");
        tokio::fs::write(&temp_path, &bytes)
            .await
            .context("Failed to write update to temp file")?;

        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&temp_path).await?.permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(&temp_path, perms).await?;
        }

        info!("Update downloaded to: {}", temp_path.display());
        Ok(temp_path)
    }

    /// Apply an update by replacing the binary
    /// Returns the path to the backup of the old binary
    pub async fn apply_update(&self, new_binary: &std::path::Path) -> Result<std::path::PathBuf> {
        // Get current executable path
        let current_exe = std::env::current_exe().context("Failed to get current executable path")?;

        // Create backup
        let backup_path = current_exe.with_extension("bak");
        tokio::fs::copy(&current_exe, &backup_path)
            .await
            .context("Failed to create backup of current binary")?;

        // Replace binary
        tokio::fs::copy(new_binary, &current_exe)
            .await
            .context("Failed to replace binary")?;

        info!(
            "Update applied. Backup saved to: {}. Service restart required.",
            backup_path.display()
        );

        Ok(backup_path)
    }
}

/// Start the background update checker task
pub fn start_update_checker(config: AutoUpdateConfig) -> Arc<UpdateChecker> {
    let checker = Arc::new(UpdateChecker::new(config.clone()));

    if !config.enabled {
        info!("Auto-update checking is disabled");
        return checker;
    }

    let checker_clone = checker.clone();
    let interval_hours = config.check_interval_hours;

    tokio::spawn(async move {
        // Initial check after a short delay
        tokio::time::sleep(Duration::from_secs(60)).await;

        info!(
            "Starting update checker (interval: {} hours)",
            interval_hours
        );
        checker_clone.run_check().await;

        // Periodic checks
        let mut check_interval = interval(Duration::from_secs(interval_hours * 3600));
        loop {
            check_interval.tick().await;
            checker_clone.run_check().await;

            // Auto-apply if enabled
            if checker_clone.config.auto_apply {
                let should_apply = {
                    let state = checker_clone.state.read();
                    state.latest_release.is_some() && !state.update_in_progress
                };
                if should_apply {
                    info!("Auto-applying update...");
                    match checker_clone.download_update().await {
                        Ok(path) => {
                            if let Err(e) = checker_clone.apply_update(&path).await {
                                error!("Failed to apply update: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to download update: {}", e);
                        }
                    }
                }
            }
        }
    });

    checker
}

/// Compare semantic versions, returns true if v1 > v2
fn is_newer_version(v1: &str, v2: &str) -> bool {
    let parse_version = |v: &str| -> (u32, u32, u32) {
        let parts: Vec<u32> = v
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect();
        (
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
            parts.get(2).copied().unwrap_or(0),
        )
    };

    let (major1, minor1, patch1) = parse_version(v1);
    let (major2, minor2, patch2) = parse_version(v2);

    (major1, minor1, patch1) > (major2, minor2, patch2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_newer_version("1.0.0", "0.9.9"));
        assert!(is_newer_version("0.2.10", "0.2.9"));
        assert!(is_newer_version("0.3.0", "0.2.10"));
        assert!(!is_newer_version("0.2.9", "0.2.10"));
        assert!(!is_newer_version("0.2.10", "0.2.10"));
        assert!(is_newer_version("1.0.0", "0.99.99"));
    }
}
