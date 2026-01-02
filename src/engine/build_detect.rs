//! Build type auto-detection for Rivetr deployments.
//!
//! This module analyzes repository files to determine the best build strategy
//! for deploying applications.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tracing::{debug, info};

/// Supported build types for applications
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildType {
    /// Build using a Dockerfile
    Dockerfile,
    /// Build using Nixpacks (auto-detect language/framework)
    Nixpacks,
    /// Build using Railpack (Railway's Nixpacks successor)
    Railpack,
    /// Build using Cloud Native Buildpacks (Paketo/Heroku via pack CLI)
    Cnb,
    /// Static site (HTML/CSS/JS served by nginx)
    StaticSite,
    /// Docker Compose multi-container deployment
    DockerCompose,
    /// Pull pre-built image from registry (no build needed)
    DockerImage,
}

impl BuildType {
    /// Convert to string representation used in database
    pub fn as_str(&self) -> &'static str {
        match self {
            BuildType::Dockerfile => "dockerfile",
            BuildType::Nixpacks => "nixpacks",
            BuildType::Railpack => "railpack",
            BuildType::Cnb => "cnb",
            BuildType::StaticSite => "static",
            BuildType::DockerCompose => "docker-compose",
            BuildType::DockerImage => "docker-image",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "dockerfile" => Some(BuildType::Dockerfile),
            "nixpacks" => Some(BuildType::Nixpacks),
            "railpack" => Some(BuildType::Railpack),
            "cnb" | "buildpack" | "buildpacks" | "paketo" | "heroku-cnb" => Some(BuildType::Cnb),
            "static" | "staticsite" | "static-site" => Some(BuildType::StaticSite),
            "docker-compose" | "dockercompose" | "compose" => Some(BuildType::DockerCompose),
            "docker-image" | "dockerimage" | "image" => Some(BuildType::DockerImage),
            _ => None,
        }
    }
}

impl std::fmt::Display for BuildType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Result of build type detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildDetectionResult {
    /// The detected build type
    pub build_type: BuildType,
    /// Publish directory for static sites (e.g., "dist", "build", "out")
    pub publish_directory: Option<String>,
    /// Human-readable explanation of how the build type was detected
    pub detected_from: String,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f32,
    /// Additional recommendations or notes
    pub notes: Option<String>,
}

impl BuildDetectionResult {
    fn new(build_type: BuildType, detected_from: impl Into<String>) -> Self {
        Self {
            build_type,
            publish_directory: None,
            detected_from: detected_from.into(),
            confidence: 1.0,
            notes: None,
        }
    }

    fn with_publish_dir(mut self, dir: impl Into<String>) -> Self {
        self.publish_directory = Some(dir.into());
        self
    }

    fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }

    fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

/// Detect the build type by analyzing files in the source directory
///
/// Detection priority:
/// 1. Dockerfile/Containerfile - highest priority, explicit container definition
/// 2. docker-compose.yml - multi-container deployment
/// 3. railpack.toml - Railpack configuration (Railway's Nixpacks successor)
/// 4. project.toml - Cloud Native Buildpacks configuration
/// 5. nixpacks.toml - Nixpacks configuration
/// 6. Static site indicators - framework-specific detection
/// 7. Language files (package.json, requirements.txt, etc.) - use Nixpacks
pub async fn detect_build_type(source_dir: &Path) -> Result<BuildDetectionResult> {
    info!("Detecting build type for: {:?}", source_dir);

    // 1. Check for Dockerfile/Containerfile first (highest priority)
    if let Some(result) = detect_dockerfile(source_dir).await? {
        info!("Detected: {}", result.detected_from);
        return Ok(result);
    }

    // 2. Check for docker-compose.yml
    if let Some(result) = detect_docker_compose(source_dir).await? {
        info!("Detected: {}", result.detected_from);
        return Ok(result);
    }

    // 3. Check for railpack.toml (Railway's Nixpacks successor)
    if let Some(result) = detect_railpack_config(source_dir).await? {
        info!("Detected: {}", result.detected_from);
        return Ok(result);
    }

    // 4. Check for project.toml (Cloud Native Buildpacks)
    if let Some(result) = detect_cnb_config(source_dir).await? {
        info!("Detected: {}", result.detected_from);
        return Ok(result);
    }

    // 5. Check for nixpacks.toml
    if let Some(result) = detect_nixpacks_config(source_dir).await? {
        info!("Detected: {}", result.detected_from);
        return Ok(result);
    }

    // 6. Check for static site frameworks
    if let Some(result) = detect_static_site(source_dir).await? {
        info!("Detected: {}", result.detected_from);
        return Ok(result);
    }

    // 7. Check for language files that Nixpacks can handle
    if let Some(result) = detect_nixpacks_compatible(source_dir).await? {
        info!("Detected: {}", result.detected_from);
        return Ok(result);
    }

    // 8. Default to Nixpacks with lower confidence - it can often auto-detect
    info!("No specific build type detected, defaulting to Nixpacks");
    Ok(BuildDetectionResult::new(
        BuildType::Nixpacks,
        "No specific build configuration found, using Nixpacks auto-detection",
    )
    .with_confidence(0.5)
    .with_notes("Nixpacks will attempt to auto-detect the project type"))
}

