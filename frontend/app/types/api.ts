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
  // Docker Registry support (alternative to git-based deployments)
  docker_image: string | null;
  docker_image_tag: string | null;
  registry_url: string | null;
  registry_username: string | null;
  // Container labels (JSON object stored as string)
  container_labels: string | null;
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
  databases: ManagedDatabase[];
  services: Service[];
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
  | "stopped"
  | "replaced";

export interface DeploymentLog {
  id: string;
  deployment_id: string;
  level: "info" | "warn" | "error";
  message: string;
  timestamp: string;
}

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
  /** Number of running databases */
  running_databases_count: number;
  /** Total number of databases */
  total_databases_count: number;
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

// App status for start/stop functionality
export interface AppStatus {
  app_id: string;
  container_id: string | null;
  running: boolean;
  status: "running" | "stopped" | "not_deployed" | "no_container" | "not_found";
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

// Disk space statistics for system monitoring
export interface DiskStats {
  /** Total disk space in bytes */
  total_bytes: number;
  /** Used disk space in bytes */
  used_bytes: number;
  /** Free disk space in bytes */
  free_bytes: number;
  /** Percentage of disk space used (0-100) */
  usage_percent: number;
  /** Path being monitored */
  path: string;
  /** Human-readable total (e.g., "100 GB") */
  total_human: string;
  /** Human-readable used (e.g., "80 GB") */
  used_human: string;
  /** Human-readable free (e.g., "20 GB") */
  free_human: string;
}

// System health check result
export interface CheckResult {
  /** Name of the check */
  name: string;
  /** Whether the check passed */
  passed: boolean;
  /** Whether this check is critical (failure should abort startup) */
  critical: boolean;
  /** Human-readable message describing the result */
  message: string;
  /** Additional details (optional) */
  details?: string;
}

// System health status from /api/system/health
export interface SystemHealthStatus {
  /** Overall system health */
  healthy: boolean;
  /** Database connectivity */
  database_healthy: boolean;
  /** Container runtime availability */
  runtime_healthy: boolean;
  /** Disk space status */
  disk_healthy: boolean;
  /** Individual check results */
  checks: CheckResult[];
  /** Rivetr version */
  version: string;
}

// -------------------------------------------------------------------------
// Notification types
// -------------------------------------------------------------------------

/** Notification channel types */
export type NotificationChannelType = "slack" | "discord" | "email";

/** Notification event types */
export type NotificationEventType =
  | "deployment_started"
  | "deployment_success"
  | "deployment_failed"
  | "app_stopped"
  | "app_started";

/** Slack webhook configuration */
export interface SlackConfig {
  webhook_url: string;
}

/** Discord webhook configuration */
export interface DiscordConfig {
  webhook_url: string;
}

/** Email (SMTP) configuration */
export interface EmailConfig {
  smtp_host: string;
  smtp_port: number;
  smtp_username?: string;
  smtp_password?: string;
  smtp_tls: boolean;
  from_address: string;
  to_addresses: string[];
}

/** Notification channel */
export interface NotificationChannel {
  id: string;
  name: string;
  channel_type: NotificationChannelType;
  config: SlackConfig | DiscordConfig | EmailConfig | Record<string, unknown>;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

/** Notification subscription */
export interface NotificationSubscription {
  id: string;
  channel_id: string;
  event_type: NotificationEventType;
  app_id: string | null;
  app_name: string | null;
  created_at: string;
}

/** Request to create a notification channel */
export interface CreateNotificationChannelRequest {
  name: string;
  channel_type: NotificationChannelType;
  config: SlackConfig | DiscordConfig | EmailConfig;
  enabled?: boolean;
}

/** Request to update a notification channel */
export interface UpdateNotificationChannelRequest {
  name?: string;
  config?: SlackConfig | DiscordConfig | EmailConfig;
  enabled?: boolean;
}

/** Request to create a notification subscription */
export interface CreateNotificationSubscriptionRequest {
  event_type: NotificationEventType;
  app_id?: string;
}

/** Request to test a notification channel */
export interface TestNotificationRequest {
  message?: string;
}

// -------------------------------------------------------------------------
// Team types
// -------------------------------------------------------------------------

/** Team roles with hierarchical permissions */
export type TeamRole = "owner" | "admin" | "developer" | "viewer";

/** Team entity */
export interface Team {
  id: string;
  name: string;
  slug: string;
  created_at: string;
  updated_at: string;
}

/** Team with member count for list views */
export interface TeamWithMemberCount {
  id: string;
  name: string;
  slug: string;
  created_at: string;
  updated_at: string;
  member_count: number;
  /** Current user's role in this team (if applicable) */
  user_role: TeamRole | null;
}

/** Team member entity */
export interface TeamMember {
  id: string;
  team_id: string;
  user_id: string;
  role: TeamRole;
  created_at: string;
}

/** Team member with user details */
export interface TeamMemberWithUser {
  id: string;
  team_id: string;
  user_id: string;
  role: TeamRole;
  created_at: string;
  user_name: string;
  user_email: string;
}

/** Team detail response with members */
export interface TeamDetail {
  id: string;
  name: string;
  slug: string;
  created_at: string;
  updated_at: string;
  members: TeamMemberWithUser[];
}

/** Request to create a new team */
export interface CreateTeamRequest {
  name: string;
  /** Optional slug (auto-generated from name if not provided) */
  slug?: string;
}

/** Request to update a team */
export interface UpdateTeamRequest {
  name?: string;
  slug?: string;
}

/** Request to invite/add a member to a team */
export interface InviteMemberRequest {
  /** User ID or email to invite */
  user_identifier: string;
  /** Role to assign */
  role: TeamRole;
}

/** Request to update a member's role */
export interface UpdateMemberRoleRequest {
  role: TeamRole;
}

/** Helper: Check if user has at least the required role */
export function hasRoleAtLeast(userRole: TeamRole | null, requiredRole: TeamRole): boolean {
  if (!userRole) return false;
  const roleOrder: TeamRole[] = ["viewer", "developer", "admin", "owner"];
  return roleOrder.indexOf(userRole) >= roleOrder.indexOf(requiredRole);
}

/** Helper: Check if user can manage team members */
export function canManageMembers(role: TeamRole | null): boolean {
  return hasRoleAtLeast(role, "admin");
}

/** Helper: Check if user can deploy apps */
export function canDeploy(role: TeamRole | null): boolean {
  return hasRoleAtLeast(role, "developer");
}

/** Helper: Check if user can manage apps (create/edit) */
export function canManageApps(role: TeamRole | null): boolean {
  return hasRoleAtLeast(role, "developer");
}

/** Helper: Check if user can delete apps */
export function canDeleteApps(role: TeamRole | null): boolean {
  return hasRoleAtLeast(role, "admin");
}

/** Helper: Check if user can delete the team */
export function canDeleteTeam(role: TeamRole | null): boolean {
  return role === "owner";
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
// Managed Database types
// -------------------------------------------------------------------------

/** Supported database types for managed databases */
export type DatabaseType = "postgres" | "mysql" | "mongodb" | "redis";

/** Database deployment status */
export type DatabaseStatus =
  | "pending"
  | "pulling"
  | "starting"
  | "running"
  | "stopped"
  | "failed";

/** Database credentials */
export interface DatabaseCredentials {
  username: string;
  password: string;
  database?: string;
  root_password?: string;
}

/** Managed database response */
export interface ManagedDatabase {
  id: string;
  name: string;
  db_type: DatabaseType;
  version: string;
  container_id: string | null;
  status: DatabaseStatus;
  internal_port: number;
  external_port: number;
  public_access: boolean;
  credentials: DatabaseCredentials;
  volume_name: string | null;
  volume_path: string | null;
  memory_limit: string | null;
  cpu_limit: string | null;
  internal_connection_string: string | null;
  external_connection_string: string | null;
  error_message: string | null;
  project_id: string | null;
  created_at: string;
  updated_at: string;
}

/** Request to create a managed database */
export interface CreateManagedDatabaseRequest {
  name: string;
  db_type: DatabaseType;
  version?: string;
  public_access?: boolean;
  /** Custom username (optional, auto-generated if not provided) */
  username?: string;
  /** Custom password (optional, auto-generated if not provided) */
  password?: string;
  /** Custom database name (optional, defaults to username) */
  database?: string;
  /** Custom root password for MySQL (optional, auto-generated if not provided) */
  root_password?: string;
  memory_limit?: string;
  cpu_limit?: string;
  project_id?: string;
}

/** Request to update a managed database */
export interface UpdateManagedDatabaseRequest {
  public_access?: boolean;
  memory_limit?: string;
  cpu_limit?: string;
}

/** Database type configuration (for UI) */
export interface DatabaseTypeInfo {
  type: DatabaseType;
  name: string;
  description: string;
  defaultPort: number;
  versions: string[];
  defaultVersion: string;
}

/** Database log entry */
export interface DatabaseLogEntry {
  timestamp: string;
  message: string;
  stream: "stdout" | "stderr";
}

/** Database backup status */
export type BackupStatus = "pending" | "running" | "completed" | "failed";

/** Database backup type */
export type BackupType = "manual" | "scheduled";

/** Schedule type for backups */
export type ScheduleType = "hourly" | "daily" | "weekly";

/** Database backup record */
export interface DatabaseBackup {
  id: string;
  database_id: string;
  backup_type: BackupType;
  status: BackupStatus;
  file_path?: string;
  file_size?: number;
  file_size_human?: string;
  backup_format?: string;
  started_at?: string;
  completed_at?: string;
  duration_seconds?: number;
  error_message?: string;
  created_at: string;
}

/** Database backup schedule */
export interface DatabaseBackupSchedule {
  id: string;
  database_id: string;
  enabled: boolean;
  schedule_type: ScheduleType;
  schedule_hour: number;
  schedule_day?: number;
  retention_count: number;
  last_run_at?: string;
  next_run_at?: string;
  created_at: string;
}

/** Request to create/update backup schedule */
export interface CreateBackupScheduleRequest {
  enabled?: boolean;
  schedule_type?: ScheduleType;
  schedule_hour?: number;
  schedule_day?: number;
  retention_count?: number;
}

// -------------------------------------------------------------------------
// Docker Compose Service types
// -------------------------------------------------------------------------

/** Service status */
export type ServiceStatus = "pending" | "running" | "stopped" | "failed";

/** Docker Compose service */
export interface Service {
  id: string;
  name: string;
  project_id: string | null;
  compose_content: string;
  status: ServiceStatus;
  error_message: string | null;
  created_at: string;
  updated_at: string;
}

/** Request to create a service */
export interface CreateServiceRequest {
  name: string;
  compose_content: string;
  project_id?: string;
}

/** Request to update a service */
export interface UpdateServiceRequest {
  compose_content?: string;
  project_id?: string;
}

// -------------------------------------------------------------------------
// Service Template types
// -------------------------------------------------------------------------

/** Template categories */
export type TemplateCategory =
  | "monitoring"
  | "database"
  | "storage"
  | "development"
  | "analytics"
  | "networking"
  | "security";

/** Environment variable schema entry */
export interface EnvSchemaEntry {
  name: string;
  label: string;
  required: boolean;
  default: string;
  secret: boolean;
}

/** Service template */
export interface ServiceTemplate {
  id: string;
  name: string;
  description: string | null;
  category: TemplateCategory;
  icon: string | null;
  compose_template: string;
  env_schema: EnvSchemaEntry[];
  is_builtin: boolean;
  created_at: string;
}

/** Request to deploy a template */
export interface DeployTemplateRequest {
  name: string;
  env_vars?: Record<string, string>;
  project_id?: string;
}

/** Response after deploying a template */
export interface DeployTemplateResponse {
  service_id: string;
  name: string;
  template_id: string;
  status: string;
  message: string;
}

/** Template category info for UI */
export interface TemplateCategoryInfo {
  id: TemplateCategory;
  name: string;
  description: string;
  icon: string;
}

/** Available template categories */
export const TEMPLATE_CATEGORIES: TemplateCategoryInfo[] = [
  { id: "monitoring", name: "Monitoring", description: "Observability and alerting tools", icon: "activity" },
  { id: "database", name: "Databases", description: "Database management systems", icon: "database" },
  { id: "storage", name: "Storage", description: "File storage and object stores", icon: "hard-drive" },
  { id: "development", name: "Development", description: "Developer tools and utilities", icon: "code" },
  { id: "analytics", name: "Analytics", description: "Data analytics and visualization", icon: "bar-chart" },
  { id: "networking", name: "Networking", description: "Network tools and proxies", icon: "network" },
  { id: "security", name: "Security", description: "Security and authentication", icon: "shield" },
];

/** Available database configurations */
export const DATABASE_TYPES: DatabaseTypeInfo[] = [
  {
    type: "postgres",
    name: "PostgreSQL",
    description: "The world's most advanced open source relational database",
    defaultPort: 5432,
    versions: ["16", "15", "14", "13", "12"],
    defaultVersion: "16",
  },
  {
    type: "mysql",
    name: "MySQL",
    description: "The most popular open source relational database",
    defaultPort: 3306,
    versions: ["8.0", "8.4", "5.7"],
    defaultVersion: "8.0",
  },
  {
    type: "mongodb",
    name: "MongoDB",
    description: "A document-oriented NoSQL database",
    defaultPort: 27017,
    versions: ["7", "6", "5", "4.4"],
    defaultVersion: "7",
  },
  {
    type: "redis",
    name: "Redis",
    description: "In-memory data structure store for caching and messaging",
    defaultPort: 6379,
    versions: ["7", "7.2", "6", "6.2"],
    defaultVersion: "7",
  },
];
