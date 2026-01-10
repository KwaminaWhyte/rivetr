//! Railpack builder integration - Railway's successor to Nixpacks
//!
//! Railpack is a zero-config application builder that automatically analyzes
//! source code and turns it into optimized container images.
//!
//! # Features
//!
//! - Automatic language and framework detection
//! - Better caching than Nixpacks (38% faster Node.js, 77% faster Python builds)
//! - First-class SPA support (Vite, Astro, CRA, Angular)
//! - Smaller image sizes
//! - Requires BuildKit for building
//!
//! # Requirements
//!
//! - BuildKit container running: `docker run --rm --privileged -d --name buildkit moby/buildkit`
//! - BUILDKIT_HOST environment variable set
//! - Note: Windows is NOT currently supported by Railpack

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{debug, error, info, warn};

/// Errors specific to Railpack operations
#[derive(Error, Debug)]
pub enum RailpackError {
    #[error(
        "Railpack CLI is not installed. Install with: mise install ubi:railwayapp/railpack@latest"
    )]
    NotInstalled,

    #[error("BuildKit is not available. Start with: docker run --rm --privileged -d --name buildkit moby/buildkit")]
    BuildKitNotAvailable,

    #[error("Railpack is not supported on Windows")]
    WindowsNotSupported,

    #[error("Failed to execute Railpack command: {0}")]
    ExecutionFailed(String),

    #[error("Railpack build failed: {0}")]
    BuildFailed(String),

    #[error("Unsupported project type")]
    UnsupportedProject,
}

/// Configuration for Railpack builds
///
/// Environment variables can be used to override build commands:
/// - RAILPACK_INSTALL_COMMAND - Custom install command
/// - RAILPACK_BUILD_COMMAND - Custom build command
/// - RAILPACK_START_COMMAND - Custom start command
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RailpackConfig {
    /// Custom install command (overrides auto-detected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_cmd: Option<String>,

    /// Custom build command (overrides auto-detected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_cmd: Option<String>,

    /// Custom start command (overrides auto-detected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_cmd: Option<String>,

    /// Force a specific provider (e.g., "node", "python", "go")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,

    /// Disable build caching
    #[serde(default, skip_serializing_if = "is_false")]
    pub no_cache: bool,
}

/// Helper for serde skip_serializing_if
fn is_false(b: &bool) -> bool {
    !*b
}

impl RailpackConfig {
    /// Parse from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to parse railpack config from JSON")
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).context("Failed to serialize railpack config to JSON")
    }

    /// Parse from TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self> {
        toml::from_str(toml_str).context("Failed to parse railpack.toml")
    }

    /// Load configuration from a railpack.toml file in the given directory
    pub async fn load_from_repo(source_path: &Path) -> Option<Self> {
        let toml_path = source_path.join("railpack.toml");

        if !toml_path.exists() {
            debug!("No railpack.toml found at {:?}", toml_path);
            return None;
        }

        match tokio::fs::read_to_string(&toml_path).await {
            Ok(contents) => match Self::from_toml(&contents) {
                Ok(config) => {
                    info!("Loaded railpack.toml configuration from {:?}", toml_path);
                    Some(config)
                }
                Err(e) => {
                    warn!("Failed to parse railpack.toml: {}", e);
                    None
                }
            },
            Err(e) => {
                warn!("Failed to read railpack.toml: {}", e);
                None
            }
        }
    }

    /// Merge another config into this one (other takes precedence)
    pub fn merge(&mut self, other: &RailpackConfig) {
        if other.install_cmd.is_some() {
            self.install_cmd = other.install_cmd.clone();
        }
        if other.build_cmd.is_some() {
            self.build_cmd = other.build_cmd.clone();
        }
        if other.start_cmd.is_some() {
            self.start_cmd = other.start_cmd.clone();
        }
        if other.provider.is_some() {
            self.provider = other.provider.clone();
        }
        if other.no_cache {
            self.no_cache = true;
        }
    }

    /// Check if this config has any custom settings
    pub fn is_empty(&self) -> bool {
        self.install_cmd.is_none()
            && self.build_cmd.is_none()
            && self.start_cmd.is_none()
            && self.provider.is_none()
            && !self.no_cache
    }
}

/// Check if running on Windows (Railpack doesn't support Windows)
fn is_windows() -> bool {
    cfg!(target_os = "windows")
}

