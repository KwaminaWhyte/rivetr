export type AppEnvironment = "development" | "staging" | "production";

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

// Project types
export interface Project {
  id: string;
  name: string;
  description: string | null;
  created_at: string;
  updated_at: string;
}

export interface ProjectWithApps extends Project {
  apps: App[];
}

export interface CreateProjectRequest {
  name: string;
  description?: string;
}

export interface UpdateProjectRequest {
  name?: string;
  description?: string;
}

export interface Deployment {
  id: string;
  app_id: string;
  status: DeploymentStatus;
  started_at: string;
  finished_at: string | null;
  container_id: string | null;
  error_message: string | null;
  commit_sha: string | null;
  commit_message: string | null;
}

export type DeploymentStatus =
  | "pending"
  | "cloning"
  | "building"
  | "starting"
  | "checking"
  | "running"
  | "failed"
  | "stopped";

export interface DeploymentLog {
  id: string;
  deployment_id: string;
  level: "info" | "warn" | "error";
  message: string;
  timestamp: string;
}

export interface CreateAppRequest {
  name: string;
  git_url: string;
  branch?: string;
  dockerfile?: string;
  domain?: string;
  port?: number;
  healthcheck?: string;
  cpu_limit?: string;
  memory_limit?: string;
  environment?: AppEnvironment;
  project_id?: string;
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
}

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

// Git Provider types
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

// Environment Variables
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

// System-wide statistics for dashboard
export interface SystemStats {
  /** Number of apps with a running deployment */
  running_apps_count: number;
  /** Total number of apps */
  total_apps_count: number;
  /** Aggregate CPU usage percentage across all running containers */
  total_cpu_percent: number;
  /** Aggregate memory usage in bytes across all running containers */
  memory_used_bytes: number;
  /** Total memory limit in bytes (sum of all container limits) */
  memory_total_bytes: number;
  /** Server uptime in seconds */
  uptime_seconds: number;
  /** Uptime percentage based on health checks */
  uptime_percent: number;
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