/// Check for railpack.toml configuration file
async fn detect_railpack_config(source_dir: &Path) -> Result<Option<BuildDetectionResult>> {
    let config_path = source_dir.join("railpack.toml");
    if config_path.exists() {
        debug!("Found railpack.toml at {:?}", config_path);
        return Ok(Some(
            BuildDetectionResult::new(BuildType::Railpack, "railpack.toml found")
                .with_notes("Railpack (Railway's Nixpacks successor) will be used for building"),
        ));
    }
    Ok(None)
}

/// Check for project.toml (Cloud Native Buildpacks configuration)
async fn detect_cnb_config(source_dir: &Path) -> Result<Option<BuildDetectionResult>> {
    let config_path = source_dir.join("project.toml");
    if config_path.exists() {
        debug!("Found project.toml (CNB config) at {:?}", config_path);
        return Ok(Some(
            BuildDetectionResult::new(BuildType::Cnb, "project.toml found (Cloud Native Buildpacks)")
                .with_notes("Pack CLI with Paketo/Heroku buildpacks will be used"),
        ));
    }
    Ok(None)
}

/// Check for nixpacks.toml configuration file
async fn detect_nixpacks_config(source_dir: &Path) -> Result<Option<BuildDetectionResult>> {
    let config_path = source_dir.join("nixpacks.toml");
    if config_path.exists() {
        debug!("Found nixpacks.toml at {:?}", config_path);
        return Ok(Some(
            BuildDetectionResult::new(BuildType::Nixpacks, "nixpacks.toml found")
                .with_notes("Nixpacks will use the configuration from nixpacks.toml"),
        ));
    }
    Ok(None)
}

/// Check for Dockerfile or Containerfile
async fn detect_dockerfile(source_dir: &Path) -> Result<Option<BuildDetectionResult>> {
    let dockerfile_names = ["Dockerfile", "dockerfile", "Containerfile", "containerfile"];

    for name in dockerfile_names {
        let path = source_dir.join(name);
        if path.exists() {
            debug!("Found {} at {:?}", name, path);
            return Ok(Some(BuildDetectionResult::new(
                BuildType::Dockerfile,
                format!("{} found", name),
            )));
        }
    }

    // Also check common subdirectories
    let subdirs = ["docker", ".docker", "build"];
    for subdir in subdirs {
        for name in dockerfile_names {
            let path = source_dir.join(subdir).join(name);
            if path.exists() {
                debug!("Found {}/{} at {:?}", subdir, name, path);
                return Ok(Some(
                    BuildDetectionResult::new(
                        BuildType::Dockerfile,
                        format!("{}/{} found", subdir, name),
                    )
                    .with_notes(format!(
                        "Dockerfile path: {}/{}",
                        subdir, name
                    )),
                ));
            }
        }
    }

    Ok(None)
}

/// Check for docker-compose files
async fn detect_docker_compose(source_dir: &Path) -> Result<Option<BuildDetectionResult>> {
    let compose_files = [
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ];

    for name in compose_files {
        let path = source_dir.join(name);
        if path.exists() {
            debug!("Found {} at {:?}", name, path);
            return Ok(Some(BuildDetectionResult::new(
                BuildType::DockerCompose,
                format!("{} found", name),
            )));
        }
    }

    Ok(None)
}

