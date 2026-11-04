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
  team_id: string | null;
  compose_content: string;
  domain: string | null;
  port: number;
  status: ServiceStatus;
  error_message: string | null;
  /** When true (default), the service runs in a dedicated Docker network isolated from other services. */
  isolated_network: boolean;
  /** When true, the compose file is deployed verbatim without Rivetr network/label injections. */
  raw_compose_mode: boolean;
  /** When true, the service exposes a port directly on the host. */
  public_access: boolean;
  /** Host port bound when public_access is true (e.g. 6380). */
  external_port: number;
  /** Container port exposed when public_access is true (e.g. 6379). */
  expose_container_port: number;
  /** Optional CPU limit (e.g. "0.5") applied to each compose service. */
  cpu_limit: string | null;
  /** Optional memory limit (e.g. "512M") applied to each compose service. */
  memory_limit: string | null;
  created_at: string;
  updated_at: string;
}

/** Request to create a service */
export interface CreateServiceRequest {
  name: string;
  compose_content: string;
  project_id?: string;
  team_id?: string;
  domain?: string;
  port?: number;
}

/** Request to update a service */
export interface UpdateServiceRequest {
  compose_content?: string;
  project_id?: string;
  domain?: string;
  port?: number;
  /** Toggle isolated Docker network for this service */
  isolated_network?: boolean;
  /** Toggle raw compose mode for this service */
  raw_compose_mode?: boolean;
  /** Enable/disable exposing the service port on the host */
  public_access?: boolean;
  /** Host port to bind when public_access is enabled */
  external_port?: number;
  /** Container port to expose when public_access is enabled */
  expose_container_port?: number;
  /** CPU limit (e.g. "0.5"); empty string clears it */
  cpu_limit?: string;
  /** Memory limit (e.g. "512M"); empty string clears it */
  memory_limit?: string;
}

/** Service log entry */
export interface ServiceLogEntry {
  timestamp: string;
  service: string;
  message: string;
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
  | "security"
  | "ai"
  | "automation"
  | "cms"
  | "communication";

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
  {
    id: "monitoring",
    name: "Monitoring",
    description: "Observability and alerting tools",
    icon: "activity",
  },
  {
    id: "database",
    name: "Databases",
    description: "Database management systems",
    icon: "database",
  },
  {
    id: "storage",
    name: "Storage",
    description: "File storage and object stores",
    icon: "hard-drive",
  },
  {
    id: "development",
    name: "Development",
    description: "Developer tools and utilities",
    icon: "code",
  },
  {
    id: "analytics",
    name: "Analytics",
    description: "Data analytics and visualization",
    icon: "bar-chart",
  },
  {
    id: "networking",
    name: "Networking",
    description: "Network tools and proxies",
    icon: "network",
  },
  {
    id: "security",
    name: "Security",
    description: "Security and authentication",
    icon: "shield",
  },
  {
    id: "ai",
    name: "AI / ML",
    description: "Artificial intelligence and machine learning tools",
    icon: "brain",
  },
  {
    id: "automation",
    name: "Automation",
    description: "Workflow automation and orchestration",
    icon: "zap",
  },
  {
    id: "cms",
    name: "CMS",
    description: "Content management systems",
    icon: "file-text",
  },
  {
    id: "communication",
    name: "Communication",
    description: "Team chat and messaging platforms",
    icon: "message-circle",
  },
];

// -------------------------------------------------------------------------
// Service deployments
// -------------------------------------------------------------------------

export type ServiceDeploymentStatus =
  | "pending"
  | "building"
  | "deploying"
  | "succeeded"
  | "failed";

export type ServiceDeploymentTrigger = "manual" | "webhook" | "rollback" | "api";

/** The image one compose service ran at a point in time. */
export interface ServiceImageState {
  image: string;
  image_id?: string;
  /** Registry digest reference, e.g. `localhost:5000/app@sha256:...` */
  digest?: string;
}

