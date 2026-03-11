// -------------------------------------------------------------------------
// App Environment & Build Types
// -------------------------------------------------------------------------

export type AppEnvironment = "development" | "staging" | "production";

/** Build type for applications */
export type BuildType =
  | "dockerfile"
  | "nixpacks"
  | "railpack"
  | "cnb"
  | "staticsite";

/** Deployment source type */
export type DeploymentSource = "git" | "upload" | "registry";

/** Result from build type auto-detection */
export interface BuildDetectionResult {
  /** Detected build type */
  build_type: BuildType | "dockercompose" | "dockerimage";
  /** Confidence level (0.0-1.0) */
  confidence: number;
  /** How the build type was detected */
  detected_from: string;
  /** Suggested publish directory for static sites */
  publish_directory?: string;
  /** Detected framework (e.g., "vite", "next", "create-react-app") */
  framework?: string;
  /** Detected language (e.g., "javascript", "typescript", "python") */
  language?: string;
}

/** Response from upload deploy endpoint */
export interface UploadDeployResponse {
  deployment: import("./deployments").Deployment;
  detected_build_type: BuildDetectionResult;
}

/** Response from upload create app endpoint */
export interface UploadAppResponse {
  app: App;
  deployment_id: string;
  detected_build_type: BuildDetectionResult;
}

/** Nixpacks configuration for auto-build */
export interface NixpacksConfig {
  /** Custom install command (overrides auto-detected) */
  install_cmd?: string;
  /** Custom build command (overrides auto-detected) */
  build_cmd?: string;
  /** Custom start command (overrides auto-detected) */
  start_cmd?: string;
  /** Additional Nix packages to install */
  packages?: string[];
  /** Additional apt packages to install */
  apt_packages?: string[];
}

// -------------------------------------------------------------------------
// Preview Deployment Types
// -------------------------------------------------------------------------

/** Preview deployment status */
export type PreviewDeploymentStatus =
  | "pending"
  | "cloning"
  | "building"
  | "starting"
  | "running"
  | "failed"
  | "closed";

/** Preview deployment for a pull request */
export interface PreviewDeployment {
  id: string;
  app_id: string;
  pr_number: number;
  pr_title: string | null;
  pr_source_branch: string;
  pr_target_branch: string;
  pr_author: string | null;
  pr_url: string | null;
  provider_type: "github" | "gitlab" | "gitea";
  repo_full_name: string;
  preview_domain: string;
  container_id: string | null;
  container_name: string | null;
  image_tag: string | null;
  port: number | null;
  commit_sha: string | null;
  commit_message: string | null;
  status: PreviewDeploymentStatus;
  error_message: string | null;
  github_comment_id: number | null;
  memory_limit: string | null;
  cpu_limit: string | null;
  created_at: string;
  updated_at: string;
  closed_at: string | null;
}

// -------------------------------------------------------------------------
// GitHub App Types
// -------------------------------------------------------------------------

/** GitHub App configuration */
export interface GitHubApp {
  id: string;
  name: string;
  app_id: number;
  client_id: string;
  slug: string | null;
  owner: string | null;
  permissions: string | null;
  events: string | null;
  is_system_wide: boolean;
  team_id: string | null;
  created_at: string;
  updated_at: string;
  created_by: string;
}

/** GitHub App installation */
export interface GitHubAppInstallation {
  id: string;
  github_app_id: string;
  installation_id: number;
  account_type: "user" | "organization";
  account_login: string;
  account_id: number;
  permissions: string | null;
  repository_selection: "all" | "selected" | null;
  suspended_at: string | null;
  created_at: string;
  updated_at: string;
}

/** Repository from GitHub App installation */
export interface GitHubAppRepository {
  id: number;
  name: string;
  full_name: string;
  description: string | null;
  html_url: string;
  clone_url: string;
  ssh_url: string;
  default_branch: string;
  private: boolean;
  owner: string;
  installation_id: string;
}

/** Git branch from a repository */
export interface GitHubBranch {
  name: string;
  protected: boolean;
  commit: {
    sha: string;
    url: string;
  };
}

/** GitHub App manifest request */
export interface CreateGitHubAppManifestRequest {
  name: string;
  is_system_wide: boolean;
  team_id?: string;
}

/** GitHub App manifest response */
export interface GitHubAppManifestResponse {
  manifest_url: string;
  manifest: string;
  state: string;
}

// -------------------------------------------------------------------------
// Network / Domain Configuration
// -------------------------------------------------------------------------

/** Port mapping configuration for containers */
export interface PortMapping {
  /** Host port to bind (0 for auto-assign) */
  host_port: number;
  /** Container port to expose */
  container_port: number;
  /** Protocol (tcp or udp) */
  protocol: string;
}