/// Detect static site frameworks and their output directories
async fn detect_static_site(source_dir: &Path) -> Result<Option<BuildDetectionResult>> {
    // Check for Next.js with static export
    if let Some(result) = detect_nextjs_static(source_dir).await? {
        return Ok(Some(result));
    }

    // Check for Astro
    if let Some(result) = detect_astro(source_dir).await? {
        return Ok(Some(result));
    }

    // Check for Vite (without SSR)
    if let Some(result) = detect_vite_static(source_dir).await? {
        return Ok(Some(result));
    }

    // Check for SvelteKit with static adapter
    if let Some(result) = detect_sveltekit_static(source_dir).await? {
        return Ok(Some(result));
    }

    // Check for plain HTML sites
    if let Some(result) = detect_plain_html(source_dir).await? {
        return Ok(Some(result));
    }

    Ok(None)
}

/// Detect Next.js with static export configuration
async fn detect_nextjs_static(source_dir: &Path) -> Result<Option<BuildDetectionResult>> {
    let config_files = ["next.config.js", "next.config.mjs", "next.config.ts"];

    for config_name in config_files {
        let config_path = source_dir.join(config_name);
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .await
                .context(format!("Failed to read {}", config_name))?;

            // Check for output: 'export' or output: "export"
            if content.contains("output:") && content.contains("export") {
                debug!("Found Next.js static export configuration in {}", config_name);
                return Ok(Some(
                    BuildDetectionResult::new(
                        BuildType::StaticSite,
                        format!("Next.js static export detected in {}", config_name),
                    )
                    .with_publish_dir("out")
                    .with_notes("Next.js static export outputs to 'out' directory"),
                ));
            }
        }
    }

    Ok(None)
}

/// Detect Astro framework
async fn detect_astro(source_dir: &Path) -> Result<Option<BuildDetectionResult>> {
    let config_files = ["astro.config.mjs", "astro.config.js", "astro.config.ts"];

    for config_name in config_files {
        let config_path = source_dir.join(config_name);
        if config_path.exists() {
            debug!("Found Astro configuration: {}", config_name);

            let content = fs::read_to_string(&config_path)
                .await
                .context(format!("Failed to read {}", config_name))?;

            // Check if using SSR adapter (not static)
            let is_ssr = content.contains("@astrojs/node")
                || content.contains("@astrojs/vercel")
                || content.contains("@astrojs/netlify")
                || content.contains("@astrojs/cloudflare")
                || (content.contains("output:") && content.contains("server"));

            if is_ssr {
                // Astro with SSR should use Nixpacks
                debug!("Astro SSR detected, recommending Nixpacks");
                return Ok(Some(
                    BuildDetectionResult::new(
                        BuildType::Nixpacks,
                        format!("Astro with SSR adapter detected in {}", config_name),
                    )
                    .with_confidence(0.9)
                    .with_notes("Astro SSR mode requires a Node.js server"),
                ));
            }

            // Static Astro site
            return Ok(Some(
                BuildDetectionResult::new(
                    BuildType::StaticSite,
                    format!("Astro static site detected via {}", config_name),
                )
                .with_publish_dir("dist"),
            ));
        }
    }

    Ok(None)
}

/// Detect Vite without SSR
async fn detect_vite_static(source_dir: &Path) -> Result<Option<BuildDetectionResult>> {
    let config_files = ["vite.config.js", "vite.config.ts", "vite.config.mjs"];

    for config_name in config_files {
        let config_path = source_dir.join(config_name);
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .await
                .context(format!("Failed to read {}", config_name))?;

            // Check for SSR configuration
            let is_ssr = content.contains("ssr:")
                && (content.contains("ssr: true") || content.contains("ssr: {"));

            if is_ssr {
                debug!("Vite SSR detected, recommending Nixpacks");
                return Ok(Some(
                    BuildDetectionResult::new(
                        BuildType::Nixpacks,
                        format!("Vite with SSR detected in {}", config_name),
                    )
                    .with_confidence(0.9)
                    .with_notes("Vite SSR mode requires a Node.js server"),
                ));
            }

            // Check for custom build output directory
            let publish_dir = if content.contains("outDir:") {
                // Try to extract outDir value - simplified regex-free approach
                if content.contains("outDir: 'build'") || content.contains("outDir: \"build\"") {
                    "build"
                } else if content.contains("outDir: 'public'")
                    || content.contains("outDir: \"public\"")
                {
                    "public"
                } else {
                    "dist" // default
                }
            } else {
                "dist" // Vite default
            };

            debug!("Found Vite static site configuration");
            return Ok(Some(
                BuildDetectionResult::new(
                    BuildType::StaticSite,
                    format!("Vite static site detected via {}", config_name),
                )
                .with_publish_dir(publish_dir),
            ));
        }
    }

    Ok(None)
}

