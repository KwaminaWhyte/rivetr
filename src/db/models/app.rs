//! Application models and DTOs.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::common::{parse_domains, Domain, Environment, PortMapping};
use crate::engine::nixpacks::NixpacksConfig;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct App {
    pub id: String,
    pub name: String,
    pub git_url: String,
    pub branch: String,
    pub dockerfile: String,
    pub domain: Option<String>,
    pub port: i32,
    pub healthcheck: Option<String>,
    pub memory_limit: Option<String>,
    pub cpu_limit: Option<String>,
    pub ssh_key_id: Option<String>,
    pub environment: String,
    pub project_id: Option<String>,
    /// Environment ID for project environment scoping
    pub environment_id: Option<String>,
    /// Team ID for multi-tenant scoping (nullable for legacy apps)
    pub team_id: Option<String>,
    // Advanced build options
    pub dockerfile_path: Option<String>,
    pub base_directory: Option<String>,
    pub build_target: Option<String>,
    pub watch_paths: Option<String>,
    pub custom_docker_options: Option<String>,
    // Network configuration (JSON stored as TEXT)
    /// JSON array of PortMapping objects
    pub port_mappings: Option<String>,
    /// JSON array of network alias strings
    pub network_aliases: Option<String>,
    /// JSON array of "hostname:ip" entries for extra hosts
    pub extra_hosts: Option<String>,
    // HTTP Basic Auth
    pub basic_auth_enabled: i32,
    #[serde(skip_serializing)]
    pub basic_auth_username: Option<String>,
    #[serde(skip_serializing)]
    pub basic_auth_password_hash: Option<String>,
    // Deployment commands (JSON stored as TEXT)
    /// JSON array of commands to run before container starts (after build)
    pub pre_deploy_commands: Option<String>,
    /// JSON array of commands to run after container is healthy
    pub post_deploy_commands: Option<String>,
    // Domain management (JSON stored as TEXT)
    /// JSON array of Domain objects for multiple domain support
    pub domains: Option<String>,
    /// Auto-generated subdomain (e.g., app-name.rivetr.example.com)
    pub auto_subdomain: Option<String>,
    // Docker Registry support
    /// Docker image name (e.g., "nginx", "ghcr.io/user/app") - when set, skip build and pull from registry
    pub docker_image: Option<String>,
    /// Docker image tag (default: "latest")
    pub docker_image_tag: Option<String>,
    /// Custom registry URL (null = Docker Hub)
    pub registry_url: Option<String>,
    /// Registry authentication username
    pub registry_username: Option<String>,
    /// Registry authentication password (encrypted)
    #[serde(skip_serializing)]
    pub registry_password: Option<String>,
    /// Container labels (JSON object stored as TEXT)
    pub container_labels: Option<String>,
    /// Build type: "dockerfile" (default), "nixpacks", or "static"
    pub build_type: Option<String>,
    /// Nixpacks-specific configuration (JSON)
    pub nixpacks_config: Option<String>,
    /// Publish directory for static site builds (e.g., "dist", "build", "out")
    pub publish_directory: Option<String>,
    /// Enable PR preview deployments for this app
    #[serde(default)]
    pub preview_enabled: i32,
    /// GitHub App installation ID for GitHub-based deployments
    pub github_app_installation_id: Option<String>,
    /// Deployment source: "git", "upload", or "registry"
    pub deployment_source: Option<String>,
    /// Enable automatic rollback on health check failure
    #[serde(default)]
    pub auto_rollback_enabled: i32,
    /// Enable pushing images to registry for rollback support
    #[serde(default)]
    pub registry_push_enabled: i32,
    /// Maximum number of deployment versions to keep for rollback (default: 5)
    #[serde(default)]
    pub max_rollback_versions: i32,
    /// Require approval before deployments are executed
    #[serde(default)]
    pub require_approval: i32,
    /// Enable maintenance mode (serves maintenance message instead of app)
    #[serde(default)]
    pub maintenance_mode: i32,
    /// Custom maintenance message shown when maintenance mode is active
    pub maintenance_message: Option<String>,
    /// Number of container replicas to run (for load balancing)
    #[serde(default = "default_replica_count")]
    pub replica_count: i64,
    /// Preferred server ID for remote deployment (nullable)
    pub server_id: Option<String>,
    /// Build server ID for offloading Docker builds to a remote machine (nullable)
    pub build_server_id: Option<String>,
    /// Number of previous successful deployments to keep for rollback (default: 10)
    #[serde(default = "default_rollback_retention_count")]
    pub rollback_retention_count: i64,
    /// Container restart policy: "always", "unless-stopped", "on-failure", or "never"
    #[serde(default = "default_restart_policy")]
    pub restart_policy: String,
    // Custom Docker run options
    /// Run container in privileged mode
    #[serde(default)]
    pub privileged: i64,
    /// JSON array of capabilities to add (e.g. ["NET_ADMIN", "SYS_PTRACE"])
    pub cap_add: Option<String>,
    /// JSON array of device mappings (e.g. ["/dev/snd:/dev/snd"])
    pub devices: Option<String>,
    /// Shared memory size (e.g. "128m", "1g")
    pub shm_size: Option<String>,
    /// Run tini as PID 1 (init process)
    #[serde(default)]
    pub init_process: i64,
    /// JSON array of build-time secrets: [{key: String, value: String}]
    /// Injected during `docker build` via BuildKit `--secret` — not baked into image layers.
    pub build_secrets: Option<String>,
    /// Target Docker build platform(s), e.g. "linux/amd64" or "linux/arm64".
    /// NULL means use the Docker daemon default (linux/amd64).
    pub build_platforms: Option<String>,
    /// Timestamp of the last crash notification sent for this app (for rate-limiting)
    pub last_crash_notified_at: Option<String>,
    /// JSON array of capabilities to drop (e.g. ["MKNOD"])
    pub docker_cap_drop: Option<String>,
    /// GPU access: "all" or "device=0,1"
    pub docker_gpus: Option<String>,
    /// JSON array of ulimit strings (e.g. ["nofile=1024:1024"])
    pub docker_ulimits: Option<String>,
    /// JSON array of security options (e.g. ["seccomp=unconfined"])
    pub docker_security_opt: Option<String>,
    // Git clone options
    /// Pass --recurse-submodules to git clone
    #[serde(default)]
    pub git_submodules: i64,
    /// Run `git lfs pull` after clone (requires git-lfs installed)
    #[serde(default)]
    pub git_lfs: i64,
    /// Use --depth 1 for fast shallow clones (default true); set to 0 for full clone
    #[serde(default = "default_shallow_clone")]
    pub shallow_clone: i64,
    // Build options
    /// Pass --no-cache to docker build / nixpacks build
    #[serde(default)]
    pub disable_build_cache: i64,
    /// Inject SOURCE_COMMIT build arg with the current git SHA
    #[serde(default)]
    pub include_source_commit: i64,
    // Container naming
    /// Override the container name used for the app's container (nullable)
    pub custom_container_name: Option<String>,
    /// Treat this app as a static site (serve files without a runtime container)
    #[serde(default)]
    pub is_static_site: i64,
    /// URL prefix to strip from incoming requests before forwarding to the container
    pub strip_prefix: Option<String>,
    /// Inline Dockerfile content — if set, skip git clone and build from this content directly
    pub inline_dockerfile: Option<String>,
    /// Docker destination (named network) for this app (nullable)
    pub destination_id: Option<String>,
    /// Custom container labels (JSON array: [{key, value}]) applied at deployment time
    pub custom_labels: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Response DTO for App that excludes sensitive fields (password hash)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppResponse {
    pub id: String,
    pub name: String,
    pub git_url: String,
    pub branch: String,
    pub dockerfile: String,
    pub domain: Option<String>,
    pub port: i32,
    pub healthcheck: Option<String>,
    pub memory_limit: Option<String>,
    pub cpu_limit: Option<String>,
    pub ssh_key_id: Option<String>,
    pub environment: String,
    pub project_id: Option<String>,
    /// Environment ID for project environment scoping
    pub environment_id: Option<String>,
    /// Team ID for multi-tenant scoping
    pub team_id: Option<String>,
    // Advanced build options
    pub dockerfile_path: Option<String>,
    pub base_directory: Option<String>,
    pub build_target: Option<String>,
    pub watch_paths: Option<String>,
    pub custom_docker_options: Option<String>,
    // Network configuration
    pub port_mappings: Option<String>,
    pub network_aliases: Option<String>,
    pub extra_hosts: Option<String>,
    // HTTP Basic Auth (password hash excluded)
    pub basic_auth_enabled: bool,
    pub basic_auth_username: Option<String>,
    // Deployment commands
    pub pre_deploy_commands: Option<String>,
    pub post_deploy_commands: Option<String>,
    // Domain management
    pub domains: Option<String>,
    pub auto_subdomain: Option<String>,
    // Docker Registry support (password excluded)
    pub docker_image: Option<String>,
    pub docker_image_tag: Option<String>,
    pub registry_url: Option<String>,
    pub registry_username: Option<String>,
    // Container labels
    pub container_labels: Option<String>,
    // Build type and Nixpacks support
    pub build_type: Option<String>,
    pub nixpacks_config: Option<String>,
    pub publish_directory: Option<String>,
    // Preview deployments
    pub preview_enabled: bool,
    /// GitHub App installation ID for GitHub-based deployments
    pub github_app_installation_id: Option<String>,
    /// Deployment source: "git", "upload", or "registry"
    pub deployment_source: Option<String>,
    /// Enable automatic rollback on health check failure
    pub auto_rollback_enabled: bool,
    /// Enable pushing images to registry for rollback support
    pub registry_push_enabled: bool,
    /// Maximum number of deployment versions to keep for rollback
    pub max_rollback_versions: i32,
    /// Require approval before deployments are executed
    pub require_approval: bool,
    /// Enable maintenance mode
    pub maintenance_mode: bool,
    /// Custom maintenance message
    pub maintenance_message: Option<String>,
    /// Number of container replicas to run (for load balancing)
    pub replica_count: i64,
    /// Preferred server ID for remote deployment (nullable)
    pub server_id: Option<String>,
    /// Build server ID for offloading Docker builds to a remote machine (nullable)
    pub build_server_id: Option<String>,
    /// Number of previous successful deployments to keep for rollback (default: 10)
    pub rollback_retention_count: i64,
    /// Container restart policy: "always", "unless-stopped", "on-failure", or "never"
    pub restart_policy: String,
    // Custom Docker run options
    /// Run container in privileged mode
    pub privileged: bool,
    /// JSON array of capabilities to add (e.g. ["NET_ADMIN", "SYS_PTRACE"])
    pub cap_add: Option<String>,
    /// JSON array of device mappings (e.g. ["/dev/snd:/dev/snd"])
    pub devices: Option<String>,
    /// Shared memory size (e.g. "128m", "1g")
    pub shm_size: Option<String>,
    /// Run tini as PID 1 (init process)
    pub init_process: bool,
    /// JSON array of build-time secrets (values are masked in responses)
    pub build_secrets: Option<String>,
    /// Target Docker build platform(s), e.g. "linux/amd64" or "linux/arm64".
    pub build_platforms: Option<String>,
    /// JSON array of capabilities to drop (e.g. ["MKNOD"])
    pub docker_cap_drop: Option<String>,
    /// GPU access: "all" or "device=0,1"
    pub docker_gpus: Option<String>,
    /// JSON array of ulimit strings (e.g. ["nofile=1024:1024"])
    pub docker_ulimits: Option<String>,
    /// JSON array of security options (e.g. ["seccomp=unconfined"])
    pub docker_security_opt: Option<String>,
    // Git clone options
    /// Pass --recurse-submodules to git clone
    pub git_submodules: bool,
    /// Run `git lfs pull` after clone
    pub git_lfs: bool,
    /// Use --depth 1 for fast shallow clones (default true)
    pub shallow_clone: bool,
    // Build options
    /// Pass --no-cache to docker build
    pub disable_build_cache: bool,
    /// Inject SOURCE_COMMIT build arg with the current git SHA
    pub include_source_commit: bool,
    // Container naming
    /// Override the container name (nullable)
    pub custom_container_name: Option<String>,
    /// Treat this app as a static site (serve files without a runtime container)
    pub is_static_site: bool,
    /// URL prefix to strip from incoming requests before forwarding to the container
    pub strip_prefix: Option<String>,
    /// Inline Dockerfile content (alternative to git-based build)
    pub inline_dockerfile: Option<String>,
    /// Docker destination (named network) for this app (nullable)
    pub destination_id: Option<String>,
    /// Custom container labels (JSON array: [{key, value}]) applied at deployment time
    pub custom_labels: Option<String>,
    /// Stable internal Docker network hostname for this app (derived).
    /// Other containers on the shared `rivetr` network can reach this app at this
    /// hostname.  Always equal to `custom_container_name` or `rivetr-<app-name>`.
    pub internal_hostname: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<App> for AppResponse {
    fn from(app: App) -> Self {
        let internal_hostname = match app.custom_container_name.as_deref() {
            Some(name) if !name.is_empty() => name.to_string(),
            _ => format!("rivetr-{}", app.name),
        };
        Self {
            id: app.id,
            name: app.name,
            git_url: app.git_url,
            branch: app.branch,
            dockerfile: app.dockerfile,
            domain: app.domain,
            port: app.port,
            healthcheck: app.healthcheck,
            memory_limit: app.memory_limit,
            cpu_limit: app.cpu_limit,
            ssh_key_id: app.ssh_key_id,
            environment: app.environment,
            project_id: app.project_id,
            environment_id: app.environment_id,
            team_id: app.team_id,
            dockerfile_path: app.dockerfile_path,
            base_directory: app.base_directory,
            build_target: app.build_target,
            watch_paths: app.watch_paths,
            custom_docker_options: app.custom_docker_options,
            port_mappings: app.port_mappings,
            network_aliases: app.network_aliases,
            extra_hosts: app.extra_hosts,
            basic_auth_enabled: app.basic_auth_enabled != 0,
            basic_auth_username: app.basic_auth_username,
            pre_deploy_commands: app.pre_deploy_commands,
            post_deploy_commands: app.post_deploy_commands,
            domains: app.domains,
            auto_subdomain: app.auto_subdomain,
            docker_image: app.docker_image,
            docker_image_tag: app.docker_image_tag,
            registry_url: app.registry_url,
            registry_username: app.registry_username,
            container_labels: app.container_labels,
            build_type: app.build_type,
            nixpacks_config: app.nixpacks_config,
            publish_directory: app.publish_directory,
            preview_enabled: app.preview_enabled != 0,
            github_app_installation_id: app.github_app_installation_id,
            deployment_source: app.deployment_source,
            auto_rollback_enabled: app.auto_rollback_enabled != 0,
            registry_push_enabled: app.registry_push_enabled != 0,
            max_rollback_versions: app.max_rollback_versions,
            require_approval: app.require_approval != 0,
            maintenance_mode: app.maintenance_mode != 0,
            maintenance_message: app.maintenance_message,
            replica_count: app.replica_count,
            server_id: app.server_id,
            build_server_id: app.build_server_id,
            rollback_retention_count: app.rollback_retention_count,
            restart_policy: app.restart_policy,
            privileged: app.privileged != 0,
            cap_add: app.cap_add,
            devices: app.devices,
            shm_size: app.shm_size,
            init_process: app.init_process != 0,
            build_secrets: app.build_secrets,
            build_platforms: app.build_platforms,
            docker_cap_drop: app.docker_cap_drop,
            docker_gpus: app.docker_gpus,
            docker_ulimits: app.docker_ulimits,
            docker_security_opt: app.docker_security_opt,
            git_submodules: app.git_submodules != 0,
            git_lfs: app.git_lfs != 0,
            shallow_clone: app.shallow_clone != 0,
            disable_build_cache: app.disable_build_cache != 0,
            include_source_commit: app.include_source_commit != 0,
            custom_container_name: app.custom_container_name,
            is_static_site: app.is_static_site != 0,
            strip_prefix: app.strip_prefix,
            inline_dockerfile: app.inline_dockerfile,
            destination_id: app.destination_id,
            custom_labels: app.custom_labels,
            internal_hostname,
            created_at: app.created_at,
            updated_at: app.updated_at,
        }
    }
}

impl App {
    /// Parse domains from JSON string
    pub fn get_domains(&self) -> Vec<Domain> {
        parse_domains(self.domains.as_deref())
    }

    /// Get the primary domain (from domains list or legacy domain field)
    pub fn get_primary_domain(&self) -> Option<String> {
        // First check the domains array for a primary domain
        let domains = self.get_domains();
        if let Some(primary) = domains.iter().find(|d| d.primary) {
            return Some(primary.domain.clone());
        }
        // If no primary in domains array but there are domains, use the first one
        if let Some(first) = domains.first() {
            return Some(first.domain.clone());
        }
        // Fall back to legacy domain field
        self.domain.clone()
    }

    /// Get all domain names (including legacy domain and auto_subdomain).
    /// Delegates to `get_all_domains_with_redirects()`, discarding redirect targets.
    pub fn get_all_domain_names(&self) -> Vec<String> {
        self.get_all_domains_with_redirects()
            .into_iter()
            .map(|(d, _)| d)
            .collect()
    }

    /// Get all domain names with optional www redirect targets.
    /// Returns `Vec<(domain, Option<redirect_target>)>` where `redirect_target` is `Some` if the
    /// domain should redirect to another domain (for `www_redirect_mode`).
    pub fn get_all_domains_with_redirects(&self) -> Vec<(String, Option<String>)> {
        let mut result: Vec<(String, Option<String>)> = Vec::new();

        let contains = |entries: &Vec<(String, Option<String>)>, d: &str| {
            entries.iter().any(|(name, _)| name == d)
        };

        for d in self.get_domains() {
            let mode = d.effective_www_redirect_mode();

            // Always add the configured domain itself first
            if !contains(&result, &d.domain) {
                result.push((d.domain.clone(), None));
            }

            match mode {
                "to_www" => {
                    let www = d.www_domain();
                    let non_www = d.non_www_domain();

                    // Ensure the www variant is present as canonical (no redirect)
                    if !contains(&result, &www) {
                        result.push((www.clone(), None));
                    } else if let Some(pos) = result.iter().position(|(n, _)| n == &www) {
                        result[pos].1 = None;
                    }

                    // non-www → redirect to www
                    if non_www != d.domain {
                        if let Some(pos) = result.iter().position(|(n, _)| n == &non_www) {
                            result[pos].1 = Some(www);
                        } else {
                            result.push((non_www, Some(www)));
                        }
                    } else {
                        // d.domain IS the non-www; redirect it to www
                        if let Some(pos) = result.iter().position(|(n, _)| n == &d.domain) {
                            result[pos].1 = Some(www);
                        }
                    }
                }
                "to_non_www" => {
                    let www = d.www_domain();
                    let non_www = d.non_www_domain();

                    // Ensure the non-www variant is present as canonical (no redirect)
                    if !contains(&result, &non_www) {
                        result.push((non_www.clone(), None));
                    } else if let Some(pos) = result.iter().position(|(n, _)| n == &non_www) {
                        result[pos].1 = None;
                    }

                    // www → redirect to non-www
                    if www != d.domain {
                        if let Some(pos) = result.iter().position(|(n, _)| n == &www) {
                            result[pos].1 = Some(non_www);
                        } else {
                            result.push((www, Some(non_www)));
                        }
                    } else {
                        // d.domain IS the www; redirect it to non-www
                        if let Some(pos) = result.iter().position(|(n, _)| n == &d.domain) {
                            result[pos].1 = Some(non_www);
                        }
                    }
                }
                _ => {
                    // "both" — register both variants without redirect (legacy redirect_www behaviour)
                    if d.redirect_www {
                        let variant = if d.domain.starts_with("www.") {
                            d.non_www_domain()
                        } else {
                            d.www_domain()
                        };
                        if !contains(&result, &variant) {
                            result.push((variant, None));
                        }
                    }
                }
            }
        }

        // Add legacy domain field
        if let Some(ref domain) = self.domain {
            if !domain.is_empty() && !contains(&result, domain) {
                result.push((domain.clone(), None));
            }
        }

        // Add auto_subdomain
        if let Some(ref subdomain) = self.auto_subdomain {
            if !subdomain.is_empty() && !contains(&result, subdomain) {
                result.push((subdomain.clone(), None));
            }
        }

        result
    }

    /// Parse port mappings from JSON string
    pub fn get_port_mappings(&self) -> Vec<PortMapping> {
        self.port_mappings
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }

    /// Parse network aliases from JSON string.
    ///
    /// Always includes a stable canonical alias `rivetr-<app-name>` so that other
    /// containers on the shared `rivetr` Docker network can resolve this app by a
    /// hostname that does NOT change across redeploys / restarts.  Without this,
    /// containers got `-restart-<hash>` suffixed names whose alias would shift on
    /// every zero-downtime restart, breaking inter-service connections.
    pub fn get_network_aliases(&self) -> Vec<String> {
        let mut aliases: Vec<String> = self
            .network_aliases
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let canonical = self.internal_hostname();
        if !aliases.iter().any(|a| a == &canonical) {
            aliases.push(canonical);
        }

        aliases
    }

    /// Canonical internal hostname for service-to-service discovery on the
    /// rivetr Docker network.  Stable across deploys, restarts, and rollbacks.
    /// Surfaced in the API response so the frontend can render it in the
    /// "Network" tab connection example.
    pub fn internal_hostname(&self) -> String {
        // If the user set a custom container name, use that as the hostname
        // (matches the actual container name).  Otherwise default to
        // `rivetr-<app-name>`.
        match self.custom_container_name.as_ref() {
            Some(name) if !name.is_empty() => name.clone(),
            _ => format!("rivetr-{}", self.name),
        }
    }

    /// Parse extra hosts from JSON string
    pub fn get_extra_hosts(&self) -> Vec<String> {
        self.extra_hosts
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }

    /// Parse pre_deploy_commands JSON into Vec<String>
    pub fn get_pre_deploy_commands(&self) -> Vec<String> {
        self.pre_deploy_commands
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }

    /// Parse post_deploy_commands JSON into Vec<String>
    pub fn get_post_deploy_commands(&self) -> Vec<String> {
        self.post_deploy_commands
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }

    /// Check if this app uses a Docker registry image instead of building from git
    pub fn uses_registry_image(&self) -> bool {
        self.docker_image
            .as_ref()
            .map(|s| !s.is_empty())
            .unwrap_or(false)
    }

    /// Get the full image reference including registry URL and tag
    /// Format: [registry/]image[:tag]
    pub fn get_full_image_reference(&self) -> Option<String> {
        let image = self.docker_image.as_ref()?;
        if image.is_empty() {
            return None;
        }

        let tag = self
            .docker_image_tag
            .as_ref()
            .filter(|t| !t.is_empty())
            .map(|t| t.as_str())
            .unwrap_or("latest");

        // If there's a custom registry URL, prepend it
        let full_image = if let Some(ref registry) = self.registry_url {
            if !registry.is_empty() {
                // Remove trailing slash from registry and leading slash from image
                let registry = registry.trim_end_matches('/');
                let image = image.trim_start_matches('/');
                format!("{}/{}", registry, image)
            } else {
                image.clone()
            }
        } else {
            image.clone()
        };

        Some(format!("{}:{}", full_image, tag))
    }

    /// Parse container_labels JSON into HashMap<String, String>
    pub fn get_container_labels(&self) -> std::collections::HashMap<String, String> {
        self.container_labels
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }

    /// Get the build type, defaulting to "dockerfile" if empty or not set
    pub fn get_build_type(&self) -> &str {
        self.build_type
            .as_ref()
            .filter(|s| !s.is_empty())
            .map(|s| s.as_str())
            .unwrap_or("dockerfile")
    }

    /// Check if this app uses Nixpacks for building
    pub fn uses_nixpacks(&self) -> bool {
        self.get_build_type() == "nixpacks"
    }

    /// Parse nixpacks_config JSON into NixpacksConfig
    pub fn get_nixpacks_config(&self) -> Option<NixpacksConfig> {
        self.nixpacks_config
            .as_ref()
            .and_then(|s| NixpacksConfig::from_json(s).ok())
    }

    /// Check if automatic rollback is enabled for this app
    pub fn is_auto_rollback_enabled(&self) -> bool {
        self.auto_rollback_enabled != 0
    }

    /// Check if deployment approval is required for this app
    pub fn is_require_approval(&self) -> bool {
        self.require_approval != 0
    }

    /// Check if maintenance mode is enabled for this app
    pub fn is_maintenance_mode(&self) -> bool {
        self.maintenance_mode != 0
    }

    /// Check if registry push is enabled for this app
    pub fn is_registry_push_enabled(&self) -> bool {
        self.registry_push_enabled != 0
    }

    /// Check if this app can push to registry (has registry configured and push enabled)
    pub fn can_push_to_registry(&self) -> bool {
        self.is_registry_push_enabled()
            && self
                .registry_url
                .as_ref()
                .map(|s| !s.is_empty())
                .unwrap_or(false)
    }

    /// Get the registry URL for pushing images (for rollback support)
    /// Uses the same registry configured for Docker image pulls
    pub fn get_rollback_registry_url(&self) -> Option<&str> {
        self.registry_url.as_deref().filter(|s| !s.is_empty())
    }

    /// Parse watch_paths JSON into Vec<String>
    pub fn get_watch_paths(&self) -> Vec<String> {
        self.watch_paths
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }

    /// Parse build_secrets JSON into Vec<BuildSecret>
    pub fn get_build_secrets(&self) -> Vec<BuildSecret> {
        self.build_secrets
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }
}

/// A single build-time secret injected via BuildKit `--secret`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildSecret {
    pub key: String,
    pub value: String,
}

// DTOs for API

#[derive(Debug, Deserialize)]
pub struct CreateAppRequest {
    pub name: String,
    /// Git URL for source-based deployments (required if docker_image is not set)
    #[serde(default)]
    pub git_url: String,
    #[serde(default = "default_branch")]
    pub branch: String,
    #[serde(default = "default_dockerfile")]
    pub dockerfile: String,
    pub domain: Option<String>,
    #[serde(default = "default_port")]
    pub port: i32,
    pub healthcheck: Option<String>,
    pub memory_limit: Option<String>,
    pub cpu_limit: Option<String>,
    pub ssh_key_id: Option<String>,
    #[serde(default)]
    pub environment: Environment,
    pub project_id: Option<String>,
    /// Team ID for multi-tenant scoping
    pub team_id: Option<String>,
    // Advanced build options
    pub dockerfile_path: Option<String>,
    pub base_directory: Option<String>,
    pub build_target: Option<String>,
    pub watch_paths: Option<String>,
    pub custom_docker_options: Option<String>,
    // Network configuration
    pub port_mappings: Option<Vec<PortMapping>>,
    pub network_aliases: Option<Vec<String>>,
    pub extra_hosts: Option<Vec<String>>,
    // Deployment commands (JSON arrays)
    pub pre_deploy_commands: Option<Vec<String>>,
    pub post_deploy_commands: Option<Vec<String>>,
    // Domain management
    pub domains: Option<Vec<Domain>>,
    // Docker Registry support (alternative to git-based deployments)
    /// Docker image name (e.g., "nginx", "ghcr.io/user/app")
    pub docker_image: Option<String>,
    /// Docker image tag (default: "latest")
    #[serde(default = "default_image_tag")]
    pub docker_image_tag: Option<String>,
    /// Custom registry URL (null = Docker Hub)
    pub registry_url: Option<String>,
    /// Registry authentication username
    pub registry_username: Option<String>,
    /// Registry authentication password
    pub registry_password: Option<String>,
    /// Container labels (key-value pairs)
    pub container_labels: Option<std::collections::HashMap<String, String>>,
    // Build type and Nixpacks support
    /// Build type: "dockerfile" (default), "nixpacks", or "static"
    #[serde(default = "default_build_type")]
    pub build_type: String,
    /// Nixpacks-specific configuration (JSON object or JSON string)
    pub nixpacks_config: Option<serde_json::Value>,
    /// Publish directory for static site builds (e.g., "dist", "build", "out")
    pub publish_directory: Option<String>,
    /// Enable PR preview deployments
    #[serde(default)]
    pub preview_enabled: bool,
    /// GitHub App installation ID for GitHub-based deployments
    pub github_app_installation_id: Option<String>,
    /// Git provider ID (OAuth) for authenticated HTTPS cloning
    pub git_provider_id: Option<String>,
    /// Container restart policy: "always", "unless-stopped", "on-failure", or "never"
    #[serde(default = "default_restart_policy")]
    pub restart_policy: String,
    // Custom Docker run options
    /// Run container in privileged mode
    #[serde(default)]
    pub privileged: bool,
    /// Capabilities to add (e.g. ["NET_ADMIN", "SYS_PTRACE"])
    pub cap_add: Option<Vec<String>>,
    /// Device mappings (e.g. ["/dev/snd:/dev/snd"])
    pub devices: Option<Vec<String>>,
    /// Shared memory size (e.g. "128m", "1g")
    pub shm_size: Option<String>,
    /// Run tini as PID 1 (init process)
    #[serde(default)]
    pub init_process: bool,
    /// Number of container replicas to run (for load balancing, default: 1)
    #[serde(default = "default_replica_count")]
    pub replica_count: i64,
}

fn default_build_type() -> String {
    "dockerfile".to_string()
}

fn default_shallow_clone() -> i64 {
    1
}

fn default_restart_policy() -> String {
    "unless-stopped".to_string()
}

fn default_rollback_retention_count() -> i64 {
    10
}

fn default_image_tag() -> Option<String> {
    Some("latest".to_string())
}

fn default_branch() -> String {
    "main".to_string()
}

fn default_dockerfile() -> String {
    "./Dockerfile".to_string()
}

fn default_port() -> i32 {
    3000
}

#[derive(Debug, Deserialize)]
pub struct UpdateAppRequest {
    pub name: Option<String>,
    pub git_url: Option<String>,
    pub branch: Option<String>,
    pub dockerfile: Option<String>,
    pub domain: Option<String>,
    pub port: Option<i32>,
    pub healthcheck: Option<String>,
    pub memory_limit: Option<String>,
    pub cpu_limit: Option<String>,
    pub ssh_key_id: Option<String>,
    pub environment: Option<Environment>,
    pub project_id: Option<String>,
    // Advanced build options
    pub dockerfile_path: Option<String>,
    pub base_directory: Option<String>,
    pub build_target: Option<String>,
    pub watch_paths: Option<String>,
    pub custom_docker_options: Option<String>,
    // Network configuration
    pub port_mappings: Option<Vec<PortMapping>>,
    pub network_aliases: Option<Vec<String>>,
    pub extra_hosts: Option<Vec<String>>,
    // HTTP Basic Auth
    pub basic_auth_enabled: Option<bool>,
    pub basic_auth_username: Option<String>,
    /// Password in plain text - will be hashed before storing
    pub basic_auth_password: Option<String>,
    // Deployment commands (JSON arrays)
    pub pre_deploy_commands: Option<Vec<String>>,
    pub post_deploy_commands: Option<Vec<String>>,
    // Domain management
    pub domains: Option<Vec<Domain>>,
    // Docker Registry support
    /// Docker image name (e.g., "nginx", "ghcr.io/user/app") - set to empty string to clear
    pub docker_image: Option<String>,
    /// Docker image tag (default: "latest")
    pub docker_image_tag: Option<String>,
    /// Custom registry URL (null = Docker Hub)
    pub registry_url: Option<String>,
    /// Registry authentication username
    pub registry_username: Option<String>,
    /// Registry authentication password
    pub registry_password: Option<String>,
    /// Container labels (key-value pairs)
    pub container_labels: Option<std::collections::HashMap<String, String>>,
    // Build type and Nixpacks support
    /// Build type: "dockerfile", "nixpacks", or "static"
    pub build_type: Option<String>,
    /// Nixpacks-specific configuration (JSON object or JSON string)
    pub nixpacks_config: Option<serde_json::Value>,
    /// Publish directory for static site builds (e.g., "dist", "build", "out")
    pub publish_directory: Option<String>,
    /// Enable PR preview deployments
    pub preview_enabled: Option<bool>,
    /// GitHub App installation ID for GitHub-based deployments
    pub github_app_installation_id: Option<String>,
    // Rollback settings
    /// Enable automatic rollback on health check failure
    pub auto_rollback_enabled: Option<bool>,
    /// Enable pushing images to registry for rollback support
    pub registry_push_enabled: Option<bool>,
    /// Maximum number of deployment versions to keep for rollback
    pub max_rollback_versions: Option<i32>,
    /// Number of previous successful deployments to retain for rollback (1-50)
    pub rollback_retention_count: Option<i64>,
    // Approval and maintenance
    /// Require approval before deployments are executed
    pub require_approval: Option<bool>,
    /// Enable maintenance mode
    pub maintenance_mode: Option<bool>,
    /// Custom maintenance message
    pub maintenance_message: Option<String>,
    /// Preferred server ID for remote deployment (set to empty string to clear)
    pub server_id: Option<String>,
    /// Build server ID for offloading Docker builds (set to empty string to clear)
    pub build_server_id: Option<String>,
    /// Container restart policy: "always", "unless-stopped", "on-failure", or "never"
    pub restart_policy: Option<String>,
    // Custom Docker run options
    /// Run container in privileged mode
    pub privileged: Option<bool>,
    /// Capabilities to add (e.g. ["NET_ADMIN", "SYS_PTRACE"])
    pub cap_add: Option<Vec<String>>,
    /// Device mappings (e.g. ["/dev/snd:/dev/snd"])
    pub devices: Option<Vec<String>>,
    /// Shared memory size (e.g. "128m", "1g")
    pub shm_size: Option<String>,
    /// Run tini as PID 1 (init process)
    pub init_process: Option<bool>,
    /// Build-time secrets injected via BuildKit `--secret` (not baked into image layers)
    pub build_secrets: Option<Vec<BuildSecret>>,
    /// Target Docker build platform(s), e.g. "linux/amd64" or "linux/arm64".
    /// Set to empty string to clear and use the daemon default.
    pub build_platforms: Option<String>,
    /// Capabilities to drop (e.g. ["MKNOD"])
    pub docker_cap_drop: Option<Vec<String>>,
    /// GPU access: "all" or "device=0,1"
    pub docker_gpus: Option<String>,
    /// Ulimits (e.g. ["nofile=1024:1024"])
    pub docker_ulimits: Option<Vec<String>>,
    /// Security options (e.g. ["seccomp=unconfined"])
    pub docker_security_opt: Option<Vec<String>>,
    // Git clone options
    /// Pass --recurse-submodules to git clone
    pub git_submodules: Option<bool>,
    /// Run `git lfs pull` after clone
    pub git_lfs: Option<bool>,
    /// Use --depth 1 for fast shallow clones (default true)
    pub shallow_clone: Option<bool>,
    // Build options
    /// Pass --no-cache to docker build
    pub disable_build_cache: Option<bool>,
    /// Inject SOURCE_COMMIT build arg with the current git SHA
    pub include_source_commit: Option<bool>,
    // Container naming
    /// Override the container name (set to empty string to clear)
    pub custom_container_name: Option<String>,
    /// Treat this app as a static site (serve files without a runtime container)
    pub is_static_site: Option<bool>,
    /// URL prefix to strip from incoming requests before forwarding to the container
    pub strip_prefix: Option<String>,
    /// Inline Dockerfile content — set to empty string to clear
    pub inline_dockerfile: Option<String>,
    /// Docker destination (named network) — set to empty string to clear
    pub destination_id: Option<String>,
    /// Custom container labels (JSON string: [{key, value}]) — set to empty string to clear
    pub custom_labels: Option<String>,
}

/// Request specifically for updating domains
#[derive(Debug, Deserialize)]
pub struct UpdateDomainsRequest {
    pub domains: Vec<Domain>,
}

/// Request specifically for updating basic auth settings
#[derive(Debug, Deserialize)]
pub struct UpdateBasicAuthRequest {
    pub enabled: bool,
    pub username: Option<String>,
    /// Password in plain text - will be hashed before storing
    pub password: Option<String>,
}

// -------------------------------------------------------------------------
// App Sharing (Cross-Team)
// -------------------------------------------------------------------------

/// App share record for sharing apps between teams
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AppShare {
    pub id: String,
    pub app_id: String,
    pub shared_with_team_id: String,
    pub permission: String,
    pub created_at: String,
    pub created_by: Option<String>,
}

/// Response DTO for app share with team details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppShareResponse {
    pub id: String,
    pub app_id: String,
    pub shared_with_team_id: String,
    pub shared_with_team_name: String,
    pub permission: String,
    pub created_at: String,
    pub created_by: Option<String>,
    pub created_by_name: Option<String>,
}

/// Request to create a new app share
#[derive(Debug, Deserialize)]
pub struct CreateAppShareRequest {
    /// The team ID to share the app with
    pub team_id: String,
    /// Permission level (currently only "view" is supported)
    #[serde(default = "default_share_permission")]
    pub permission: String,
}

fn default_share_permission() -> String {
    "view".to_string()
}

fn default_replica_count() -> i64 {
    1
}

/// Response for app with sharing indicator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppWithSharing {
    #[serde(flatten)]
    pub app: App,
    /// Indicates if this app is shared with the requesting team (not owned)
    pub is_shared: bool,
    /// The team that owns this app (when is_shared is true)
    pub owner_team_name: Option<String>,
}