export interface ServiceDeployment {
  id: string;
  service_id: string;
  trigger: ServiceDeploymentTrigger | string;
  triggered_by: string | null;
  triggered_by_name: string | null;
  status: ServiceDeploymentStatus;
  commit_sha: string | null;
  commit_message: string | null;
  image_tag: string | null;
  images_before: Record<string, ServiceImageState>;
  images_after: Record<string, ServiceImageState>;
  rollback_of: string | null;
  error_message: string | null;
  started_at: string;
  finished_at: string | null;
  duration_ms: number | null;
}

export interface ServiceDeploymentLog {
  id: number;
  deployment_id: string;
  timestamp: string;
  level: string;
  message: string;
}

export const ACTIVE_SERVICE_DEPLOYMENT_STATUSES: ServiceDeploymentStatus[] = [
  "pending",
  "building",
  "deploying",
];

// -------------------------------------------------------------------------
// Service deploy configuration (build from source, hooks)
// -------------------------------------------------------------------------

export type ServiceSourceType = "none" | "git" | "path";

export interface ServiceBuildImage {
  name?: string;
  /** Target repository, e.g. localhost:5000/megasign-app */
  image: string;
  dockerfile: string;
  context: string;
  build_args: Record<string, string>;
  target?: string;
}

export interface ServiceDeployHook {
  name?: string;
  /** Compose service whose image, env and networks the hook uses */
  service: string;
  command: string;
  timeout_secs?: number;
}

export interface ServiceDeployConfig {
  service_id: string;
  source_type: ServiceSourceType;
  git_url: string | null;
  branch: string;
  github_app_installation_id: string | null;
  git_provider_id: string | null;
  source_path: string | null;
  revision_file: string | null;
  images: ServiceBuildImage[];
  push_images: boolean;
  registry_username: string | null;
  has_registry_password: boolean;
  pre_deploy_hooks: ServiceDeployHook[];
  post_deploy_hooks: ServiceDeployHook[];
  auto_deploy: boolean;
  pinned_images: Record<string, string>;
  inject_env_file: boolean;
}

export interface UpdateServiceDeployConfigRequest {
  source_type?: ServiceSourceType;
  git_url?: string;
  branch?: string;
  github_app_installation_id?: string;
  git_provider_id?: string;
  source_path?: string;
  revision_file?: string;
  images?: ServiceBuildImage[];
  push_images?: boolean;
  registry_username?: string;
  /** Empty string clears the stored password; omit to keep it */
  registry_password?: string;
  pre_deploy_hooks?: ServiceDeployHook[];
  post_deploy_hooks?: ServiceDeployHook[];
  auto_deploy?: boolean;
  inject_env_file?: boolean;
}

// -------------------------------------------------------------------------
// Service containers
// -------------------------------------------------------------------------

export interface ServiceContainerPort {
  URL: string;
  TargetPort: number;
  PublishedPort: number;
  Protocol: string;
}

export interface ServiceContainer {
  id: string;
  name: string;
  /** Compose service name */
  service: string;
  image: string;
  state: string;
  /** healthy | unhealthy | starting | "" when the image has no healthcheck */
  health: string;
  status: string;
  created_at: string;
  exit_code: number;
  ports: ServiceContainerPort[];
  cpu_percent: number | null;
  memory_usage: number | null;
  memory_limit: number | null;
}

export interface ServiceContainerLogLine {
  timestamp: string;
  stream: "stdout" | "stderr";
  message: string;
}

// -------------------------------------------------------------------------
// Service environment variables
// -------------------------------------------------------------------------

export interface ServiceEnvVar {
  id: string;
  service_id: string;
  key: string;
  /** "********" for secrets unless revealed */
  value: string;
  is_secret: boolean;
  created_at: string;
  updated_at: string;
}

export interface ServiceEnvImportResult {
  created: number;
  updated: number;
  skipped: string[];
}