/// Detect SvelteKit with static adapter
async fn detect_sveltekit_static(source_dir: &Path) -> Result<Option<BuildDetectionResult>> {
    let config_path = source_dir.join("svelte.config.js");
    if !config_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&config_path)
        .await
        .context("Failed to read svelte.config.js")?;

    // Check for static adapter
    if content.contains("@sveltejs/adapter-static") {
        debug!("Found SvelteKit with static adapter");
        return Ok(Some(
            BuildDetectionResult::new(
                BuildType::StaticSite,
                "SvelteKit with static adapter detected",
            )
            .with_publish_dir("build"),
        ));
    }

    // SvelteKit with other adapters should use Nixpacks
    if content.contains("@sveltejs/adapter-node")
        || content.contains("@sveltejs/adapter-auto")
        || content.contains("@sveltejs/adapter-vercel")
    {
        debug!("Found SvelteKit with server adapter");
        return Ok(Some(
            BuildDetectionResult::new(
                BuildType::Nixpacks,
                "SvelteKit with server adapter detected",
            )
            .with_confidence(0.9)
            .with_notes("SvelteKit with server adapter requires Node.js"),
        ));
    }

    Ok(None)
}

/// Detect plain HTML sites (no build step needed)
async fn detect_plain_html(source_dir: &Path) -> Result<Option<BuildDetectionResult>> {
    let index_path = source_dir.join("index.html");
    let package_json_path = source_dir.join("package.json");

    // Must have index.html but no package.json (to avoid detecting JS projects)
    if index_path.exists() && !package_json_path.exists() {
        debug!("Found plain HTML site (index.html without package.json)");
        return Ok(Some(
            BuildDetectionResult::new(BuildType::StaticSite, "Plain HTML site detected (index.html found)")
                .with_publish_dir(".")
                .with_notes("No build step required"),
        ));
    }

    Ok(None)
}

