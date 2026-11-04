//! Per-service deploy configuration: build source, images, hooks, pins.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::BTreeMap;

pub mod service_source_type {
    pub const NONE: &str = "none";
    pub const GIT: &str = "git";
    pub const PATH: &str = "path";
}

/// One image Rivetr builds before deploying.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceBuildImage {
    /// Optional label shown in logs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Target repository, e.g. `localhost:5000/megasign-app`. Built as
    /// `<image>:<revision>` and `<image>:latest`.
    pub image: String,
    /// Dockerfile path, relative to the source root or absolute.
    #[serde(default = "default_dockerfile")]
    pub dockerfile: String,
    /// Build context, relative to the source root or absolute.
    #[serde(default = "default_context")]
    pub context: String,
    /// Build args. Values may use `${COMMIT_SHA}`, `${SHORT_SHA}` and `${TAG}`.
    #[serde(default)]
    pub build_args: BTreeMap<String, String>,
    /// Multi-stage build target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

fn default_dockerfile() -> String {
    "Dockerfile".to_string()
}

fn default_context() -> String {
    ".".to_string()
}

/// A command run in a one-off container of a compose service.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceDeployHook {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Compose service whose image, env and networks the hook uses.
    pub service: String,
    /// Shell command, run with `sh -c`.
    pub command: String,
    /// Seconds before the hook is killed (default 600).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ServiceDeployConfig {
    pub service_id: String,
    pub source_type: String,
    pub git_url: Option<String>,
    pub branch: String,
    pub github_app_installation_id: Option<String>,
    pub git_provider_id: Option<String>,
    pub source_path: Option<String>,
    pub revision_file: Option<String>,
    pub images: String,
    pub push_images: i32,
    pub registry_username: Option<String>,
    #[serde(skip_serializing)]
    pub registry_password: Option<String>,
    pub pre_deploy_hooks: String,
    pub post_deploy_hooks: String,
    pub auto_deploy: i32,
    pub pinned_images: Option<String>,
    pub inject_env_file: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl ServiceDeployConfig {
    /// The config a service without a row behaves as (pull-only).
    pub fn default_for(service_id: &str) -> Self {
        Self {
            service_id: service_id.to_string(),
            source_type: service_source_type::NONE.to_string(),
            git_url: None,
            branch: "main".to_string(),
            github_app_installation_id: None,
            git_provider_id: None,
            source_path: None,
            revision_file: None,
            images: "[]".to_string(),
            push_images: 1,
            registry_username: None,
            registry_password: None,
            pre_deploy_hooks: "[]".to_string(),
            post_deploy_hooks: "[]".to_string(),
            auto_deploy: 0,
            pinned_images: None,
            inject_env_file: 0,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    pub fn build_images(&self) -> Vec<ServiceBuildImage> {
        serde_json::from_str(&self.images).unwrap_or_default()
    }

    pub fn pre_hooks(&self) -> Vec<ServiceDeployHook> {
        serde_json::from_str(&self.pre_deploy_hooks).unwrap_or_default()
    }

    pub fn post_hooks(&self) -> Vec<ServiceDeployHook> {
        serde_json::from_str(&self.post_deploy_hooks).unwrap_or_default()
    }

    pub fn pins(&self) -> BTreeMap<String, String> {
        self.pinned_images
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }

    /// True when deploys build images from source.
    pub fn builds_from_source(&self) -> bool {
        self.source_type != service_source_type::NONE && !self.build_images().is_empty()
    }

    pub fn to_response(&self) -> ServiceDeployConfigResponse {
        ServiceDeployConfigResponse {
            service_id: self.service_id.clone(),
            source_type: self.source_type.clone(),
            git_url: self.git_url.clone(),
            branch: self.branch.clone(),
            github_app_installation_id: self.github_app_installation_id.clone(),
            git_provider_id: self.git_provider_id.clone(),
            source_path: self.source_path.clone(),
            revision_file: self.revision_file.clone(),
            images: self.build_images(),
            push_images: self.push_images != 0,
            registry_username: self.registry_username.clone(),
            has_registry_password: self
                .registry_password
                .as_deref()
                .is_some_and(|p| !p.is_empty()),
            pre_deploy_hooks: self.pre_hooks(),
            post_deploy_hooks: self.post_hooks(),
            auto_deploy: self.auto_deploy != 0,
            pinned_images: self.pins(),
            inject_env_file: self.inject_env_file != 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDeployConfigResponse {
    pub service_id: String,
    pub source_type: String,
    pub git_url: Option<String>,
    pub branch: String,
    pub github_app_installation_id: Option<String>,
    pub git_provider_id: Option<String>,
    pub source_path: Option<String>,
    pub revision_file: Option<String>,
    pub images: Vec<ServiceBuildImage>,
    pub push_images: bool,
    pub registry_username: Option<String>,
    pub has_registry_password: bool,
    pub pre_deploy_hooks: Vec<ServiceDeployHook>,
    pub post_deploy_hooks: Vec<ServiceDeployHook>,
    pub auto_deploy: bool,
    pub pinned_images: BTreeMap<String, String>,
    pub inject_env_file: bool,
}

/// PUT body. Omitted fields keep their current value.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateServiceDeployConfigRequest {
    pub source_type: Option<String>,
    pub git_url: Option<String>,
    pub branch: Option<String>,
    pub github_app_installation_id: Option<String>,
    pub git_provider_id: Option<String>,
    pub source_path: Option<String>,
    pub revision_file: Option<String>,
    pub images: Option<Vec<ServiceBuildImage>>,
    pub push_images: Option<bool>,
    pub registry_username: Option<String>,
    /// Empty string clears the stored password.
    pub registry_password: Option<String>,
    pub pre_deploy_hooks: Option<Vec<ServiceDeployHook>>,
    pub post_deploy_hooks: Option<Vec<ServiceDeployHook>>,
    pub auto_deploy: Option<bool>,
    pub inject_env_file: Option<bool>,
}
