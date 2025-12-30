//! Nixpacks builder integration for auto-generating Dockerfiles
//!
//! Nixpacks analyzes source code and automatically generates optimized
//! Dockerfiles without requiring manual Dockerfile authoring.
//!
//! # Features
//!
//! - Automatic language and framework detection
//! - Custom configuration via `nixpacks.toml` in repository
//! - Environment variable support during builds
//! - Real-time build output streaming
//! - Build plan generation for debugging

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{debug, error, info, warn};

/// Errors specific to Nixpacks operations
#[derive(Error, Debug)]
pub enum NixpacksError {
    #[error("Nixpacks CLI is not installed. Install with: curl -sSL https://nixpacks.com/install.sh | bash")]
    NotInstalled,

    #[error("Failed to execute Nixpacks command: {0}")]
    ExecutionFailed(String),

    #[error("Nixpacks build failed: {0}")]
    BuildFailed(String),

    #[error("Failed to parse Nixpacks configuration: {0}")]
    ConfigParseError(String),

    #[error("Failed to read nixpacks.toml: {0}")]
    TomlReadError(String),

    #[error("Unsupported project type or language not detected")]
    UnsupportedProject,
}

/// Configuration for Nixpacks builds
///
/// This struct maps to both the JSON configuration stored in the database
/// and the TOML configuration that can be placed in a repository's `nixpacks.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NixpacksConfig {
    /// Custom install command (overrides auto-detected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_cmd: Option<String>,

    /// Custom build command (overrides auto-detected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_cmd: Option<String>,

    /// Custom start command (overrides auto-detected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_cmd: Option<String>,

    /// Additional Nix packages to install
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packages: Option<Vec<String>>,

    /// Additional apt packages to install
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apt_packages: Option<Vec<String>>,

    /// Static assets directory (for static site generators)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_dir: Option<String>,

    /// Custom Nix libs to include
    #[serde(skip_serializing_if = "Option::is_none")]
    pub libs: Option<Vec<String>>,

    /// Force a specific language/provider (e.g., "node", "python", "rust")
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

impl NixpacksConfig {
    /// Parse from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to parse nixpacks config from JSON")
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).context("Failed to serialize nixpacks config to JSON")
    }

    /// Parse from TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self> {
        toml::from_str(toml_str).context("Failed to parse nixpacks.toml")
    }

    /// Load configuration from a nixpacks.toml file in the given directory
    pub async fn load_from_repo(source_path: &Path) -> Option<Self> {
        let toml_path = source_path.join("nixpacks.toml");

        if !toml_path.exists() {
            debug!("No nixpacks.toml found at {:?}", toml_path);
            return None;
        }

        match tokio::fs::read_to_string(&toml_path).await {
            Ok(contents) => match Self::from_toml(&contents) {
                Ok(config) => {
                    info!("Loaded nixpacks.toml configuration from {:?}", toml_path);
                    Some(config)
                }
                Err(e) => {
                    warn!("Failed to parse nixpacks.toml: {}", e);
                    None
                }
            },
            Err(e) => {
                warn!("Failed to read nixpacks.toml: {}", e);
                None
            }
        }
    }

    /// Merge another config into this one (other takes precedence)
    pub fn merge(&mut self, other: &NixpacksConfig) {
        if other.install_cmd.is_some() {
            self.install_cmd = other.install_cmd.clone();
        }
        if other.build_cmd.is_some() {
            self.build_cmd = other.build_cmd.clone();
        }
        if other.start_cmd.is_some() {
            self.start_cmd = other.start_cmd.clone();
        }
        if other.packages.is_some() {
            self.packages = other.packages.clone();
        }
        if other.apt_packages.is_some() {
            self.apt_packages = other.apt_packages.clone();
        }
        if other.static_dir.is_some() {
            self.static_dir = other.static_dir.clone();
        }
        if other.libs.is_some() {
            self.libs = other.libs.clone();
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
            && self.packages.is_none()
            && self.apt_packages.is_none()
            && self.static_dir.is_none()
            && self.libs.is_none()
            && self.provider.is_none()
            && !self.no_cache
    }
}

