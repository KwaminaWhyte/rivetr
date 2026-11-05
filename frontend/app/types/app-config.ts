/**
 * Types for app configuration: structured healthcheck, container file browser and
 * resource cloning.
 */

export type HealthcheckType = "http" | "command";

export interface HealthcheckConfig {
  enabled: boolean;
  check_type: HealthcheckType;
  method: "GET" | "HEAD" | "POST" | "OPTIONS";
  scheme: "http" | "https";
  host: string;
  /** Container port; null uses the app port */
  port: number | null;
  path: string;
  expected_status: number;
  expected_body: string | null;
  command: string | null;
  interval_secs: number;
  timeout_secs: number;
  retries: number;
  start_period_secs: number;
}

export type ContainerFileKind = "dir" | "file" | "symlink" | "other";

export interface ContainerFileEntry {
  name: string;
  path: string;
  kind: ContainerFileKind;
  is_dir: boolean;
  size: number | null;
  /** Unix seconds */
  modified: number | null;
  permissions: string;
  owner: string;
  group: string;
  link_target: string | null;
}

export interface ContainerFileList {
  path: string;
  entries: ContainerFileEntry[];
}

export interface ContainerFileContent {
  path: string;
  content: string;
  size: number;
  binary: boolean;
}

export type CloneableResource = "app" | "database" | "service";

export interface CloneResourceRequest {
  name?: string;
  /** "" clears the project; omitted keeps the source project */
  project_id?: string;
  /** Apps only */
  environment_id?: string;
  /** Apps only; "" is this Rivetr host */
  server_id?: string;
  clone_volume_data: boolean;
}

export type CloneJobStatus = "pending" | "running" | "completed" | "failed";

export interface CloneJob {
  id: string;
  resource_type: CloneableResource;
  source_id: string;
  target_id: string;
  clone_volume_data: number;
  status: CloneJobStatus;
  progress: number;
  log: string;
  error: string | null;
  created_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface CloneResourceResponse {
  resource_type: CloneableResource;
  resource_id: string;
  name: string;
  job: CloneJob;
}