/** Domain configuration for an application */
export interface Domain {
  /** The domain name (e.g., "example.com") */
  domain: string;
  /** Whether this is the primary domain for the app */
  primary: boolean;
  /** Whether to redirect www to non-www (or vice versa) */
  redirect_www: boolean;
}

// -------------------------------------------------------------------------
// App Core Types
// -------------------------------------------------------------------------

export interface App {
  id: string;
  name: string;
  git_url: string;
  branch: string;
  dockerfile: string;
  domain: string | null;
  port: number;
  healthcheck: string | null;
  memory_limit: string | null;
  cpu_limit: string | null;
  environment: AppEnvironment;
  project_id: string | null;
  /** Environment ID for project environment scoping */
  environment_id: string | null;
  /** Team ID for multi-tenant scoping (null for legacy/unassigned apps) */
  team_id: string | null;
  // Advanced build options
  dockerfile_path: string | null;
  base_directory: string | null;
  build_target: string | null;
  watch_paths: string | null;
  custom_docker_options: string | null;
  // Network configuration (stored as JSON strings)
  port_mappings: string | null;
  network_aliases: string | null;
  extra_hosts: string | null;
  // HTTP Basic Auth
  basic_auth_enabled: boolean;
  basic_auth_username: string | null;
  // Deployment commands (stored as JSON strings)
  pre_deploy_commands: string | null;
  post_deploy_commands: string | null;
  // Domain management (stored as JSON string)
  domains: string | null;
  auto_subdomain: string | null;
  // Docker Registry support (alternative to git-based deployments)
  docker_image: string | null;
  docker_image_tag: string | null;
  registry_url: string | null;
  registry_username: string | null;
  // Container labels (JSON object stored as string)
  container_labels: string | null;
  // Build type configuration
  build_type: BuildType;
  nixpacks_config: string | null;
  publish_directory: string | null;
  // Preview deployments
  preview_enabled: boolean;
  // GitHub App installation (for auto-deploy)
  github_app_installation_id: string | null;
  // Deployment source
  deployment_source?: DeploymentSource;
  // Rollback settings
  auto_rollback_enabled: boolean;
  registry_push_enabled: boolean;
  max_rollback_versions: number;
  rollback_retention_count: number;
  // Replica settings
  replica_count: number;
  // Deployment approval and maintenance mode
  require_approval: boolean;
  maintenance_mode: boolean;
  maintenance_message: string | null;
  created_at: string;
  updated_at: string;
}

// HTTP Basic Auth
export interface BasicAuthStatus {
  enabled: boolean;
  username: string | null;
}

export interface UpdateBasicAuthRequest {
  enabled: boolean;
  username?: string;
  password?: string;
}

// -------------------------------------------------------------------------
// App Sharing types
// -------------------------------------------------------------------------

/** App share response with team details */
export interface AppShare {
  id: string;
  app_id: string;
  shared_with_team_id: string;
  shared_with_team_name: string;
  permission: string;
  created_at: string;
  created_by: string | null;
  created_by_name: string | null;
}

/** Request to create a new app share */
export interface CreateAppShareRequest {
  /** The team ID to share the app with */
  team_id: string;
  /** Permission level (currently only "view" is supported) */
  permission?: string;
}

/** App with sharing indicator (for list views) */
export interface AppWithSharing extends App {
  /** Indicates if this app is shared with the requesting team (not owned) */
  is_shared: boolean;
  /** The team that owns this app (when is_shared is true) */
  owner_team_name: string | null;
}

// -------------------------------------------------------------------------
// Project types
// -------------------------------------------------------------------------

