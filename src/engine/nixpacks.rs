//! Nixpacks builder integration for auto-generating Dockerfiles
//!
//! Nixpacks analyzes source code and automatically generates optimized
//! Dockerfiles without requiring manual Dockerfile authoring.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Configuration for Nixpacks builds
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
}

impl NixpacksConfig {
    /// Parse from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to parse nixpacks config")
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).context("Failed to serialize nixpacks config")
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

/// Build an image directly using Nixpacks
///
/// This builds the Docker image directly without exporting a Dockerfile.
/// Nixpacks uses Docker BuildKit internally.
pub async fn build_image(
    source_path: &Path,
    image_tag: &str,
    config: Option<&NixpacksConfig>,
    env_vars: &[(String, String)],
) -> Result<String> {
    info!("Building image with Nixpacks: {}", image_tag);
    debug!("Source path: {:?}", source_path);

    let mut cmd = Command::new("nixpacks");
    cmd.arg("build")
        .arg(source_path)
        .arg("--name")
        .arg(image_tag);

    // Pass environment variables
    for (key, value) in env_vars {
        cmd.arg("--env").arg(format!("{}={}", key, value));
    }

    // Apply custom configuration
    if let Some(cfg) = config {
        if let Some(ref install) = cfg.install_cmd {
            debug!("Using custom install command: {}", install);
            cmd.arg("-i").arg(install);
        }
        if let Some(ref build) = cfg.build_cmd {
            debug!("Using custom build command: {}", build);
            cmd.arg("-b").arg(build);
        }
        if let Some(ref start) = cfg.start_cmd {
            debug!("Using custom start command: {}", start);
            cmd.arg("-s").arg(start);
        }
        if let Some(ref packages) = cfg.packages {
            for pkg in packages {
                debug!("Adding Nix package: {}", pkg);
                cmd.arg("-p").arg(pkg);
            }
        }
        if let Some(ref apt_packages) = cfg.apt_packages {
            for pkg in apt_packages {
                debug!("Adding apt package: {}", pkg);
                cmd.arg("--apt").arg(pkg);
            }
        }
    }

    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to execute nixpacks")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        warn!("Nixpacks build failed: {}", stderr);
        anyhow::bail!("Nixpacks build failed: {}", stderr);
    }

    info!("Nixpacks build completed successfully");
    debug!("Nixpacks output: {}", stdout);

    Ok(image_tag.to_string())
}

/// Generate a build plan without building (for preview/debugging)
pub async fn generate_plan(source_path: &Path) -> Result<String> {
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
        anyhow::bail!("Nixpacks plan failed: {}", stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Generate a Dockerfile using Nixpacks (for inspection)
pub async fn generate_dockerfile(
    source_path: &Path,
    config: Option<&NixpacksConfig>,
) -> Result<String> {
    let output_dir = source_path.join(".nixpacks");

    let mut cmd = Command::new("nixpacks");
    cmd.arg("build")
        .arg(source_path)
        .arg("-o")
        .arg(&output_dir);

    // Apply custom configuration
    if let Some(cfg) = config {
        if let Some(ref install) = cfg.install_cmd {
            cmd.arg("-i").arg(install);
        }
        if let Some(ref build) = cfg.build_cmd {
            cmd.arg("-b").arg(build);
        }
        if let Some(ref start) = cfg.start_cmd {
            cmd.arg("-s").arg(start);
        }
        if let Some(ref packages) = cfg.packages {
            for pkg in packages {
                cmd.arg("-p").arg(pkg);
            }
        }
        if let Some(ref apt_packages) = cfg.apt_packages {
            for pkg in apt_packages {
                cmd.arg("--apt").arg(pkg);
            }
        }
    }

    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to execute nixpacks")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Nixpacks failed: {}", stderr);
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
    fn test_nixpacks_config_serialization() {
        let config = NixpacksConfig {
            install_cmd: Some("npm install".to_string()),
            build_cmd: Some("npm run build".to_string()),
            start_cmd: Some("npm start".to_string()),
            packages: Some(vec!["ffmpeg".to_string()]),
            apt_packages: None,
        };

        let json = config.to_json().unwrap();
        let parsed = NixpacksConfig::from_json(&json).unwrap();

        assert_eq!(parsed.install_cmd, config.install_cmd);
        assert_eq!(parsed.build_cmd, config.build_cmd);
        assert_eq!(parsed.start_cmd, config.start_cmd);
        assert_eq!(parsed.packages, config.packages);
    }

    #[test]
    fn test_empty_config() {
        let config = NixpacksConfig::default();
        let json = config.to_json().unwrap();
        assert_eq!(json, "{}");
    }
}