/// Detect language/framework files that Nixpacks can handle
async fn detect_nixpacks_compatible(source_dir: &Path) -> Result<Option<BuildDetectionResult>> {
    // Check for Node.js
    if source_dir.join("package.json").exists() {
        let publish_dir = detect_nodejs_publish_dir(source_dir).await?;

        // If we can determine it's a static build, prefer that
        if let Some(dir) = &publish_dir {
            debug!("Node.js project with static output to: {}", dir);
            return Ok(Some(
                BuildDetectionResult::new(BuildType::Nixpacks, "Node.js project detected (package.json)")
                    .with_publish_dir(dir.clone())
                    .with_notes("Build output directory detected from package.json"),
            ));
        }

        debug!("Found Node.js project (package.json)");
        return Ok(Some(BuildDetectionResult::new(
            BuildType::Nixpacks,
            "Node.js project detected (package.json)",
        )));
    }

    // Check for Python
    let python_files = ["requirements.txt", "pyproject.toml", "setup.py", "Pipfile"];
    for file in python_files {
        if source_dir.join(file).exists() {
            debug!("Found Python project ({})", file);
            return Ok(Some(BuildDetectionResult::new(
                BuildType::Nixpacks,
                format!("Python project detected ({})", file),
            )));
        }
    }

    // Check for Go
    if source_dir.join("go.mod").exists() {
        debug!("Found Go project (go.mod)");
        return Ok(Some(BuildDetectionResult::new(
            BuildType::Nixpacks,
            "Go project detected (go.mod)",
        )));
    }

    // Check for Rust
    if source_dir.join("Cargo.toml").exists() {
        debug!("Found Rust project (Cargo.toml)");
        return Ok(Some(BuildDetectionResult::new(
            BuildType::Nixpacks,
            "Rust project detected (Cargo.toml)",
        )));
    }

    // Check for Ruby
    if source_dir.join("Gemfile").exists() {
        debug!("Found Ruby project (Gemfile)");
        return Ok(Some(BuildDetectionResult::new(
            BuildType::Nixpacks,
            "Ruby project detected (Gemfile)",
        )));
    }

    // Check for PHP
    if source_dir.join("composer.json").exists() {
        debug!("Found PHP project (composer.json)");
        return Ok(Some(BuildDetectionResult::new(
            BuildType::Nixpacks,
            "PHP project detected (composer.json)",
        )));
    }

    // Check for Java/Kotlin
    let java_files = ["pom.xml", "build.gradle", "build.gradle.kts"];
    for file in java_files {
        if source_dir.join(file).exists() {
            debug!("Found Java/Kotlin project ({})", file);
            return Ok(Some(BuildDetectionResult::new(
                BuildType::Nixpacks,
                format!("Java/Kotlin project detected ({})", file),
            )));
        }
    }

    // Check for .NET
    let dotnet_patterns = ["*.csproj", "*.fsproj", "*.vbproj"];
    for pattern in dotnet_patterns {
        if has_file_matching(source_dir, pattern).await? {
            debug!("Found .NET project");
            return Ok(Some(BuildDetectionResult::new(
                BuildType::Nixpacks,
                ".NET project detected",
            )));
        }
    }

    // Check for Elixir
    if source_dir.join("mix.exs").exists() {
        debug!("Found Elixir project (mix.exs)");
        return Ok(Some(BuildDetectionResult::new(
            BuildType::Nixpacks,
            "Elixir project detected (mix.exs)",
        )));
    }

    Ok(None)
}

