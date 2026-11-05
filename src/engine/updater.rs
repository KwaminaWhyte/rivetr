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
use crate::settings::updates::{cron_fired_between, UpdateChannel, UpdateSettings};

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
    /// Release channel: "stable" or "beta"
    pub channel: String,
    /// Cron (UTC) for update checks
    pub check_cron: String,
    /// Cron (UTC) for automatic updates
    pub auto_update_cron: String,
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
    /// Live settings (toml defaults overridden by dashboard values).
    settings: RwLock<UpdateSettings>,
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
            settings: RwLock::new(UpdateSettings::from_toml(&config)),
            config,
            state: Arc::new(RwLock::new(UpdateState::default())),
            http_client,
        }
    }

    /// Current effective settings.
    pub fn settings(&self) -> UpdateSettings {
        self.settings.read().clone()
    }

    /// Replace the live settings. The background loop picks them up on its
    /// next tick, no restart needed.
    pub fn set_settings(&self, settings: UpdateSettings) {
        *self.settings.write() = settings;
    }

    /// The toml `[auto_update]` section (defaults for the dashboard settings).
    pub fn toml_config(&self) -> &AutoUpdateConfig {
        &self.config
    }

    /// Get the shared state for API access
    pub fn state(&self) -> Arc<RwLock<UpdateState>> {
        self.state.clone()
    }

    /// Check for updates once
    pub async fn check_for_updates(&self) -> Result<Option<GitHubRelease>> {
        let include_prereleases = self.settings.read().include_prereleases();
        // `/releases/latest` never returns pre-releases, so the beta channel
        // lists recent releases and picks the newest one.
        let url = if include_prereleases {
            format!(
                "https://api.github.com/repos/{}/releases?per_page=30",
                self.config.github_repo
            )
        } else {
            format!(
                "https://api.github.com/repos/{}/releases/latest",
                self.config.github_repo
            )
        };

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
            return Err(anyhow!("GitHub API returned error {}: {}", status, body));
        }

        let releases: Vec<GitHubRelease> = if include_prereleases {
            response
                .json()
                .await
                .context("Failed to parse GitHub releases response")?
        } else {
            vec![response
                .json()
                .await
                .context("Failed to parse GitHub release response")?]
        };

        let Some(release) = pick_release(releases, include_prereleases) else {
            debug!("No eligible release found");
            return Ok(None);
        };

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
        let settings = self.settings();
        let state = self.state.read();
        let update_available = state
            .latest_release
            .as_ref()
            .map(|r| is_newer_version(r.tag_name.trim_start_matches('v'), CURRENT_VERSION))
            .unwrap_or(false);

        UpdateStatus {
            current_version: format!("v{}", CURRENT_VERSION),
            latest_version: state.latest_release.as_ref().map(|r| r.tag_name.clone()),
            update_available,
            download_url: state.latest_release.as_ref().and_then(|r| {
                r.assets
                    .iter()
                    .find(|a| a.name.contains("linux") && a.name.contains("x86_64"))
                    .map(|a| a.browser_download_url.clone())
            }),
            release_notes: state.latest_release.as_ref().and_then(|r| r.body.clone()),
            release_url: state.latest_release.as_ref().map(|r| r.html_url.clone()),
            last_checked: state
                .last_checked
                .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
            last_error: state.last_error.clone(),
            auto_update_enabled: settings.check_enabled,
            auto_apply_enabled: settings.auto_update_enabled,
            channel: match settings.channel {
                UpdateChannel::Stable => "stable".to_string(),
                UpdateChannel::Beta => "beta".to_string(),
            },
            check_cron: settings.check_cron.clone(),
            auto_update_cron: settings.auto_update_cron.clone(),
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

        let bytes = response
            .bytes()
            .await
            .context("Failed to read update file")?;

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
        // Get current executable path - resolve symlinks to get the real path
        let current_exe =
            std::env::current_exe().context("Failed to get current executable path")?;
        let current_exe = tokio::fs::canonicalize(&current_exe)
            .await
            .unwrap_or(current_exe);

        info!("Current binary path: {}", current_exe.display());

        // Create backup in the same directory as the binary
        let backup_path = current_exe.with_extension("bak");

        // On Linux with systemd ProtectSystem=strict, the binary directory may
        // be read-only. Try the binary's directory first, then fall back to temp.
        let backup_path = match tokio::fs::copy(&current_exe, &backup_path).await {
            Ok(_) => {
                info!("Backup created at: {}", backup_path.display());
                backup_path
            }
            Err(e) => {
                warn!(
                    "Cannot backup to {}: {}. Trying temp directory.",
                    backup_path.display(),
                    e
                );
                let fallback = std::env::temp_dir().join("rivetr.bak");
                tokio::fs::copy(&current_exe, &fallback)
                    .await
                    .context("Failed to create backup of current binary (tried both install dir and temp dir)")?;
                info!("Backup created at fallback: {}", fallback.display());
                fallback
            }
        };

        // Replace binary - on some systems we need to remove first then copy
        // because the running binary may be locked
        if tokio::fs::copy(new_binary, &current_exe).await.is_err() {
            // Try remove-then-rename approach
            warn!("Direct copy failed, trying remove-then-rename approach");
            let _ = tokio::fs::remove_file(&current_exe).await;
            tokio::fs::copy(new_binary, &current_exe)
                .await
                .context("Failed to replace binary")?;
        }

        // Ensure the new binary is executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&current_exe).await?.permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(&current_exe, perms).await?;
        }

        info!(
            "Update applied. Backup saved to: {}. Service restart required.",
            backup_path.display()
        );

        Ok(backup_path)
    }
}

