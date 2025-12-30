//! Static Site Builder
//!
//! Generates Docker images for static frontend sites (Astro, Next.js static export,
//! Vite, Create React App, etc.) using NGINX as the web server.
//!
//! The builder:
//! - Auto-detects package managers (npm, yarn, pnpm, bun)
//! - Generates optimized multi-stage Dockerfiles
//! - Configures NGINX for SPA routing with try_files
//! - Supports custom build commands and publish directories

use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tracing::{debug, info};

use crate::runtime::{BuildContext, ContainerRuntime};

/// Package manager detected from lock files
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Npm,
    Yarn,
    Pnpm,
    Bun,
}

impl PackageManager {
    /// Returns the install command for this package manager
    pub fn install_command(&self) -> &'static str {
        match self {
            PackageManager::Npm => "npm ci",
            PackageManager::Yarn => "yarn install --frozen-lockfile",
            PackageManager::Pnpm => "pnpm install --frozen-lockfile",
            PackageManager::Bun => "bun install --frozen-lockfile",
        }
    }

    /// Returns the build command for this package manager
    pub fn build_command(&self) -> &'static str {
        match self {
            PackageManager::Npm => "npm run build",
            PackageManager::Yarn => "yarn build",
            PackageManager::Pnpm => "pnpm build",
            PackageManager::Bun => "bun run build",
        }
    }

    /// Returns the base image to use for building
    pub fn base_image(&self) -> &'static str {
        match self {
            PackageManager::Bun => "oven/bun:1-alpine",
            _ => "node:20-alpine",
        }
    }

    /// Returns the lock file name for this package manager
    pub fn lock_file(&self) -> &'static str {
        match self {
            PackageManager::Npm => "package-lock.json",
            PackageManager::Yarn => "yarn.lock",
            PackageManager::Pnpm => "pnpm-lock.yaml",
            PackageManager::Bun => "bun.lockb",
        }
    }

    /// Returns files to copy for caching dependencies
    pub fn dependency_files(&self) -> Vec<&'static str> {
        match self {
            PackageManager::Npm => vec!["package.json", "package-lock.json"],
            PackageManager::Yarn => vec!["package.json", "yarn.lock"],
            PackageManager::Pnpm => vec!["package.json", "pnpm-lock.yaml"],
            PackageManager::Bun => vec!["package.json", "bun.lockb"],
        }
    }
}

/// Configuration for building a static site
#[derive(Debug, Clone)]
pub struct StaticSiteConfig {
    /// Source directory containing the project
    pub source_dir: String,
    /// Output directory relative to source (e.g., "dist", "build", "out", ".next/out")
    pub publish_dir: String,
    /// Custom build command (overrides auto-detected)
    pub custom_build_command: Option<String>,
    /// Custom install command (overrides auto-detected)
    pub custom_install_command: Option<String>,
    /// Environment variables for the build
    pub env_vars: Vec<(String, String)>,
    /// Node.js version to use (default: 20)
    pub node_version: Option<String>,
    /// Whether to enable SPA mode (try_files for client-side routing)
    pub spa_mode: bool,
    /// Custom NGINX configuration (optional)
    pub custom_nginx_config: Option<String>,
    /// Build arguments to pass to Docker
    pub build_args: Vec<(String, String)>,
    /// CPU limit for build
    pub cpu_limit: Option<String>,
    /// Memory limit for build
    pub memory_limit: Option<String>,
    /// Port to listen on (default: 3000)
    pub port: u16,
}

impl Default for StaticSiteConfig {
    fn default() -> Self {
        Self {
            source_dir: ".".to_string(),
            publish_dir: "dist".to_string(),
            custom_build_command: None,
            custom_install_command: None,
            env_vars: Vec::new(),
            node_version: None,
            spa_mode: true,
            custom_nginx_config: None,
            build_args: Vec::new(),
            cpu_limit: None,
            memory_limit: None,
            port: 3000,
        }
    }
}

