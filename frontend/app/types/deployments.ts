// -------------------------------------------------------------------------
// Deployment Types
// -------------------------------------------------------------------------

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
  git_tag: string | null;
  // Approval workflow fields
  approval_status: "pending" | "approved" | "rejected" | null;
  approved_by: string | null;
  approved_at: string | null;
  rejection_reason: string | null;
  // Scheduled deployment
  scheduled_at: string | null;
  // Registry push (image tag built and pushed)
  image_tag: string | null;
  // How the deployment was initiated: 'manual', 'webhook', 'rollback', 'restart', 'scheduled'
  trigger: string | null;
}

/** Git commit info from the commits list API */
export interface GitCommit {
  sha: string;
  message: string;
  author: string;
  date: string;
}

/** Git tag info from the tags list API */
export interface GitTag {
  name: string;
  sha: string;
}

/** Request body for triggering a deployment with optional commit/tag */
export interface TriggerDeployRequest {
  commit_sha?: string;
  git_tag?: string;
  /** ISO 8601 datetime to schedule the deployment for future execution */
  scheduled_at?: string;
}

/** Paginated response for deployment list */
export interface DeploymentListResponse {
  items: Deployment[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}

/** Query parameters for fetching deployments */
export interface DeploymentQuery {
  page?: number;
  per_page?: number;
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
  | "replaced"
  | "cancelled";

export interface DeploymentLog {
  id: string;
  deployment_id: string;
  level: "info" | "warn" | "error";
  message: string;
  timestamp: string;
}

/** Deployment freeze window — prevents deployments during a specified time range */
export interface DeploymentFreezeWindow {
  id: string;
  app_id: string | null;
  team_id: string | null;
  name: string;
  /** Start time in HH:MM UTC format */
  start_time: string;
  /** End time in HH:MM UTC format */
  end_time: string;
  /** Comma-separated days of week: 0=Sun, 1=Mon, ..., 6=Sat */
  days_of_week: string;
  is_active: boolean;
  created_at: string;
}

/** Request body for creating a freeze window */
export interface CreateFreezeWindowRequest {
  name: string;
  start_time: string;
  end_time: string;
  days_of_week: string;
  app_id?: string;
  team_id?: string;
}

/** Request body for rejecting a deployment */
export interface RejectDeploymentRequest {
  reason?: string;
}

// -------------------------------------------------------------------------
// Scheduled Job types
// -------------------------------------------------------------------------

/** Scheduled job that runs a command inside an app container on a cron schedule */
export interface ScheduledJob {
  id: string;
  app_id: string;
  name: string;
  command: string;
  cron_expression: string;
  enabled: boolean;
  last_run_at: string | null;
  next_run_at: string | null;
  created_at: string;
  updated_at: string;
}

/** Scheduled job run status */
export type ScheduledJobRunStatus = "running" | "success" | "failed";

/** A single execution record of a scheduled job */
export interface ScheduledJobRun {
  id: string;
  job_id: string;
  status: ScheduledJobRunStatus;
  output: string | null;
  error_message: string | null;
  started_at: string;
  finished_at: string | null;
  duration_ms: number | null;
}

/** Request to create a new scheduled job */
export interface CreateScheduledJobRequest {
  name: string;
  command: string;
  cron_expression: string;
  enabled?: boolean;
}

/** Request to update an existing scheduled job */
export interface UpdateScheduledJobRequest {
  name?: string;
  command?: string;
  cron_expression?: string;
  enabled?: boolean;
}

// -------------------------------------------------------------------------
// Scheduled Restart types
// -------------------------------------------------------------------------

/** Scheduled restart configuration */
export interface ScheduledRestart {
  id: string;
  app_id: string;
  cron_expression: string;
  enabled: boolean;
  last_restart: string | null;
  next_restart: string | null;
  created_at: string;
}

/** Request to create a scheduled restart */
export interface CreateScheduledRestartRequest {
  cron_expression: string;
  enabled?: boolean;
}

/** Request to update a scheduled restart */
export interface UpdateScheduledRestartRequest {
  cron_expression?: string;
  enabled?: boolean;
}
