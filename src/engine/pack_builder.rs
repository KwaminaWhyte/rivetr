//! Cloud Native Buildpacks (CNB) integration using the Pack CLI
//!
//! This module supports building container images using the Pack CLI with
//! various builders including Paketo and Heroku Cloud Native Buildpacks.
//!
//! # Supported Builders
//!
//! - **Paketo** - `paketobuildpacks/builder-jammy-base` (default)
//! - **Heroku** - `heroku/builder:24`
//!
//! # Features
//!
//! - No Dockerfile required - automatic language detection
//! - Production-ready, security-focused images
//! - Reproducible builds across environments
//! - Automatic base image updates
//!
//! # Requirements
//!
//! - Pack CLI installed: https://buildpacks.io/docs/tools/pack/
//! - Docker daemon running

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{debug, error, info, warn};

/// Errors specific to Pack CLI operations
#[derive(Error, Debug)]
pub enum PackError {
    #[error("Pack CLI is not installed. Install from: https://buildpacks.io/docs/tools/pack/")]
    NotInstalled,

    #[error("Docker is not available for Pack builds")]
    DockerNotAvailable,

    #[error("Failed to execute Pack command: {0}")]
    ExecutionFailed(String),

    #[error("Pack build failed: {0}")]
    BuildFailed(String),

    #[error("Invalid builder specified: {0}")]
    InvalidBuilder(String),

    #[error("Unsupported project type")]
    UnsupportedProject,
}

/// Available Cloud Native Buildpack builders
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CnbBuilder {
    /// Paketo Base builder - good balance of features and size
    #[default]
    PaketoBase,
    /// Paketo Full builder - includes more dependencies
    PaketoFull,
    /// Paketo Tiny builder - minimal footprint
    PaketoTiny,
    /// Heroku builder (version 24)
    Heroku24,
    /// Heroku builder (version 22)
    Heroku22,
    /// Custom builder image
    Custom,
}

impl CnbBuilder {
    /// Get the Docker image reference for this builder
    pub fn image(&self) -> &'static str {
        match self {
            CnbBuilder::PaketoBase => "paketobuildpacks/builder-jammy-base",
            CnbBuilder::PaketoFull => "paketobuildpacks/builder-jammy-full",
            CnbBuilder::PaketoTiny => "paketobuildpacks/builder-jammy-tiny",
            CnbBuilder::Heroku24 => "heroku/builder:24",
            CnbBuilder::Heroku22 => "heroku/builder:22",
            CnbBuilder::Custom => "", // Custom builder uses config.custom_builder
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "paketo" | "paketo-base" | "paketobuildpacks/builder-jammy-base" => {
                Some(CnbBuilder::PaketoBase)
            }
            "paketo-full" | "paketobuildpacks/builder-jammy-full" => Some(CnbBuilder::PaketoFull),
            "paketo-tiny" | "paketobuildpacks/builder-jammy-tiny" => Some(CnbBuilder::PaketoTiny),
            "heroku" | "heroku24" | "heroku/builder:24" => Some(CnbBuilder::Heroku24),
            "heroku22" | "heroku/builder:22" => Some(CnbBuilder::Heroku22),
            "custom" => Some(CnbBuilder::Custom),
            _ => None,
        }
    }

    /// Convert to display string
    pub fn as_str(&self) -> &'static str {
        match self {
            CnbBuilder::PaketoBase => "paketo-base",
            CnbBuilder::PaketoFull => "paketo-full",
            CnbBuilder::PaketoTiny => "paketo-tiny",
            CnbBuilder::Heroku24 => "heroku24",
            CnbBuilder::Heroku22 => "heroku22",
            CnbBuilder::Custom => "custom",
        }
    }
}

impl std::fmt::Display for CnbBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Configuration for Pack CLI builds
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackConfig {
    /// Which CNB builder to use
    #[serde(default)]
    pub builder: CnbBuilder,

    /// Custom builder image (when builder is Custom)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_builder: Option<String>,

    /// Additional buildpacks to include
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buildpacks: Option<Vec<String>>,

    /// Build-time environment variables
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_env: Option<Vec<(String, String)>>,

    /// Disable build cache
    #[serde(default, skip_serializing_if = "is_false")]
    pub clear_cache: bool,

    /// Trust the builder (skip confirmation)
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub trust_builder: bool,

    /// Pull policy for builder image (always, if-not-present, never)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pull_policy: Option<String>,

    /// Path to project.toml file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub descriptor: Option<String>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn is_true(b: &bool) -> bool {
    *b
}