/// Builder for static sites that creates NGINX-based Docker images
pub struct StaticSiteBuilder {
    runtime: Arc<dyn ContainerRuntime>,
}

impl StaticSiteBuilder {
    /// Create a new static site builder
    pub fn new(runtime: Arc<dyn ContainerRuntime>) -> Self {
        Self { runtime }
    }

    /// Detect the package manager from lock files in the source directory
    pub async fn detect_package_manager(source_dir: &Path) -> Result<PackageManager> {
        // Check for lock files in order of preference
        let checks = [
            ("bun.lockb", PackageManager::Bun),
            ("pnpm-lock.yaml", PackageManager::Pnpm),
            ("yarn.lock", PackageManager::Yarn),
            ("package-lock.json", PackageManager::Npm),
        ];

        for (lock_file, pm) in checks {
            let lock_path = source_dir.join(lock_file);
            if fs::try_exists(&lock_path).await.unwrap_or(false) {
                debug!(lock_file = %lock_file, "Detected package manager from lock file");
                return Ok(pm);
            }
        }

        // Default to npm if no lock file found
        info!("No lock file found, defaulting to npm");
        Ok(PackageManager::Npm)
    }

    /// Detect the publish directory based on common framework patterns
    pub async fn detect_publish_dir(source_dir: &Path) -> String {
        // Check for framework-specific output directories
        let checks = [
            // Next.js static export
            (".next/out", "next.config.js"),
            (".next/out", "next.config.mjs"),
            (".next/out", "next.config.ts"),
            // Astro
            ("dist", "astro.config.mjs"),
            ("dist", "astro.config.js"),
            ("dist", "astro.config.ts"),
            // Vite
            ("dist", "vite.config.js"),
            ("dist", "vite.config.ts"),
            ("dist", "vite.config.mjs"),
            // Create React App
            ("build", "react-scripts"),
            // Nuxt static
            (".output/public", "nuxt.config.js"),
            (".output/public", "nuxt.config.ts"),
            // SvelteKit static adapter
            ("build", "svelte.config.js"),
            // Gatsby
            ("public", "gatsby-config.js"),
            ("public", "gatsby-config.ts"),
            // Angular
            ("dist", "angular.json"),
            // Vue CLI
            ("dist", "vue.config.js"),
            // Solid.js
            ("dist", "solid.config.js"),
            ("dist", "solid.config.ts"),
            // Remix (SPA mode)
            ("build/client", "remix.config.js"),
        ];

        for (output_dir, config_file) in checks {
            let config_path = source_dir.join(config_file);
            if fs::try_exists(&config_path).await.unwrap_or(false) {
                debug!(config_file = %config_file, output_dir = %output_dir, "Detected framework");
                return output_dir.to_string();
            }
        }

        // Check package.json for framework hints
        let package_json_path = source_dir.join("package.json");
        if let Ok(content) = fs::read_to_string(&package_json_path).await {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                // Check dependencies for framework indicators
                let deps = json.get("dependencies").and_then(|d| d.as_object());
                let dev_deps = json.get("devDependencies").and_then(|d| d.as_object());

                let has_dep = |name: &str| -> bool {
                    deps.map_or(false, |d| d.contains_key(name))
                        || dev_deps.map_or(false, |d| d.contains_key(name))
                };

                if has_dep("react-scripts") {
                    return "build".to_string();
                }
                if has_dep("gatsby") {
                    return "public".to_string();
                }
                if has_dep("nuxt") {
                    return ".output/public".to_string();
                }
            }
        }

