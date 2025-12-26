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
  created_at: string;
  updated_at: string;
}

export interface Deployment {
  id: string;
  app_id: string;
  status: DeploymentStatus;
  started_at: string;
  finished_at: string | null;
  container_id: string | null;
  error_message: string | null;
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
