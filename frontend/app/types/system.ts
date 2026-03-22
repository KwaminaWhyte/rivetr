// -------------------------------------------------------------------------
// System-wide Statistics
// -------------------------------------------------------------------------

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
  /** Number of running services (Docker Compose) */
  running_services_count: number;
  /** Total number of services */
  total_services_count: number;
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
// Auto-Update types
// -------------------------------------------------------------------------

/** Update status information */
export interface UpdateStatus {
  /** Current running version */
  current_version: string;
  /** Latest available version (null if check failed or up-to-date) */
  latest_version: string | null;
  /** Whether an update is available */
  update_available: boolean;
  /** URL to download the update */
  download_url: string | null;
  /** Release notes/changelog */
  release_notes: string | null;
  /** Release page URL */
  release_url: string | null;
  /** When the last check was performed (ISO 8601) */
  last_checked: string | null;
  /** Error message if the last check failed */
  last_error: string | null;
  /** Whether auto-update is enabled */
  auto_update_enabled: boolean;
  /** Whether auto-apply is enabled */
  auto_apply_enabled: boolean;
}

// -------------------------------------------------------------------------
// Instance Backup & Restore types
// -------------------------------------------------------------------------

/** Information about a backup file */
export interface BackupInfo {
  /** Filename of the backup */
  name: string;
  /** Size in bytes */
  size: number;
  /** ISO 8601 timestamp when the backup was created */
  created_at: string;
}

/** Result of a restore operation */
export interface RestoreResult {
  /** Whether the database was restored */
  database_restored: boolean;
  /** Whether the config was restored */
  config_restored: boolean;
  /** Whether SSL certificates were restored */
  certs_restored: boolean;
  /** Warning messages */
  warnings: string[];
}

// -------------------------------------------------------------------------
// Audit Log types
// -------------------------------------------------------------------------

/** Audit log entry */
export interface AuditLog {
  id: string;
  action: string;
  resource_type: string;
  resource_id: string | null;
  resource_name: string | null;
  user_id: string | null;
  /** Email of the user who performed the action (resolved from users table) */
  user_email: string | null;
  ip_address: string | null;
  details: string | null;
  created_at: string;
}