export interface Project {
  id: string;
  name: string;
  description: string | null;
  team_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface ProjectWithApps extends Project {
  apps: App[];
  databases: import("./databases").ManagedDatabase[];
  services: import("./services").Service[];
}

export interface CreateProjectRequest {
  name: string;
  description?: string;
  team_id?: string;
}

export interface UpdateProjectRequest {
  name?: string;
  description?: string;
}

// Project Environment types
export interface ProjectEnvironment {
  id: string;
  project_id: string;
  name: string;
  description: string | null;
  is_default: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateEnvironmentRequest {
  name: string;
  description?: string;
}

export interface UpdateEnvironmentRequest {
  name?: string;
  description?: string;
}

export interface EnvironmentEnvVar {
  id: string;
  environment_id: string;
  key: string;
  value: string;
  is_secret: boolean;
  created_at: string;
}

export interface CreateEnvironmentEnvVarRequest {
  key: string;
  value: string;
  is_secret?: boolean;
}

export interface UpdateEnvironmentEnvVarRequest {
  value?: string;
  is_secret?: boolean;
}

// -------------------------------------------------------------------------
// Create / Update App Requests
// -------------------------------------------------------------------------

export interface CreateAppRequest {
  name: string;
  /** Git URL for source-based deployments (required if docker_image is not set) */
  git_url?: string;
  branch?: string;
  dockerfile?: string;
  domain?: string;
  port?: number;
  healthcheck?: string;
  cpu_limit?: string;
  memory_limit?: string;
  environment?: AppEnvironment;
  project_id?: string;
  /** Team ID for multi-tenant scoping */
  team_id?: string;
  // Advanced build options
  dockerfile_path?: string;
  base_directory?: string;
  build_target?: string;
  watch_paths?: string;
  custom_docker_options?: string;
  // Network configuration
  port_mappings?: PortMapping[];
  network_aliases?: string[];
  extra_hosts?: string[];
  // Deployment commands
  pre_deploy_commands?: string[];
  post_deploy_commands?: string[];
  // Domain management
  domains?: Domain[];
  // Docker Registry support (alternative to git-based deployments)
  /** Docker image name (e.g., "nginx", "ghcr.io/user/app") */
  docker_image?: string;
  /** Docker image tag (default: "latest") */
  docker_image_tag?: string;
  /** Custom registry URL (null = Docker Hub) */
  registry_url?: string;
  /** Registry authentication username */
  registry_username?: string;
  /** Registry authentication password */
  registry_password?: string;
  // Container labels
  container_labels?: Record<string, string>;
  // Build type configuration
  build_type?: BuildType;
  nixpacks_config?: NixpacksConfig;
  publish_directory?: string;
  // Preview deployments
  preview_enabled?: boolean;
  // GitHub App installation
  github_app_installation_id?: string;
  // Git provider (OAuth) link for authenticated HTTPS cloning
  git_provider_id?: string;
}

export interface UpdateAppRequest {
  name?: string;
  git_url?: string;
  branch?: string;
  dockerfile?: string;
  domain?: string;
  port?: number;
  healthcheck?: string;
  ssh_key_id?: string | null;
  cpu_limit?: string;
  memory_limit?: string;
  environment?: AppEnvironment;
  project_id?: string | null;
  // Advanced build options
  dockerfile_path?: string;
  base_directory?: string;
  build_target?: string;
  watch_paths?: string;
  custom_docker_options?: string;
  // Network configuration
  port_mappings?: PortMapping[];
  network_aliases?: string[];
  extra_hosts?: string[];
  // Deployment commands
  pre_deploy_commands?: string[];
  post_deploy_commands?: string[];
  // Domain management
  domains?: Domain[];
  // Docker Registry support
  /** Docker image name (e.g., "nginx", "ghcr.io/user/app") - set to empty string to clear */
  docker_image?: string;
  /** Docker image tag (default: "latest") */
  docker_image_tag?: string;
  /** Custom registry URL (null = Docker Hub) */
  registry_url?: string;
  /** Registry authentication username */
  registry_username?: string;
  /** Registry authentication password */
  registry_password?: string;
  // Container labels
  container_labels?: Record<string, string>;
  // Build type configuration
  build_type?: BuildType;
  nixpacks_config?: NixpacksConfig;
  publish_directory?: string;
  // Preview deployments
  preview_enabled?: boolean;
  // GitHub App installation
  github_app_installation_id?: string | null;
  // Rollback settings
  auto_rollback_enabled?: boolean;
  registry_push_enabled?: boolean;
  max_rollback_versions?: number;
  rollback_retention_count?: number;
  // Deployment approval and maintenance
  require_approval?: boolean;
  maintenance_mode?: boolean;
  maintenance_message?: string;
}

// -------------------------------------------------------------------------
// Autoscaling types
// -------------------------------------------------------------------------

export interface AutoscalingRule {
  id: string;
  app_id: string;
  metric: "cpu" | "memory" | "request_rate";
  scale_up_threshold: number;
  scale_down_threshold: number;
  min_replicas: number;
  max_replicas: number;
  cooldown_seconds: number;
  enabled: number;
  last_scaled_at: string | null;
  created_at: string;
}

export interface CreateAutoscalingRuleRequest {
  metric: "cpu" | "memory" | "request_rate";
  scale_up_threshold: number;
  scale_down_threshold: number;
  min_replicas?: number;
  max_replicas?: number;
  cooldown_seconds?: number;
  enabled?: boolean;
}

// -------------------------------------------------------------------------
// Template Suggestion types
// -------------------------------------------------------------------------

export interface TemplateSuggestion {
  id: string;
  name: string;
  description: string;
  docker_image: string;
  category: string;
  website_url: string | null;
  notes: string | null;
  status: "pending" | "approved" | "rejected";
  submitted_by: string | null;
  reviewed_by: string | null;
  reviewed_at: string | null;
  created_at: string;
}

export interface TemplateSuggestionRequest {
  name: string;
  description: string;
  docker_image: string;
  category: string;
  website_url?: string;
  notes?: string;
}

// -------------------------------------------------------------------------
// SSH Key types
// -------------------------------------------------------------------------

export interface SshKey {
  id: string;
  name: string;
  public_key: string | null;
  app_id: string | null;
  is_global: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateSshKeyRequest {
  name: string;
  private_key: string;
  public_key?: string;
  app_id?: string;
  is_global?: boolean;
}

export interface UpdateSshKeyRequest {
  name?: string;
  private_key?: string;
  public_key?: string;
  app_id?: string | null;
  is_global?: boolean;
}

// -------------------------------------------------------------------------
// Git Provider types
// -------------------------------------------------------------------------

export type GitProviderType = "github" | "gitlab" | "bitbucket";

export interface GitProvider {
  id: string;
  provider: GitProviderType;
  username: string;
  display_name: string | null;
  email: string | null;
  avatar_url: string | null;
  scopes: string | null;
  created_at: string;
  updated_at: string;
}

export interface GitRepository {
  id: string;
  name: string;
  full_name: string;
  description: string | null;
  html_url: string;
  clone_url: string;
  ssh_url: string;
  default_branch: string;
  private: boolean;
  owner: string;
}

export interface OAuthAuthorizationResponse {
  authorization_url: string;
  state: string;
}

// -------------------------------------------------------------------------
// Environment Variables
// -------------------------------------------------------------------------

export interface EnvVar {
  id: string;
  app_id: string;
  key: string;
  value: string;
  is_secret: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateEnvVarRequest {
  key: string;
  value: string;
  is_secret?: boolean;
}

export interface UpdateEnvVarRequest {
  value?: string;
  is_secret?: boolean;
}

// -------------------------------------------------------------------------
// Shared Environment Variables (Team-level and Project-level)
// -------------------------------------------------------------------------

/** Team-level shared environment variable (inherited by all apps in the team) */
export interface TeamEnvVar {
  id: string;
  team_id: string;
  key: string;
  value: string;
  is_secret: boolean;
  description: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateTeamEnvVarRequest {
  key: string;
  value: string;
  is_secret?: boolean;
  description?: string;
}

export interface UpdateTeamEnvVarRequest {
  value?: string;
  is_secret?: boolean;
  description?: string;
}

/** Project-level shared environment variable (inherited by all apps in the project) */
export interface ProjectEnvVar {
  id: string;
  project_id: string;
  key: string;
  value: string;
  is_secret: boolean;
  description: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateProjectEnvVarRequest {
  key: string;
  value: string;
  is_secret?: boolean;
  description?: string;
}

export interface UpdateProjectEnvVarRequest {
  value?: string;
  is_secret?: boolean;
  description?: string;
}

/** The source level for a resolved environment variable */
export type EnvVarSource = "app" | "environment" | "project" | "team";

/** A resolved environment variable with effective value and its inheritance source */
export interface ResolvedEnvVar {
  key: string;
  /** Secrets are masked as `***` */
  value: string;
  is_secret: boolean;
  /** Where this variable comes from in the inheritance chain */
  source: EnvVarSource;
  description: string | null;
}

// -------------------------------------------------------------------------
// Container / App Status
// -------------------------------------------------------------------------

// Container resource statistics
export interface ContainerStats {
  /** CPU usage percentage (0-100, can exceed 100 on multi-core) */
  cpu_percent: number;
  /** Current memory usage in bytes */
  memory_usage: number;
  /** Memory limit in bytes (0 if no limit) */
  memory_limit: number;
  /** Network bytes received */
  network_rx: number;
  /** Network bytes transmitted */
  network_tx: number;
}

// App status for start/stop functionality
export interface AppStatus {
  app_id: string;
  container_id: string | null;
  running: boolean;
  status: "running" | "stopped" | "not_deployed" | "no_container" | "not_found";
  /** The host port the container is accessible on (for "Open App" functionality) */
  host_port: number | null;
  /** Blue/green deployment phase */
  deployment_phase: "stable" | "deploying" | "health_checking" | "switching";
  /** ID of the currently active deployment */
  active_deployment_id: string | null;
  /** Seconds the active deployment has been running */
  uptime_seconds: number | null;
}

// -------------------------------------------------------------------------
// Dependency Graph types
// -------------------------------------------------------------------------

export interface DependencyNode {
  id: string;
  type: "app" | "database" | "service";
  name: string;
  status: string | null;
}

export interface DependencyEdge {
  from: string;
  to: string;
  label: string;
}

export interface DependencyGraph {
  nodes: DependencyNode[];
  edges: DependencyEdge[];
}

export interface AddDependencyRequest {
  depends_on_app_id?: string;
  depends_on_database_id?: string;
  depends_on_service_id?: string;
}

export interface AddDependencyResponse {
  id: string;
  app_id: string;
  depends_on_app_id: string | null;
  depends_on_database_id: string | null;
  depends_on_service_id: string | null;
  created_at: string;
}

// Recent event for dashboard feed
export interface RecentEvent {
  /** Unique event ID */
  id: string;
  /** App name this event is associated with */
  app_name: string;
  /** App ID */
  app_id: string;
  /** Type of event: "deployed", "failed", "building", "pending", "stopped" */
  event_type: string;
  /** Event status for display: "success", "error", "warning", "info" */
  status: "success" | "error" | "warning" | "info";
  /** Human-readable message */
  message: string;
  /** When the event occurred (ISO 8601 timestamp) */
  timestamp: string;
}

// -------------------------------------------------------------------------
// Volume types
// -------------------------------------------------------------------------

/** Volume mount for persistent storage */
export interface Volume {
  id: string;
  app_id: string;
  name: string;
  host_path: string;
  container_path: string;
  read_only: boolean;
  created_at: string;
  updated_at: string;
}

/** Request to create a volume */
export interface CreateVolumeRequest {
  name: string;
  host_path: string;
  container_path: string;
  read_only?: boolean;
}

/** Request to update a volume */
export interface UpdateVolumeRequest {
  name?: string;
  host_path?: string;
  container_path?: string;
  read_only?: boolean;
}

// -------------------------------------------------------------------------
// Bulk Operations
// -------------------------------------------------------------------------

/** Result for one app in a bulk operation */
export interface BulkAppResult {
  app_id: string;
  success: boolean;
  error: string | null;
}

/** Response for bulk start / stop / restart / deploy */
export interface BulkOperationResponse {
  results: BulkAppResult[];
}

/** Request body for bulk operations */
export interface BulkAppIdsRequest {
  app_ids: string[];
}

// -------------------------------------------------------------------------
// Config Snapshots
// -------------------------------------------------------------------------

/** Config snapshot of an app's configuration */
export interface ConfigSnapshot {
  id: string;
  app_id: string;
  name: string;
  description: string | null;
  config_json: string;
  env_vars_json: string;
  created_by: string | null;
  created_at: string;
}

/** Request to create a config snapshot */
export interface CreateSnapshotRequest {
  name: string;
  description?: string;
}

/** Request to clone an app */
export interface CloneAppRequest {
  name?: string;
}

/** Response from clone app */
export interface CloneAppResponse {
  app: App;
}

/** Request to toggle maintenance mode */
export interface MaintenanceModeRequest {
  enabled: boolean;
  message?: string;
}

/** Response from maintenance mode toggle */
export interface MaintenanceModeResponse {
  app_id: string;
  maintenance_mode: boolean;
  maintenance_message: string | null;
}

// -------------------------------------------------------------------------
// Project Export / Import
// -------------------------------------------------------------------------

/** Env var entry in a project export */
export interface ExportEnvVar {
  key: string;
  value: string;
  is_secret: boolean;
}

/** Volume entry in a project export */
export interface ExportVolume {
  name: string;
  host_path: string;
  container_path: string;
  read_only: boolean;
}

/** App entry in a project export */
export interface ExportApp {
  name: string;
  git_url: string;
  branch: string;
  dockerfile: string;
  port: number;
  healthcheck: string | null;
  memory_limit: string | null;
  cpu_limit: string | null;
  environment: string;
  dockerfile_path: string | null;
  base_directory: string | null;
  build_target: string | null;
  pre_deploy_commands: string | null;
  post_deploy_commands: string | null;
  domains: string | null;
  docker_image: string | null;
  docker_image_tag: string | null;
  build_type: string | null;
  env_vars: ExportEnvVar[];
  volumes: ExportVolume[];
}

/** Full project export envelope */
export interface ProjectExport {
  project_name: string;
  export_version: number;
  apps: ExportApp[];
}

/** Response from project import */
export interface ProjectImportResponse {
  apps_created: number;
  app_ids: string[];
}