/// Check if Railpack CLI is available on the system
pub async fn is_available() -> bool {
    if is_windows() {
        debug!("Railpack is not supported on Windows");
        return false;
    }

    Command::new("railpack")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Get Railpack version if available
pub async fn get_version() -> Option<String> {
    if is_windows() {
        return None;
    }

    let output = Command::new("railpack")
        .arg("--version")
        .output()
        .await
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Check if BuildKit is available (required for Railpack)
pub async fn is_buildkit_available() -> bool {
    // Check if BUILDKIT_HOST is set
    if std::env::var("BUILDKIT_HOST").is_ok() {
        return true;
    }

    // Check if buildkit container is running
    let output = Command::new("docker")
        .args(["ps", "--filter", "name=buildkit", "--format", "{{.Names}}"])
        .output()
        .await;

    match output {
        Ok(out) => {
            let output_str = String::from_utf8_lossy(&out.stdout);
            output_str.contains("buildkit")
        }
        Err(_) => false,
    }
}

/// Start BuildKit container if not running
pub async fn ensure_buildkit() -> Result<()> {
    if is_buildkit_available().await {
        return Ok(());
    }

    info!("Starting BuildKit container for Railpack...");

    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--privileged",
            "-d",
            "--name",
            "buildkit",
            "moby/buildkit",
        ])
        .output()
        .await
        .context("Failed to start BuildKit container")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Check if it's already running
        if !stderr.contains("is already in use") {
            return Err(RailpackError::BuildKitNotAvailable.into());
        }
    }

    // Set BUILDKIT_HOST environment variable for this process
    std::env::set_var("BUILDKIT_HOST", "docker-container://buildkit");

    info!("BuildKit container started successfully");
    Ok(())
}

/// Check if a railpack.toml configuration file exists in the source directory
pub async fn has_config_file(source_path: &Path) -> bool {
    let toml_path = source_path.join("railpack.toml");
    toml_path.exists()
}