fn default_true() -> bool {
    true
}

impl PackConfig {
    /// Parse from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to parse pack config from JSON")
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).context("Failed to serialize pack config to JSON")
    }

    /// Get the effective builder image
    pub fn get_builder_image(&self) -> Result<String> {
        if self.builder == CnbBuilder::Custom {
            self.custom_builder
                .clone()
                .ok_or_else(|| PackError::InvalidBuilder("Custom builder specified but no image provided".into()).into())
        } else {
            Ok(self.builder.image().to_string())
        }
    }

    /// Load configuration from a project.toml file in the given directory
    pub async fn load_from_repo(source_path: &Path) -> Option<Self> {
        let toml_path = source_path.join("project.toml");

        if !toml_path.exists() {
            debug!("No project.toml found at {:?}", toml_path);
            return None;
        }

        // project.toml is CNB's config format - we just note it exists
        // Pack CLI will read it automatically
        info!("Found project.toml configuration at {:?}", toml_path);
        Some(PackConfig {
            descriptor: Some("project.toml".to_string()),
            ..Default::default()
        })
    }

    /// Merge another config into this one (other takes precedence)
    pub fn merge(&mut self, other: &PackConfig) {
        // Only override builder if explicitly set (not default)
        if other.builder != CnbBuilder::PaketoBase || other.custom_builder.is_some() {
            self.builder = other.builder;
        }
        if other.custom_builder.is_some() {
            self.custom_builder = other.custom_builder.clone();
        }
        if other.buildpacks.is_some() {
            self.buildpacks = other.buildpacks.clone();
        }
        if other.build_env.is_some() {
            self.build_env = other.build_env.clone();
        }
        if other.clear_cache {
            self.clear_cache = true;
        }
        if !other.trust_builder {
            self.trust_builder = false;
        }
        if other.pull_policy.is_some() {
            self.pull_policy = other.pull_policy.clone();
        }
        if other.descriptor.is_some() {
            self.descriptor = other.descriptor.clone();
        }
    }

    /// Check if this config has any custom settings
    pub fn is_empty(&self) -> bool {
        self.builder == CnbBuilder::PaketoBase
            && self.custom_builder.is_none()
            && self.buildpacks.is_none()
            && self.build_env.is_none()
            && !self.clear_cache
            && self.trust_builder
            && self.pull_policy.is_none()
            && self.descriptor.is_none()
    }
}

