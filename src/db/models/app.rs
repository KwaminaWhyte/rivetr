//! Application models and DTOs.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::common::{parse_domains, Domain, Environment, PortMapping};

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
    pub created_at: String,
    pub updated_at: String,
}

impl From<App> for AppResponse {
    fn from(app: App) -> Self {
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

    /// Get all domain names (including legacy domain and auto_subdomain)
    pub fn get_all_domain_names(&self) -> Vec<String> {
        let mut result = Vec::new();

        // Add domains from the domains array
        for d in self.get_domains() {
            result.push(d.domain.clone());
            // If redirect_www is enabled, add the www variant too
            if d.redirect_www {
                if d.domain.starts_with("www.") {
                    result.push(d.non_www_domain());
                } else {
                    result.push(d.www_domain());
                }
            }
        }

        // Add legacy domain if not already included
        if let Some(ref domain) = self.domain {
            if !result.contains(domain) {
                result.push(domain.clone());
            }
        }

        // Add auto_subdomain if present
        if let Some(ref subdomain) = self.auto_subdomain {
            if !result.contains(subdomain) {
                result.push(subdomain.clone());
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

    /// Parse network aliases from JSON string
    pub fn get_network_aliases(&self) -> Vec<String> {
        self.network_aliases
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
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