/// Check if directory has any file matching a simple glob pattern
async fn has_file_matching(dir: &Path, pattern: &str) -> Result<bool> {
    let extension = pattern.trim_start_matches("*.");

    let mut entries = fs::read_dir(dir)
        .await
        .context("Failed to read directory")?;

    while let Some(entry) = entries.next_entry().await? {
        if let Some(ext) = entry.path().extension() {
            if ext == extension {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

/// Try to detect the publish directory from Node.js project configuration
async fn detect_nodejs_publish_dir(source_dir: &Path) -> Result<Option<String>> {
    let package_json_path = source_dir.join("package.json");
    if !package_json_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&package_json_path)
        .await
        .context("Failed to read package.json")?;

    let package: serde_json::Value =
        serde_json::from_str(&content).context("Failed to parse package.json")?;

    // Check scripts for hints about output directories
    if let Some(scripts) = package.get("scripts").and_then(|s| s.as_object()) {
        // Check build script for output hints
        if let Some(build_script) = scripts.get("build").and_then(|s| s.as_str()) {
            // Common patterns in build scripts
            if build_script.contains("--outDir=build")
                || build_script.contains("--out-dir=build")
                || build_script.contains("-o build")
            {
                return Ok(Some("build".to_string()));
            }
            if build_script.contains("--outDir=public") || build_script.contains("-o public") {
                return Ok(Some("public".to_string()));
            }
            if build_script.contains("--outDir=out") || build_script.contains("-o out") {
                return Ok(Some("out".to_string()));
            }
        }
    }

    // Check for common output directories that exist (for existing builds)
    let common_dirs = ["dist", "build", "out", "public", ".next/out"];
    for dir in common_dirs {
        let dir_path = source_dir.join(dir);
        if dir_path.exists() && dir_path.is_dir() {
            // Only suggest if it looks like a build output
            let has_index = dir_path.join("index.html").exists();
            if has_index {
                return Ok(Some(dir.to_string()));
            }
        }
    }

    Ok(None)
}

/// Detect publish directory by checking for common build output directories
pub async fn detect_publish_directory(source_dir: &Path) -> Result<Option<String>> {
    // Priority order for common static site output directories
    let candidates = [
        ("dist", true),      // Vite, Rollup, esbuild default
        ("build", true),     // Create React App, SvelteKit static
        ("out", true),       // Next.js static export
        ("public", false),   // Less common, check for index.html
        (".next/out", true), // Next.js export output
        ("_site", true),     // Jekyll, Eleventy
    ];

    for (dir, default_check) in candidates {
        let dir_path = source_dir.join(dir);
        if dir_path.exists() && dir_path.is_dir() {
            if default_check {
                return Ok(Some(dir.to_string()));
            } else {
                // Only return if it has index.html
                if dir_path.join("index.html").exists() {
                    return Ok(Some(dir.to_string()));
                }
            }
        }
    }

    // Check if source_dir itself is a static site
    if source_dir.join("index.html").exists() {
        return Ok(Some(".".to_string()));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_build_type_from_str() {
        assert_eq!(
            BuildType::from_str("dockerfile"),
            Some(BuildType::Dockerfile)
        );
        assert_eq!(BuildType::from_str("nixpacks"), Some(BuildType::Nixpacks));
        assert_eq!(BuildType::from_str("railpack"), Some(BuildType::Railpack));
        assert_eq!(BuildType::from_str("cnb"), Some(BuildType::Cnb));
        assert_eq!(BuildType::from_str("paketo"), Some(BuildType::Cnb));
        assert_eq!(BuildType::from_str("buildpacks"), Some(BuildType::Cnb));
        assert_eq!(BuildType::from_str("static"), Some(BuildType::StaticSite));
        assert_eq!(
            BuildType::from_str("docker-compose"),
            Some(BuildType::DockerCompose)
        );
        assert_eq!(
            BuildType::from_str("docker-image"),
            Some(BuildType::DockerImage)
        );
        assert_eq!(BuildType::from_str("invalid"), None);
    }

    #[test]
    fn test_build_type_display() {
        assert_eq!(BuildType::Dockerfile.to_string(), "dockerfile");
        assert_eq!(BuildType::Nixpacks.to_string(), "nixpacks");
        assert_eq!(BuildType::Railpack.to_string(), "railpack");
        assert_eq!(BuildType::Cnb.to_string(), "cnb");
        assert_eq!(BuildType::StaticSite.to_string(), "static");
        assert_eq!(BuildType::DockerCompose.to_string(), "docker-compose");
        assert_eq!(BuildType::DockerImage.to_string(), "docker-image");
    }

    #[tokio::test]
    async fn test_detect_railpack_config() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a railpack.toml
        std::fs::write(
            temp_path.join("railpack.toml"),
            r#"
install_cmd = "npm install"
build_cmd = "npm run build"
"#,
        )
        .unwrap();

        let result = detect_build_type(temp_path).await.unwrap();
        assert_eq!(result.build_type, BuildType::Railpack);
        assert!(result.detected_from.contains("railpack.toml"));
    }

    #[tokio::test]
    async fn test_detect_cnb_config() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a project.toml (CNB config)
        std::fs::write(
            temp_path.join("project.toml"),
            r#"
[project]
name = "my-app"
version = "1.0.0"
"#,
        )
        .unwrap();

        let result = detect_build_type(temp_path).await.unwrap();
        assert_eq!(result.build_type, BuildType::Cnb);
        assert!(result.detected_from.contains("project.toml"));
    }

    #[tokio::test]
    async fn test_detect_nixpacks_config() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a nixpacks.toml
        std::fs::write(
            temp_path.join("nixpacks.toml"),
            r#"
install_cmd = "yarn install"
build_cmd = "yarn build"
"#,
        )
        .unwrap();

        let result = detect_build_type(temp_path).await.unwrap();
        assert_eq!(result.build_type, BuildType::Nixpacks);
        assert!(result.detected_from.contains("nixpacks.toml"));
    }

    #[tokio::test]
    async fn test_detect_dockerfile() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a Dockerfile
        std::fs::write(temp_path.join("Dockerfile"), "FROM alpine").unwrap();

        let result = detect_build_type(temp_path).await.unwrap();
        assert_eq!(result.build_type, BuildType::Dockerfile);
        assert!(result.detected_from.contains("Dockerfile"));
    }

    #[tokio::test]
    async fn test_detect_docker_compose() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a docker-compose.yml
        std::fs::write(temp_path.join("docker-compose.yml"), "version: '3'").unwrap();

        let result = detect_build_type(temp_path).await.unwrap();
        assert_eq!(result.build_type, BuildType::DockerCompose);
    }

    #[tokio::test]
    async fn test_detect_nodejs() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a package.json
        std::fs::write(
            temp_path.join("package.json"),
            r#"{"name": "test", "version": "1.0.0"}"#,
        )
        .unwrap();

        let result = detect_build_type(temp_path).await.unwrap();
        assert_eq!(result.build_type, BuildType::Nixpacks);
        assert!(result.detected_from.contains("Node.js"));
    }

    #[tokio::test]
    async fn test_detect_python() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a requirements.txt
        std::fs::write(temp_path.join("requirements.txt"), "flask==2.0").unwrap();

        let result = detect_build_type(temp_path).await.unwrap();
        assert_eq!(result.build_type, BuildType::Nixpacks);
        assert!(result.detected_from.contains("Python"));
    }

    #[tokio::test]
    async fn test_detect_rust() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a Cargo.toml
        std::fs::write(
            temp_path.join("Cargo.toml"),
            r#"[package]
name = "test"
version = "0.1.0"
"#,
        )
        .unwrap();

        let result = detect_build_type(temp_path).await.unwrap();
        assert_eq!(result.build_type, BuildType::Nixpacks);
        assert!(result.detected_from.contains("Rust"));
    }

    #[tokio::test]
    async fn test_detect_go() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a go.mod
        std::fs::write(temp_path.join("go.mod"), "module test\n\ngo 1.21").unwrap();

        let result = detect_build_type(temp_path).await.unwrap();
        assert_eq!(result.build_type, BuildType::Nixpacks);
        assert!(result.detected_from.contains("Go"));
    }

    #[tokio::test]
    async fn test_detect_plain_html() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create an index.html (no package.json)
        std::fs::write(temp_path.join("index.html"), "<html></html>").unwrap();

        let result = detect_build_type(temp_path).await.unwrap();
        assert_eq!(result.build_type, BuildType::StaticSite);
        assert_eq!(result.publish_directory, Some(".".to_string()));
    }

    #[tokio::test]
    async fn test_detect_nextjs_static() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create next.config.js with static export
        std::fs::write(
            temp_path.join("next.config.js"),
            r#"
module.exports = {
    output: 'export',
    trailingSlash: true,
}
"#,
        )
        .unwrap();

        let result = detect_build_type(temp_path).await.unwrap();
        assert_eq!(result.build_type, BuildType::StaticSite);
        assert_eq!(result.publish_directory, Some("out".to_string()));
    }

    #[tokio::test]
    async fn test_detect_astro() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create astro.config.mjs
        std::fs::write(
            temp_path.join("astro.config.mjs"),
            r#"
import { defineConfig } from 'astro/config';
export default defineConfig({});
"#,
        )
        .unwrap();

        let result = detect_build_type(temp_path).await.unwrap();
        assert_eq!(result.build_type, BuildType::StaticSite);
        assert_eq!(result.publish_directory, Some("dist".to_string()));
    }

    #[tokio::test]
    async fn test_detect_vite() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create vite.config.js
        std::fs::write(
            temp_path.join("vite.config.js"),
            r#"
import { defineConfig } from 'vite';
export default defineConfig({});
"#,
        )
        .unwrap();

        let result = detect_build_type(temp_path).await.unwrap();
        assert_eq!(result.build_type, BuildType::StaticSite);
        assert_eq!(result.publish_directory, Some("dist".to_string()));
    }

    #[tokio::test]
    async fn test_dockerfile_takes_priority() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create both Dockerfile and package.json
        std::fs::write(temp_path.join("Dockerfile"), "FROM node:18").unwrap();
        std::fs::write(
            temp_path.join("package.json"),
            r#"{"name": "test"}"#,
        )
        .unwrap();

        let result = detect_build_type(temp_path).await.unwrap();
        // Dockerfile should take priority
        assert_eq!(result.build_type, BuildType::Dockerfile);
    }

    #[tokio::test]
    async fn test_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Empty directory should default to Nixpacks with lower confidence
        let result = detect_build_type(temp_path).await.unwrap();
        assert_eq!(result.build_type, BuildType::Nixpacks);
        assert!(result.confidence < 1.0);
    }
}