/** Paginated audit log response */
export interface AuditLogListResponse {
  items: AuditLog[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}

/** Query parameters for fetching audit logs */
export interface AuditLogQuery {
  action?: string;
  resource_type?: string;
  resource_id?: string;
  user_id?: string;
  start_date?: string;
  end_date?: string;
  page?: number;
  per_page?: number;
}

// -------------------------------------------------------------------------
// S3 Storage & Backup Types
// -------------------------------------------------------------------------

/** S3 storage configuration */
export interface S3StorageConfig {
  id: string;
  name: string;
  endpoint: string | null;
  bucket: string;
  region: string;
  access_key: string;
  secret_key: string;
  path_prefix: string | null;
  is_default: boolean;
  team_id: string | null;
  created_at: string;
  updated_at: string;
}

/** S3 storage configuration response (keys may be masked) */
export interface S3StorageConfigResponse {
  id: string;
  name: string;
  endpoint: string | null;
  bucket: string;
  region: string;
  access_key: string;
  secret_key: string;
  path_prefix: string | null;
  is_default: boolean;
  team_id: string | null;
  created_at: string;
  updated_at: string;
}

/** Request to create an S3 storage configuration */
export interface CreateS3StorageConfigRequest {
  name: string;
  endpoint?: string;
  bucket: string;
  region?: string;
  access_key: string;
  secret_key: string;
  path_prefix?: string;
  is_default?: boolean;
  team_id?: string;
}

/** Request to update an S3 storage configuration */
export interface UpdateS3StorageConfigRequest {
  name?: string;
  endpoint?: string;
  bucket?: string;
  region?: string;
  access_key?: string;
  secret_key?: string;
  path_prefix?: string;
  is_default?: boolean;
}

/** S3 backup type */
export type S3BackupType = "instance" | "database" | "volume";

/** S3 backup status */
export type S3BackupStatus = "pending" | "uploading" | "completed" | "failed";

/** S3 backup response */
export interface S3BackupResponse {
  id: string;
  storage_config_id: string;
  storage_config_name: string | null;
  backup_type: S3BackupType;
  source_id: string | null;
  s3_key: string;
  size_bytes: number | null;
  size_human: string | null;
  status: S3BackupStatus;
  error_message: string | null;
  team_id: string | null;
  created_at: string;
}

/** Request to trigger an S3 backup */
export interface TriggerS3BackupRequest {
  storage_config_id: string;
  backup_type: S3BackupType;
  source_id?: string;
}

/** S3 connection test result */
export interface S3TestConnectionResult {
  success: boolean;
  message: string;
}

// -------------------------------------------------------------------------
// Webhook Event Audit types
// -------------------------------------------------------------------------

/** A single webhook audit event */
export interface WebhookEvent {
  id: string;
  provider: string;
  event_type: string;
  repository: string | null;
  branch: string | null;
  commit_sha: string | null;
  payload_size: number | null;
  apps_triggered: number;
  status: "received" | "processed" | "ignored" | "error";
  error_message: string | null;
  received_at: string;
}

/** Paginated response for webhook events */
export interface WebhookEventListResponse {
  items: WebhookEvent[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}

// -------------------------------------------------------------------------
// Advanced Monitoring types
// -------------------------------------------------------------------------

/** Log search result */
export interface LogSearchResult {
  id: number;
  deployment_id: string;
  timestamp: string;
  level: string;
  message: string;
}

/** Log retention policy for an app */
export interface LogRetentionPolicy {
  id: string;
  app_id: string;
  retention_days: number;
  max_size_mb: number | null;
  created_at: string;
  updated_at: string;
}

/** Request to update log retention policy */
export interface UpdateLogRetentionRequest {
  retention_days?: number;
  max_size_mb?: number | null;
}

/** Result of system log cleanup */
export interface LogCleanupResult {
  apps_processed: number;
  logs_deleted: number;
}

/** Uptime check record */
export interface UptimeCheck {
  id: string;
  app_id: string;
  status: "up" | "down" | "degraded";
  response_time_ms: number | null;
  status_code: number | null;
  error_message: string | null;
  checked_at: string;
}

/** Uptime summary for an app */
export interface UptimeSummary {
  app_id: string;
  availability_percent: number;
  total_checks: number;
  up_checks: number;
  down_checks: number;
  degraded_checks: number;
  avg_response_time_ms: number | null;
  recent_checks: UptimeCheck[];
}

// -------------------------------------------------------------------------
// Cost Estimation types
// -------------------------------------------------------------------------

/** Cost summary with totals and projections */
export interface CostSummary {
  cpu_cost: number;
  memory_cost: number;
  disk_cost: number;
  total_cost: number;
  avg_cpu_cores: number;
  avg_memory_gb: number;
  avg_disk_gb: number;
  days_in_period: number;
  projected_monthly_cost: number;
}

/** Cost breakdown for a single app */
export interface AppCostBreakdown {
  app_id: string;
  app_name: string;
  cpu_cost: number;
  memory_cost: number;
  disk_cost: number;
  total_cost: number;
}

/** Daily cost data point for trend display */
export interface DailyCostPoint {
  date: string;
  total_cost: number;
}

/** Dashboard cost response with summary, top apps, and trend data */
export interface DashboardCostResponse {
  summary: CostSummary;
  top_apps: AppCostBreakdown[];
  trend: DailyCostPoint[];
  period: string;
  period_days: number;
}

/** Cost response for app/project/team endpoints */
export interface CostResponse {
  summary: CostSummary;
  breakdown?: AppCostBreakdown[];
  period: string;
  period_days: number;
}
