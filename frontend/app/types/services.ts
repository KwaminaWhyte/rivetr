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