        // Default to dist (most common)
        "dist".to_string()
    }

    /// Generate NGINX configuration for serving static files
    fn generate_nginx_config(config: &StaticSiteConfig) -> String {
        if let Some(ref custom_config) = config.custom_nginx_config {
            return custom_config.clone();
        }

        let try_files = if config.spa_mode {
            "try_files $uri $uri/ /index.html;"
        } else {
            "try_files $uri $uri/ =404;"
        };

        let port = config.port;

        format!(
            r#"server {{
    listen {port};
    listen [::]:{port};
    server_name _;

    root /usr/share/nginx/html;
    index index.html index.htm;

    # Gzip compression
    gzip on;
    gzip_vary on;
    gzip_min_length 1024;
    gzip_proxied expired no-cache no-store private auth;
    gzip_types text/plain text/css text/xml text/javascript application/x-javascript application/javascript application/xml application/json;

    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;

    # Cache static assets
    location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2|ttf|eot)$ {{
        expires 1y;
        add_header Cache-Control "public, immutable";
    }}

    # HTML files - no cache for SPA
    location ~* \.html$ {{
        expires -1;
        add_header Cache-Control "no-store, no-cache, must-revalidate";
    }}

    location / {{
        {try_files}
    }}

    # Health check endpoint
    location /health {{
        access_log off;
        return 200 "OK";
        add_header Content-Type text/plain;
    }}

    # Error pages
    error_page 404 /index.html;
    error_page 500 502 503 504 /50x.html;
    location = /50x.html {{
        root /usr/share/nginx/html;
    }}
}}
"#
        )
    }

    /// Generate a simple Dockerfile for plain HTML sites (no build step)
    fn generate_simple_dockerfile(config: &StaticSiteConfig) -> String {
        // For publish_dir ".", we copy everything from the build context
        // The nginx.conf will be copied after to ensure it's in the right place
        let publish_dir = if config.publish_dir == "." {
            ".".to_string()
        } else {
            config.publish_dir.clone()
        };

        let port = config.port;

        format!(
            r#"# Simple static site (no build step)
FROM nginx:alpine

# Remove default nginx content
RUN rm -rf /usr/share/nginx/html/*

# Copy static files
COPY {publish_dir}/ /usr/share/nginx/html/

# Copy NGINX configuration
COPY nginx.conf /etc/nginx/conf.d/default.conf

# Expose port {port}
EXPOSE {port}

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:{port}/health || exit 1

# Start NGINX
CMD ["nginx", "-g", "daemon off;"]
"#,
            publish_dir = publish_dir,
            port = port,
        )
    }

    /// Generate the Dockerfile for building the static site
    fn generate_dockerfile(
        package_manager: PackageManager,
        config: &StaticSiteConfig,
    ) -> String {
        let node_version = config.node_version.as_deref().unwrap_or("20");
        let base_image = if package_manager == PackageManager::Bun {
            package_manager.base_image().to_string()
        } else {
            format!("node:{}-alpine", node_version)
        };

        let install_cmd = config
            .custom_install_command
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or_else(|| package_manager.install_command());

        let build_cmd = config
            .custom_build_command
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or_else(|| package_manager.build_command());

        // Build environment variables string
        let env_lines: String = config
            .env_vars
            .iter()
            .map(|(k, v)| format!("ENV {}=\"{}\"", k, v.replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join("\n");

        // Copy dependency files for better caching
        let dep_files = package_manager.dependency_files();
        let copy_deps: String = dep_files
            .iter()
            .map(|f| format!("COPY {} ./", f))
            .collect::<Vec<_>>()
            .join("\n");

        // Handle pnpm - needs to be installed in the image
        let pnpm_setup = if package_manager == PackageManager::Pnpm {
            "RUN corepack enable && corepack prepare pnpm@latest --activate\n"
        } else {
            ""
        };

        format!(
            r#"# Build stage
FROM {base_image} AS builder

WORKDIR /app

{pnpm_setup}# Copy dependency files for better layer caching
{copy_deps}

# Install dependencies
RUN {install_cmd}

# Copy source code
COPY . .

# Build environment variables
{env_lines}

# Build the application
RUN {build_cmd}

# Production stage
FROM nginx:alpine AS production

# Copy built assets from builder
COPY --from=builder /app/{publish_dir} /usr/share/nginx/html

# Copy NGINX configuration
COPY nginx.conf /etc/nginx/conf.d/default.conf

# Expose port {port}
EXPOSE {port}

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:{port}/health || exit 1

# Start NGINX
CMD ["nginx", "-g", "daemon off;"]
"#,
            base_image = base_image,
            pnpm_setup = pnpm_setup,
            copy_deps = copy_deps,
            install_cmd = install_cmd,
            env_lines = env_lines,
            build_cmd = build_cmd,
            publish_dir = config.publish_dir,
            port = config.port,
        )
    }

    /// Build a static site and return the image tag
    ///
    /// # Arguments
    /// * `config` - Static site configuration
    /// * `image_tag` - Tag for the built image (e.g., "rivetr-myapp:abc123")
    ///
    /// # Returns
    /// The image tag on success
    pub async fn build(&self, config: &StaticSiteConfig, image_tag: &str) -> Result<String> {
        let source_path = Path::new(&config.source_dir);

        // Check if this is a plain HTML site (no package.json)
        let package_json_path = source_path.join("package.json");
        let has_package_json = fs::try_exists(&package_json_path).await.unwrap_or(false);

        // Generate Dockerfile content based on whether it's a plain HTML site or has a build step
        let dockerfile_content = if has_package_json {
            // Detect package manager and generate build Dockerfile
            let package_manager = Self::detect_package_manager(source_path).await?;
            info!(
                package_manager = ?package_manager,
                "Building static site with package manager"
            );
            Self::generate_dockerfile(package_manager, config)
        } else {
            // Plain HTML site - no build step needed
            info!("Building plain HTML static site (no package.json, no build step)");
            Self::generate_simple_dockerfile(config)
        };
        debug!("Generated Dockerfile:\n{}", dockerfile_content);

        // Generate NGINX config
        let nginx_config = Self::generate_nginx_config(config);
        debug!("Generated NGINX config:\n{}", nginx_config);

        // Write Dockerfile to source directory
        let dockerfile_path = source_path.join("Dockerfile.rivetr-static");
        fs::write(&dockerfile_path, &dockerfile_content)
            .await
            .context("Failed to write Dockerfile")?;

        // Write NGINX config to source directory
        let nginx_config_path = source_path.join("nginx.conf");
        fs::write(&nginx_config_path, &nginx_config)
            .await
            .context("Failed to write nginx.conf")?;

        // Build the Docker image
        let build_ctx = BuildContext {
            path: config.source_dir.clone(),
            dockerfile: "Dockerfile.rivetr-static".to_string(),
            tag: image_tag.to_string(),
            build_args: config.build_args.clone(),
            build_target: None,
            custom_options: None,
            cpu_limit: config.cpu_limit.clone(),
            memory_limit: config.memory_limit.clone(),
        };

        let result = self
            .runtime
            .build(&build_ctx)
            .await
            .context("Failed to build static site image");

        // Clean up generated files
        let _ = fs::remove_file(&dockerfile_path).await;
        // Keep nginx.conf as it might be useful for debugging
        // let _ = fs::remove_file(&nginx_config_path).await;

        result
    }

    /// Build a static site with auto-detected settings
    ///
    /// This is a convenience method that auto-detects the publish directory
    /// and uses sensible defaults.
    pub async fn build_auto(
        &self,
        source_dir: &str,
        image_tag: &str,
        env_vars: Vec<(String, String)>,
    ) -> Result<String> {
        let source_path = Path::new(source_dir);
        let publish_dir = Self::detect_publish_dir(source_path).await;

        info!(
            source_dir = %source_dir,
            publish_dir = %publish_dir,
            "Auto-detected static site configuration"
        );

        let config = StaticSiteConfig {
            source_dir: source_dir.to_string(),
            publish_dir,
            env_vars,
            ..Default::default()
        };

        self.build(&config, image_tag).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_manager_commands() {
        assert_eq!(PackageManager::Npm.install_command(), "npm ci");
        assert_eq!(PackageManager::Yarn.install_command(), "yarn install --frozen-lockfile");
        assert_eq!(PackageManager::Pnpm.install_command(), "pnpm install --frozen-lockfile");
        assert_eq!(PackageManager::Bun.install_command(), "bun install --frozen-lockfile");

        assert_eq!(PackageManager::Npm.build_command(), "npm run build");
        assert_eq!(PackageManager::Yarn.build_command(), "yarn build");
        assert_eq!(PackageManager::Pnpm.build_command(), "pnpm build");
        assert_eq!(PackageManager::Bun.build_command(), "bun run build");
    }

    #[test]
    fn test_generate_dockerfile() {
        let config = StaticSiteConfig {
            source_dir: "/tmp/test".to_string(),
            publish_dir: "dist".to_string(),
            env_vars: vec![("NODE_ENV".to_string(), "production".to_string())],
            ..Default::default()
        };

        let dockerfile = StaticSiteBuilder::generate_dockerfile(PackageManager::Npm, &config);
        assert!(dockerfile.contains("FROM node:20-alpine AS builder"));
        assert!(dockerfile.contains("npm ci"));
        assert!(dockerfile.contains("npm run build"));
        assert!(dockerfile.contains("FROM nginx:alpine"));
        assert!(dockerfile.contains("COPY --from=builder /app/dist"));
        assert!(dockerfile.contains("ENV NODE_ENV=\"production\""));
    }

    #[test]
    fn test_generate_dockerfile_with_pnpm() {
        let config = StaticSiteConfig::default();

        let dockerfile = StaticSiteBuilder::generate_dockerfile(PackageManager::Pnpm, &config);
        assert!(dockerfile.contains("corepack enable"));
        assert!(dockerfile.contains("pnpm install --frozen-lockfile"));
        assert!(dockerfile.contains("pnpm build"));
    }

    #[test]
    fn test_generate_dockerfile_with_bun() {
        let config = StaticSiteConfig::default();

        let dockerfile = StaticSiteBuilder::generate_dockerfile(PackageManager::Bun, &config);
        assert!(dockerfile.contains("FROM oven/bun:1-alpine"));
        assert!(dockerfile.contains("bun install --frozen-lockfile"));
        assert!(dockerfile.contains("bun run build"));
    }

    #[test]
    fn test_generate_nginx_config_spa_mode() {
        let config = StaticSiteConfig {
            spa_mode: true,
            ..Default::default()
        };

        let nginx_config = StaticSiteBuilder::generate_nginx_config(&config);
        assert!(nginx_config.contains("try_files $uri $uri/ /index.html;"));
        assert!(nginx_config.contains("gzip on;"));
        assert!(nginx_config.contains("location /health"));
    }

    #[test]
    fn test_generate_nginx_config_non_spa_mode() {
        let config = StaticSiteConfig {
            spa_mode: false,
            ..Default::default()
        };

        let nginx_config = StaticSiteBuilder::generate_nginx_config(&config);
        assert!(nginx_config.contains("try_files $uri $uri/ =404;"));
    }

    #[test]
    fn test_custom_commands() {
        let config = StaticSiteConfig {
            custom_install_command: Some("npm install --legacy-peer-deps".to_string()),
            custom_build_command: Some("npm run build:prod".to_string()),
            ..Default::default()
        };

        let dockerfile = StaticSiteBuilder::generate_dockerfile(PackageManager::Npm, &config);
        assert!(dockerfile.contains("npm install --legacy-peer-deps"));
        assert!(dockerfile.contains("npm run build:prod"));
    }

    #[test]
    fn test_custom_nginx_config() {
        let custom_config = "server { listen 80; }".to_string();
        let config = StaticSiteConfig {
            custom_nginx_config: Some(custom_config.clone()),
            ..Default::default()
        };

        let nginx_config = StaticSiteBuilder::generate_nginx_config(&config);
        assert_eq!(nginx_config, custom_config);
    }

    #[test]
    fn test_custom_node_version() {
        let config = StaticSiteConfig {
            node_version: Some("18".to_string()),
            ..Default::default()
        };

        let dockerfile = StaticSiteBuilder::generate_dockerfile(PackageManager::Npm, &config);
        assert!(dockerfile.contains("FROM node:18-alpine AS builder"));
    }
}