/// Check if Pack CLI is available on the system
pub async fn is_available() -> bool {
    Command::new("pack")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Get Pack CLI version if available
pub async fn get_version() -> Option<String> {
    let output = Command::new("pack")
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

/// Check if a project.toml configuration file exists in the source directory
pub async fn has_config_file(source_path: &Path) -> bool {
    source_path.join("project.toml").exists()
}

/// Build an image using Pack CLI with real-time output streaming
///
/// # Arguments
///
/// * `source_path` - Path to the source code directory
/// * `image_tag` - Docker image tag to use (e.g., "rivetr-myapp:abc123")
/// * `config` - Optional Pack configuration (from database or API)
/// * `env_vars` - Environment variables to pass to the build
///
/// # Returns
///
/// Returns the image tag on success.
pub async fn build_image(
    source_path: &Path,
    image_tag: &str,
    config: Option<&PackConfig>,
    env_vars: &[(String, String)],
) -> Result<String> {
    info!("Building image with Pack CLI: {}", image_tag);
    debug!("Source path: {:?}", source_path);

    // Check if Pack CLI is available
    if !is_available().await {
        return Err(PackError::NotInstalled.into());
    }

    // Load config from project.toml if present, and merge with provided config
    let mut effective_config = PackConfig::default();

    // First, load from project.toml in the repo (lowest priority)
    if let Some(repo_config) = PackConfig::load_from_repo(source_path).await {
        effective_config.merge(&repo_config);
    }

    // Then merge in the provided config (higher priority - from database/API)
    if let Some(cfg) = config {
        if !cfg.is_empty() {
            info!("Merging custom Pack configuration from app settings");
            effective_config.merge(cfg);
        }
    }

    // Get the builder image
    let builder_image = effective_config.get_builder_image()?;
    info!("Using CNB builder: {}", builder_image);

    let mut cmd = Command::new("pack");
    cmd.arg("build")
        .arg(image_tag)
        .arg("--builder")
        .arg(&builder_image)
        .arg("--path")
        .arg(source_path);

    // Trust the builder to avoid prompts
    if effective_config.trust_builder {
        cmd.arg("--trust-builder");
    }

    // Clear cache if requested
    if effective_config.clear_cache {
        debug!("Clearing build cache");
        cmd.arg("--clear-cache");
    }

    // Set pull policy
    if let Some(ref policy) = effective_config.pull_policy {
        cmd.arg("--pull-policy").arg(policy);
    }

    // Add additional buildpacks
    if let Some(ref buildpacks) = effective_config.buildpacks {
        for bp in buildpacks {
            debug!("Adding buildpack: {}", bp);
            cmd.arg("--buildpack").arg(bp);
        }
    }

    // Add project descriptor if specified
    if let Some(ref descriptor) = effective_config.descriptor {
        let descriptor_path = source_path.join(descriptor);
        if descriptor_path.exists() {
            cmd.arg("--descriptor").arg(&descriptor_path);
        }
    }

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

    // Add build-time env vars from config
    if let Some(ref build_env) = effective_config.build_env {
        for (key, value) in build_env {
            cmd.arg("--env").arg(format!("{}={}", key, value));
        }
    }

    // Set up for streaming output
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    debug!(
        "Executing Pack command: pack build {} --builder {}",
        image_tag, builder_image
    );

    let mut child = cmd.spawn().context("Failed to spawn pack process")?;

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
                        info!(target: "pack", "{}", line);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        warn!("Error reading pack stdout: {}", e);
                        break;
                    }
                }
            }
            stderr_line = stderr_lines.next_line() => {
                match stderr_line {
                    Ok(Some(line)) => {
                        // Pack outputs progress info to stderr
                        if line.contains("ERROR") || line.contains("error") || line.contains("failed") {
                            warn!(target: "pack", "{}", line);
                        } else {
                            info!(target: "pack", "{}", line);
                        }
                        stderr_output.push(line);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        warn!("Error reading pack stderr: {}", e);
                        break;
                    }
                }
            }
            status = child.wait() => {
                match status {
                    Ok(exit_status) => {
                        if exit_status.success() {
                            info!("Pack build completed successfully for {}", image_tag);
                            return Ok(image_tag.to_string());
                        } else {
                            let error_output = stderr_output.join("\n");
                            let error_msg = if error_output.is_empty() {
                                format!("Pack build failed with exit code: {:?}", exit_status.code())
                            } else {
                                // Get the last few lines of stderr for the error message
                                let last_lines: Vec<_> = stderr_output.iter().rev().take(15).collect();
                                format!(
                                    "Pack build failed (exit code {:?}):\n{}",
                                    exit_status.code(),
                                    last_lines.into_iter().rev().cloned().collect::<Vec<_>>().join("\n")
                                )
                            };
                            error!("Pack build failed: {}", error_msg);
                            return Err(PackError::BuildFailed(error_msg).into());
                        }
                    }
                    Err(e) => {
                        return Err(PackError::ExecutionFailed(e.to_string()).into());
                    }
                }
            }
        }
    }

    // If we get here, the streams ended but we didn't get an exit status
    let status = child.wait().await?;
    if status.success() {
        info!("Pack build completed successfully for {}", image_tag);
        Ok(image_tag.to_string())
    } else {
        let error_output = stderr_output.join("\n");
        Err(PackError::BuildFailed(error_output).into())
    }
}