/// Start the background update checker task.
///
/// The loop wakes up every minute and runs a check or an automatic update when
/// the configured cron fired since the previous tick. Settings are re-read on
/// every tick, so changes saved from the dashboard apply without a restart.
pub fn start_update_checker(config: AutoUpdateConfig) -> Arc<UpdateChecker> {
    let checker = Arc::new(UpdateChecker::new(config));
    let checker_clone = checker.clone();

    tokio::spawn(async move {
        // Initial check after a short delay
        tokio::time::sleep(Duration::from_secs(60)).await;
        if checker_clone.settings().check_enabled {
            info!("Starting update checker");
            checker_clone.run_check().await;
        }

        let mut last_tick = chrono::Utc::now();
        let mut ticker = interval(Duration::from_secs(60));
        loop {
            ticker.tick().await;
            let now = chrono::Utc::now();
            let settings = checker_clone.settings();

            if settings.check_enabled && cron_fired_between(&settings.check_cron, last_tick, now) {
                crate::utils::supervise::guarded("update_checker", checker_clone.run_check()).await;
            }

            if settings.auto_update_enabled
                && cron_fired_between(&settings.auto_update_cron, last_tick, now)
            {
                checker_clone.auto_apply().await;
            }

            last_tick = now;
        }
    });

    checker
}

impl UpdateChecker {
    /// Check, then download and apply the latest release if one is available.
    async fn auto_apply(&self) {
        self.run_check().await;
        let should_apply = {
            let mut state = self.state.write();
            let go = state.latest_release.is_some() && !state.update_in_progress;
            if go {
                state.update_in_progress = true;
            }
            go
        };
        if !should_apply {
            return;
        }
        info!("Auto-applying update...");
        match self.download_update().await {
            Ok(path) => {
                if let Err(e) = self.apply_update(&path).await {
                    error!("Failed to apply update: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to download update: {}", e);
            }
        }
        self.state.write().update_in_progress = false;
    }
}

/// Newest non-draft release, skipping pre-releases unless asked for.
fn pick_release(releases: Vec<GitHubRelease>, include_prereleases: bool) -> Option<GitHubRelease> {
    releases
        .into_iter()
        .filter(|r| !r.draft && (include_prereleases || !r.prerelease))
        .max_by(|a, b| {
            compare_versions(
                a.tag_name.trim_start_matches('v'),
                b.tag_name.trim_start_matches('v'),
            )
        })
}

/// Order two versions. A pre-release (`1.2.0-beta.1`) sorts below the
/// matching release (`1.2.0`); two pre-releases of the same version compare
/// by their suffix.
fn compare_versions(v1: &str, v2: &str) -> std::cmp::Ordering {
    type Parsed = ((u32, u32, u32), Option<String>);
    let parse = |v: &str| -> Parsed {
        let (core, pre) = match v.split_once('-') {
            Some((c, p)) => (c, Some(p.to_string())),
            None => (v, None),
        };
        let parts: Vec<u32> = core.split('.').filter_map(|s| s.parse().ok()).collect();
        (
            (
                parts.first().copied().unwrap_or(0),
                parts.get(1).copied().unwrap_or(0),
                parts.get(2).copied().unwrap_or(0),
            ),
            pre,
        )
    };
    let (c1, p1) = parse(v1);
    let (c2, p2) = parse(v2);
    c1.cmp(&c2).then_with(|| match (p1, p2) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(a), Some(b)) => a.cmp(&b),
    })
}

/// Compare semantic versions, returns true if v1 > v2
fn is_newer_version(v1: &str, v2: &str) -> bool {
    compare_versions(v1, v2) == std::cmp::Ordering::Greater
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

    #[test]
    fn test_prerelease_ordering() {
        assert!(is_newer_version("0.11.0-beta.1", "0.10.20"));
        assert!(!is_newer_version("0.11.0-beta.1", "0.11.0"));
        assert!(is_newer_version("0.11.0", "0.11.0-beta.2"));
        assert!(is_newer_version("0.11.0-beta.2", "0.11.0-beta.1"));
    }
}