/// Check if Nixpacks CLI is available on the system
pub async fn is_available() -> bool {
    Command::new("nixpacks")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Get Nixpacks version if available
pub async fn get_version() -> Option<String> {
    let output = Command::new("nixpacks")
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

/// Check if a nixpacks.toml configuration file exists in the source directory
pub async fn has_config_file(source_path: &Path) -> bool {
    let toml_path = source_path.join("nixpacks.toml");
    toml_path.exists()
}

/// Apply NixpacksConfig options to a Command
fn apply_config_to_command(cmd: &mut Command, config: &NixpacksConfig) {
    if let Some(ref install) = config.install_cmd {
        debug!("Using custom install command: {}", install);
        cmd.arg("-i").arg(install);
    }
    if let Some(ref build) = config.build_cmd {
        debug!("Using custom build command: {}", build);
        cmd.arg("-b").arg(build);
    }
    if let Some(ref start) = config.start_cmd {
        debug!("Using custom start command: {}", start);
        cmd.arg("-s").arg(start);
    }
    if let Some(ref packages) = config.packages {
        for pkg in packages {
            debug!("Adding Nix package: {}", pkg);
            cmd.arg("-p").arg(pkg);
        }
    }
    if let Some(ref apt_packages) = config.apt_packages {
        for pkg in apt_packages {
            debug!("Adding apt package: {}", pkg);
            cmd.arg("--apt").arg(pkg);
        }
    }
    if let Some(ref libs) = config.libs {
        for lib in libs {
            debug!("Adding Nix lib: {}", lib);
            cmd.arg("--lib").arg(lib);
        }
    }
    if let Some(ref provider) = config.provider {
        debug!("Forcing provider: {}", provider);
        cmd.arg("--provider").arg(provider);
    }
    if config.no_cache {
        debug!("Disabling build cache");
        cmd.arg("--no-cache");
    }
}

/// Build an image directly using Nixpacks with real-time output streaming
///
/// This builds the Docker image directly without exporting a Dockerfile.
/// Nixpacks uses Docker BuildKit internally.
///
/// # Arguments
///
/// * `source_path` - Path to the source code directory
/// * `image_tag` - Docker image tag to use (e.g., "rivetr-myapp:abc123")
/// * `config` - Optional Nixpacks configuration (from database or API)
/// * `env_vars` - Environment variables to pass to the build
///
/// # Returns
///
/// Returns the image tag on success.
pub async fn build_image(
    source_path: &Path,
    image_tag: &str,
    config: Option<&NixpacksConfig>,
    env_vars: &[(String, String)],
) -> Result<String> {
    info!("Building image with Nixpacks: {}", image_tag);
    debug!("Source path: {:?}", source_path);

    // Check if Nixpacks is available
    if !is_available().await {
        return Err(NixpacksError::NotInstalled.into());
    }

    // Load config from nixpacks.toml if present, and merge with provided config
    let mut effective_config = NixpacksConfig::default();

    // First, load from nixpacks.toml in the repo (lowest priority)
    if let Some(repo_config) = NixpacksConfig::load_from_repo(source_path).await {
        info!("Using nixpacks.toml configuration from repository");
        effective_config.merge(&repo_config);
    }

    // Then merge in the provided config (higher priority - from database/API)
    if let Some(cfg) = config {
        if !cfg.is_empty() {
            info!("Merging custom Nixpacks configuration from app settings");
            effective_config.merge(cfg);
        }
    }

    let mut cmd = Command::new("nixpacks");
    cmd.arg("build")
        .arg(source_path)
        .arg("--name")
        .arg(image_tag);

    // Pass environment variables
    for (key, value) in env_vars {
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

    // Apply configuration
    if !effective_config.is_empty() {
        apply_config_to_command(&mut cmd, &effective_config);
    }

    // Set up for streaming output
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    debug!("Executing Nixpacks command: nixpacks build {:?}", source_path);

    let mut child = cmd
        .spawn()
        .context("Failed to spawn nixpacks process")?;

    // Stream stdout
    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    let mut stdout_lines = stdout_reader.lines();
    let mut stderr_lines = stderr_reader.lines();

    // Collect output for error reporting
    let mut stdout_output = Vec::new();
    let mut stderr_output = Vec::new();

    // Stream output in real-time using tokio::select
    loop {
        tokio::select! {
            stdout_line = stdout_lines.next_line() => {
                match stdout_line {
                    Ok(Some(line)) => {
                        info!(target: "nixpacks", "{}", line);
                        stdout_output.push(line);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        warn!("Error reading nixpacks stdout: {}", e);
                        break;
                    }
                }
            }
            stderr_line = stderr_lines.next_line() => {
                match stderr_line {
                    Ok(Some(line)) => {
                        // Nixpacks outputs progress info to stderr
                        // Only log as warning if it looks like an actual error
                        if line.contains("error") || line.contains("Error") || line.contains("failed") {
                            warn!(target: "nixpacks", "{}", line);
                        } else {
                            info!(target: "nixpacks", "{}", line);
                        }
                        stderr_output.push(line);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        warn!("Error reading nixpacks stderr: {}", e);
                        break;
                    }
                }
            }
            status = child.wait() => {
                match status {
                    Ok(exit_status) => {
                        if exit_status.success() {
                            info!("Nixpacks build completed successfully for {}", image_tag);
                            return Ok(image_tag.to_string());
                        } else {
                            let error_output = stderr_output.join("\n");
                            let error_msg = if error_output.is_empty() {
                                format!("Nixpacks build failed with exit code: {:?}", exit_status.code())
                            } else {
                                // Get the last few lines of stderr for the error message
                                let last_lines: Vec<_> = stderr_output.iter().rev().take(10).collect();
                                format!(
                                    "Nixpacks build failed (exit code {:?}):\n{}",
                                    exit_status.code(),
                                    last_lines.into_iter().rev().cloned().collect::<Vec<_>>().join("\n")
                                )
                            };
                            error!("Nixpacks build failed: {}", error_msg);
                            return Err(NixpacksError::BuildFailed(error_msg).into());
                        }
                    }
                    Err(e) => {
                        return Err(NixpacksError::ExecutionFailed(e.to_string()).into());
                    }
                }
            }
        }
    }

    // If we get here, the streams ended but we didn't get an exit status
    // Wait for the process to complete
    let status = child.wait().await?;
    if status.success() {
        info!("Nixpacks build completed successfully for {}", image_tag);
        Ok(image_tag.to_string())
    } else {
        let error_output = stderr_output.join("\n");
        Err(NixpacksError::BuildFailed(error_output).into())
    }
}

/// Generate a build plan without building (for preview/debugging)
///
/// This is useful for understanding what Nixpacks detected and how it
/// plans to build the project.
pub async fn generate_plan(source_path: &Path) -> Result<String> {
    if !is_available().await {
        return Err(NixpacksError::NotInstalled.into());
    }

    let output = Command::new("nixpacks")
        .arg("plan")
        .arg(source_path)
        .arg("--format")
        .arg("json")
        .output()
        .await
        .context("Failed to execute nixpacks plan")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Could not determine") || stderr.contains("No provider") {
            return Err(NixpacksError::UnsupportedProject.into());
        }
        return Err(NixpacksError::ExecutionFailed(stderr.to_string()).into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Detect the primary language/framework of a project
///
/// Returns the detected provider name (e.g., "node", "python", "rust")
/// or None if no supported language is detected.
pub async fn detect_language(source_path: &Path) -> Option<String> {
    let plan = generate_plan(source_path).await.ok()?;

    // Parse the JSON plan to extract the provider
    #[derive(Deserialize)]
    struct Plan {
        providers: Option<Vec<Provider>>,
    }

    #[derive(Deserialize)]
    struct Provider {
        name: Option<String>,
    }

    let parsed: Plan = serde_json::from_str(&plan).ok()?;

    parsed
        .providers
        .and_then(|p| p.first().and_then(|p| p.name.clone()))
}

/// Generate a Dockerfile using Nixpacks (for inspection)
///
/// This outputs the Dockerfile to a `.nixpacks` directory in the source path.
pub async fn generate_dockerfile(
    source_path: &Path,
    config: Option<&NixpacksConfig>,
) -> Result<String> {
    if !is_available().await {
        return Err(NixpacksError::NotInstalled.into());
    }

    let output_dir = source_path.join(".nixpacks");

    let mut cmd = Command::new("nixpacks");
    cmd.arg("build")
        .arg(source_path)
        .arg("-o")
        .arg(&output_dir);

    // Load config from nixpacks.toml if present
    let mut effective_config = NixpacksConfig::default();
    if let Some(repo_config) = NixpacksConfig::load_from_repo(source_path).await {
        effective_config.merge(&repo_config);
    }
    if let Some(cfg) = config {
        effective_config.merge(cfg);
    }

    if !effective_config.is_empty() {
        apply_config_to_command(&mut cmd, &effective_config);
    }

    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to execute nixpacks")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(NixpacksError::BuildFailed(stderr.to_string()).into());
    }

    // Read the generated Dockerfile
    let dockerfile_path = output_dir.join("Dockerfile");
    if !dockerfile_path.exists() {
        anyhow::bail!("Nixpacks did not generate a Dockerfile");
    }

    tokio::fs::read_to_string(&dockerfile_path)
        .await
        .context("Failed to read generated Dockerfile")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nixpacks_config_json_serialization() {
        let config = NixpacksConfig {
            install_cmd: Some("npm install".to_string()),
            build_cmd: Some("npm run build".to_string()),
            start_cmd: Some("npm start".to_string()),
            packages: Some(vec!["ffmpeg".to_string()]),
            apt_packages: None,
            static_dir: None,
            libs: None,
            provider: None,
            no_cache: false,
        };

        let json = config.to_json().unwrap();
        let parsed = NixpacksConfig::from_json(&json).unwrap();

        assert_eq!(parsed.install_cmd, config.install_cmd);
        assert_eq!(parsed.build_cmd, config.build_cmd);
        assert_eq!(parsed.start_cmd, config.start_cmd);
        assert_eq!(parsed.packages, config.packages);
    }

    #[test]
    fn test_nixpacks_config_toml_parsing() {
        let toml = r#"
            install_cmd = "yarn install"
            build_cmd = "yarn build"
            start_cmd = "yarn start"
            packages = ["imagemagick", "ffmpeg"]
            apt_packages = ["libpng-dev"]
            provider = "node"
        "#;

        let config = NixpacksConfig::from_toml(toml).unwrap();

        assert_eq!(config.install_cmd, Some("yarn install".to_string()));
        assert_eq!(config.build_cmd, Some("yarn build".to_string()));
        assert_eq!(config.start_cmd, Some("yarn start".to_string()));
        assert_eq!(
            config.packages,
            Some(vec!["imagemagick".to_string(), "ffmpeg".to_string()])
        );
        assert_eq!(
            config.apt_packages,
            Some(vec!["libpng-dev".to_string()])
        );
        assert_eq!(config.provider, Some("node".to_string()));
    }

    #[test]
    fn test_empty_config() {
        let config = NixpacksConfig::default();
        let json = config.to_json().unwrap();
        assert_eq!(json, "{}");
        assert!(config.is_empty());
    }

    #[test]
    fn test_config_merge() {
        let mut base = NixpacksConfig {
            install_cmd: Some("npm install".to_string()),
            build_cmd: Some("npm build".to_string()),
            start_cmd: None,
            packages: Some(vec!["pkg1".to_string()]),
            ..Default::default()
        };

        let override_config = NixpacksConfig {
            build_cmd: Some("npm run build:prod".to_string()),
            start_cmd: Some("npm start".to_string()),
            packages: Some(vec!["pkg2".to_string()]),
            no_cache: true,
            ..Default::default()
        };

        base.merge(&override_config);

        // install_cmd should remain from base
        assert_eq!(base.install_cmd, Some("npm install".to_string()));
        // build_cmd should be overridden
        assert_eq!(base.build_cmd, Some("npm run build:prod".to_string()));
        // start_cmd should be set from override
        assert_eq!(base.start_cmd, Some("npm start".to_string()));
        // packages should be overridden (not merged)
        assert_eq!(base.packages, Some(vec!["pkg2".to_string()]));
        // no_cache should be set
        assert!(base.no_cache);
    }

    #[test]
    fn test_config_is_empty() {
        let empty = NixpacksConfig::default();
        assert!(empty.is_empty());

        let with_install = NixpacksConfig {
            install_cmd: Some("npm install".to_string()),
            ..Default::default()
        };
        assert!(!with_install.is_empty());

        let with_no_cache = NixpacksConfig {
            no_cache: true,
            ..Default::default()
        };
        assert!(!with_no_cache.is_empty());
    }
}