/// Suggest the best builder for a project based on its files
pub async fn suggest_builder(source_path: &Path) -> CnbBuilder {
    // Check for Java projects - Paketo has excellent Java support
    if source_path.join("pom.xml").exists()
        || source_path.join("build.gradle").exists()
        || source_path.join("build.gradle.kts").exists()
    {
        return CnbBuilder::PaketoFull; // Java often needs more dependencies
    }

    // Check for .NET projects - Paketo has good .NET support
    let has_dotnet = tokio::fs::read_dir(source_path)
        .await
        .map(|mut entries| {
            futures::executor::block_on(async {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if let Some(ext) = entry.path().extension() {
                        if ext == "csproj" || ext == "fsproj" {
                            return true;
                        }
                    }
                }
                false
            })
        })
        .unwrap_or(false);

    if has_dotnet {
        return CnbBuilder::PaketoBase;
    }

    // For Node.js, Python, Go, Ruby - Heroku builder works well
    if source_path.join("package.json").exists()
        || source_path.join("requirements.txt").exists()
        || source_path.join("go.mod").exists()
        || source_path.join("Gemfile").exists()
    {
        return CnbBuilder::Heroku24;
    }

    // Default to Paketo Base - good general purpose
    CnbBuilder::PaketoBase
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cnb_builder_from_str() {
        assert_eq!(CnbBuilder::from_str("paketo"), Some(CnbBuilder::PaketoBase));
        assert_eq!(
            CnbBuilder::from_str("paketo-full"),
            Some(CnbBuilder::PaketoFull)
        );
        assert_eq!(CnbBuilder::from_str("heroku"), Some(CnbBuilder::Heroku24));
        assert_eq!(CnbBuilder::from_str("heroku24"), Some(CnbBuilder::Heroku24));
        assert_eq!(CnbBuilder::from_str("heroku22"), Some(CnbBuilder::Heroku22));
        assert_eq!(CnbBuilder::from_str("invalid"), None);
    }

    #[test]
    fn test_cnb_builder_image() {
        assert_eq!(
            CnbBuilder::PaketoBase.image(),
            "paketobuildpacks/builder-jammy-base"
        );
        assert_eq!(CnbBuilder::Heroku24.image(), "heroku/builder:24");
    }

    #[test]
    fn test_pack_config_json_serialization() {
        let config = PackConfig {
            builder: CnbBuilder::Heroku24,
            custom_builder: None,
            buildpacks: Some(vec!["heroku/nodejs".to_string()]),
            build_env: None,
            clear_cache: false,
            trust_builder: true,
            pull_policy: Some("if-not-present".to_string()),
            descriptor: None,
        };

        let json = config.to_json().unwrap();
        let parsed = PackConfig::from_json(&json).unwrap();

        assert_eq!(parsed.builder, CnbBuilder::Heroku24);
        assert_eq!(parsed.buildpacks, config.buildpacks);
        assert_eq!(parsed.pull_policy, config.pull_policy);
    }

    #[test]
    fn test_pack_config_get_builder_image() {
        let paketo_config = PackConfig {
            builder: CnbBuilder::PaketoBase,
            ..Default::default()
        };
        assert_eq!(
            paketo_config.get_builder_image().unwrap(),
            "paketobuildpacks/builder-jammy-base"
        );

        let heroku_config = PackConfig {
            builder: CnbBuilder::Heroku24,
            ..Default::default()
        };
        assert_eq!(
            heroku_config.get_builder_image().unwrap(),
            "heroku/builder:24"
        );

        let custom_config = PackConfig {
            builder: CnbBuilder::Custom,
            custom_builder: Some("my-org/my-builder:latest".to_string()),
            ..Default::default()
        };
        assert_eq!(
            custom_config.get_builder_image().unwrap(),
            "my-org/my-builder:latest"
        );
    }

    #[test]
    fn test_config_merge() {
        let mut base = PackConfig::default();

        let override_config = PackConfig {
            builder: CnbBuilder::Heroku24,
            buildpacks: Some(vec!["heroku/nodejs".to_string()]),
            clear_cache: true,
            ..Default::default()
        };

        base.merge(&override_config);

        assert_eq!(base.builder, CnbBuilder::Heroku24);
        assert_eq!(base.buildpacks, Some(vec!["heroku/nodejs".to_string()]));
        assert!(base.clear_cache);
    }

    #[test]
    fn test_empty_config() {
        let config = PackConfig::default();
        assert!(config.is_empty());

        let config_with_buildpacks = PackConfig {
            buildpacks: Some(vec!["test".to_string()]),
            ..Default::default()
        };
        assert!(!config_with_buildpacks.is_empty());
    }
}