/// Build an image using Railpack with real-time output streaming
///
/// # Arguments
///
/// * `source_path` - Path to the source code directory
/// * `image_tag` - Docker image tag to use (e.g., "rivetr-myapp:abc123")
/// * `config` - Optional Railpack configuration (from database or API)
/// * `env_vars` - Environment variables to pass to the build
///
/// # Returns
///
/// Returns the image tag on success.
pub async fn build_image(
    source_path: &Path,
    image_tag: &str,
    config: Option<&RailpackConfig>,
    env_vars: &[(String, String)],
) -> Result<String> {
    info!("Building image with Railpack: {}", image_tag);
    debug!("Source path: {:?}", source_path);

    // Check platform support
    if is_windows() {
        return Err(RailpackError::WindowsNotSupported.into());
    }

    // Check if Railpack is available
    if !is_available().await {
        return Err(RailpackError::NotInstalled.into());
    }

    // Ensure BuildKit is running
    ensure_buildkit().await?;

    // Load config from railpack.toml if present, and merge with provided config
    let mut effective_config = RailpackConfig::default();

    // First, load from railpack.toml in the repo (lowest priority)
    if let Some(repo_config) = RailpackConfig::load_from_repo(source_path).await {
        info!("Using railpack.toml configuration from repository");
        effective_config.merge(&repo_config);
    }

    // Then merge in the provided config (higher priority - from database/API)
    if let Some(cfg) = config {
        if !cfg.is_empty() {
            info!("Merging custom Railpack configuration from app settings");
            effective_config.merge(cfg);
        }
    }

    let mut cmd = Command::new("railpack");
    cmd.arg("build")
        .arg(source_path)
        .arg("--name")
        .arg(image_tag);

    // Set BUILDKIT_HOST if not already set
    if std::env::var("BUILDKIT_HOST").is_err() {
        cmd.env("BUILDKIT_HOST", "docker-container://buildkit");
    }

    // Pass environment variables
    for (key, value) in env_vars {
        // Use environment variable overrides for special cases
        match key.as_str() {
            "RAILPACK_INSTALL_COMMAND" | "RAILPACK_BUILD_COMMAND" | "RAILPACK_START_COMMAND" => {
                cmd.env(key, value);
            }
            _ => {
                // Pass as build-time env var
                // Don't log sensitive values
                if key.to_lowercase().contains("secret")
                    || key.to_lowercase().contains("password")
                    || key.to_lowercase().contains("token")
                    || key.to_lowercase().contains("key")
                {
                    debug!("Setting env var: {}=<redacted>", key);
                } else {
                    debug!("Setting env var: {}={}", key, value);
                }
                cmd.arg("--env").arg(format!("{}={}", key, value));
            }
        }
    }

    // Apply configuration via environment variables (Railpack style)
    if let Some(ref install) = effective_config.install_cmd {
        debug!("Using custom install command: {}", install);
        cmd.env("RAILPACK_INSTALL_COMMAND", install);
    }
    if let Some(ref build) = effective_config.build_cmd {
        debug!("Using custom build command: {}", build);
        cmd.env("RAILPACK_BUILD_COMMAND", build);
    }
    if let Some(ref start) = effective_config.start_cmd {
        debug!("Using custom start command: {}", start);
        cmd.env("RAILPACK_START_COMMAND", start);
    }

    if effective_config.no_cache {
        debug!("Disabling build cache");
        cmd.arg("--no-cache");
    }

    // Set up for streaming output
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    debug!(
        "Executing Railpack command: railpack build {:?}",
        source_path
    );

    let mut child = cmd.spawn().context("Failed to spawn railpack process")?;

    // Stream stdout
    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    let mut stdout_lines = stdout_reader.lines();
    let mut stderr_lines = stderr_reader.lines();

    // Collect output for error reporting
    let mut stderr_output = Vec::new();

    // Stream output in real-time using tokio::select
    loop {
        tokio::select! {
            stdout_line = stdout_lines.next_line() => {
                match stdout_line {
                    Ok(Some(line)) => {
                        info!(target: "railpack", "{}", line);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        warn!("Error reading railpack stdout: {}", e);
                        break;
                    }
                }
            }
            stderr_line = stderr_lines.next_line() => {
                match stderr_line {
                    Ok(Some(line)) => {
                        // Railpack outputs progress info to stderr
                        if line.contains("error") || line.contains("Error") || line.contains("failed") {
                            warn!(target: "railpack", "{}", line);
                        } else {
                            info!(target: "railpack", "{}", line);
                        }
                        stderr_output.push(line);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        warn!("Error reading railpack stderr: {}", e);
                        break;
                    }
                }
            }
            status = child.wait() => {
                match status {
                    Ok(exit_status) => {
                        if exit_status.success() {
                            info!("Railpack build completed successfully for {}", image_tag);
                            return Ok(image_tag.to_string());
                        } else {
                            let error_output = stderr_output.join("\n");
                            let error_msg = if error_output.is_empty() {
                                format!("Railpack build failed with exit code: {:?}", exit_status.code())
                            } else {
                                // Get the last few lines of stderr for the error message
                                let last_lines: Vec<_> = stderr_output.iter().rev().take(10).collect();
                                format!(
                                    "Railpack build failed (exit code {:?}):\n{}",
                                    exit_status.code(),
                                    last_lines.into_iter().rev().cloned().collect::<Vec<_>>().join("\n")
                                )
                            };
                            error!("Railpack build failed: {}", error_msg);
                            return Err(RailpackError::BuildFailed(error_msg).into());
                        }
                    }
                    Err(e) => {
                        return Err(RailpackError::ExecutionFailed(e.to_string()).into());
                    }
                }
            }
        }
    }

    // If we get here, the streams ended but we didn't get an exit status
    let status = child.wait().await?;
    if status.success() {
        info!("Railpack build completed successfully for {}", image_tag);
        Ok(image_tag.to_string())
    } else {
        let error_output = stderr_output.join("\n");
        Err(RailpackError::BuildFailed(error_output).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_railpack_config_json_serialization() {
        let config = RailpackConfig {
            install_cmd: Some("npm install".to_string()),
            build_cmd: Some("npm run build".to_string()),
            start_cmd: Some("npm start".to_string()),
            provider: None,
            no_cache: false,
        };

        let json = config.to_json().unwrap();
        let parsed = RailpackConfig::from_json(&json).unwrap();

        assert_eq!(parsed.install_cmd, config.install_cmd);
        assert_eq!(parsed.build_cmd, config.build_cmd);
        assert_eq!(parsed.start_cmd, config.start_cmd);
    }

    #[test]
    fn test_railpack_config_toml_parsing() {
        let toml = r#"
            install_cmd = "yarn install"
            build_cmd = "yarn build"
            start_cmd = "yarn start"
            provider = "node"
        "#;

        let config = RailpackConfig::from_toml(toml).unwrap();

        assert_eq!(config.install_cmd, Some("yarn install".to_string()));
        assert_eq!(config.build_cmd, Some("yarn build".to_string()));
        assert_eq!(config.start_cmd, Some("yarn start".to_string()));
        assert_eq!(config.provider, Some("node".to_string()));
    }

    #[test]
    fn test_empty_config() {
        let config = RailpackConfig::default();
        let json = config.to_json().unwrap();
        assert_eq!(json, "{}");
        assert!(config.is_empty());
    }

    #[test]
    fn test_config_merge() {
        let mut base = RailpackConfig {
            install_cmd: Some("npm install".to_string()),
            build_cmd: Some("npm build".to_string()),
            start_cmd: None,
            provider: None,
            no_cache: false,
        };

        let override_config = RailpackConfig {
            build_cmd: Some("npm run build:prod".to_string()),
            start_cmd: Some("npm start".to_string()),
            provider: Some("node".to_string()),
            no_cache: true,
            ..Default::default()
        };

        base.merge(&override_config);

        assert_eq!(base.install_cmd, Some("npm install".to_string()));
        assert_eq!(base.build_cmd, Some("npm run build:prod".to_string()));
        assert_eq!(base.start_cmd, Some("npm start".to_string()));
        assert_eq!(base.provider, Some("node".to_string()));
        assert!(base.no_cache);
    }
}
