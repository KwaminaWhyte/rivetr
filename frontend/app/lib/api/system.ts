/**
 * System API module.
 * Handles system stats, health, events, audit logs, costs, and settings.
 */

import { apiRequest, apiRawRequest, getStoredToken } from "./core";
import type {
  SystemStats,
  DiskStats,
  RecentEvent,
  SystemHealthStatus,
  AuditLogListResponse,
  AuditLogQuery,
  DashboardCostResponse,
  CostResponse,
  GlobalAlertDefaultsResponse,
  UpdateGlobalAlertDefaultsRequest,
  AlertStatsResponse,
  UpdateStatus,
  BackupInfo,
  RestoreResult,
  WebhookEvent,
} from "@/types/api";

/** A backup schedule record */
export interface BackupSchedule {
  id: string;
  backup_type: string;
  cron_expression: string;
  target_id: string | null;
  s3_config_id: string | null;
  retention_days: number;
  enabled: number;
  last_run_at: string | null;
  next_run_at: string | null;
  created_at: string;
}

/** Request to create a backup schedule */
export interface CreateBackupScheduleRequest {
  backup_type: string;
  cron_expression: string;
  target_id?: string | null;
  s3_config_id?: string | null;
  retention_days?: number;
}

/** Options for getting system stats */
export interface GetSystemStatsOptions {
  /** Team ID to filter stats by team scope */
  teamId?: string | null;
}

export const systemApi = {
  // -------------------------------------------------------------------------
  // System Stats & Health
  // -------------------------------------------------------------------------

  /** Get system-wide statistics, optionally scoped to a team */
  getSystemStats: (options: GetSystemStatsOptions = {}, token?: string) => {
    const params = new URLSearchParams();
    if (options.teamId) {
      params.append("team_id", options.teamId);
    }
    const queryString = params.toString();
    return apiRequest<SystemStats>(
      `/system/stats${queryString ? `?${queryString}` : ""}`,
      {},
      token
    );
  },

  /** Get disk usage statistics */
  getDiskStats: (token?: string) =>
    apiRequest<DiskStats>("/system/disk", {}, token),

  /** Get system health status */
  getSystemHealth: (token?: string) =>
    apiRequest<SystemHealthStatus>("/system/health", {}, token),

  // -------------------------------------------------------------------------
  // Events
  // -------------------------------------------------------------------------

  /** Get recent events for the dashboard */
  getRecentEvents: (token?: string) =>
    apiRequest<RecentEvent[]>("/events/recent", {}, token),

  // -------------------------------------------------------------------------
  // Audit Logs
  // -------------------------------------------------------------------------

  /** Get audit logs with optional filters */
  getAuditLogs: (query: AuditLogQuery = {}, token?: string) => {
    const params = new URLSearchParams();
    if (query.action) params.append("action", query.action);
    if (query.resource_type) params.append("resource_type", query.resource_type);
    if (query.resource_id) params.append("resource_id", query.resource_id);
    if (query.user_id) params.append("user_id", query.user_id);
    if (query.start_date) params.append("start_date", query.start_date);
    if (query.end_date) params.append("end_date", query.end_date);
    if (query.page) params.append("page", query.page.toString());
    if (query.per_page) params.append("per_page", query.per_page.toString());
    const queryString = params.toString();
    return apiRequest<AuditLogListResponse>(
      `/audit${queryString ? `?${queryString}` : ""}`,
      {},
      token
    );
  },

  /** Get available audit action types */
  getAuditActionTypes: (token?: string) =>
    apiRequest<string[]>("/audit/actions", {}, token),

  /** Get available audit resource types */
  getAuditResourceTypes: (token?: string) =>
    apiRequest<string[]>("/audit/resource-types", {}, token),

  // -------------------------------------------------------------------------
  // Costs
  // -------------------------------------------------------------------------

  /** Get dashboard cost summary with top apps and trend data */
  getDashboardCosts: (period: "7d" | "30d" | "90d" = "30d", token?: string) =>
    apiRequest<DashboardCostResponse>(`/system/costs?period=${period}`, {}, token),

  /** Get cost data for a specific team */
  getTeamCosts: (teamId: string, period: "7d" | "30d" | "90d" = "30d", token?: string) =>
    apiRequest<CostResponse>(`/teams/${teamId}/costs?period=${period}`, {}, token),

  /** Get cost data for a specific project */
  getProjectCosts: (projectId: string, period: "7d" | "30d" | "90d" = "30d", token?: string) =>
    apiRequest<CostResponse>(`/projects/${projectId}/costs?period=${period}`, {}, token),

  /** Get cost data for a specific app */
  getAppCosts: (appId: string, period: "7d" | "30d" | "90d" = "30d", token?: string) =>
    apiRequest<CostResponse>(`/apps/${appId}/costs?period=${period}`, {}, token),

  // -------------------------------------------------------------------------
  // Alert Defaults (Settings)
  // -------------------------------------------------------------------------

  /** Get global alert defaults */
  getAlertDefaults: (token?: string) =>
    apiRequest<GlobalAlertDefaultsResponse>("/settings/alert-defaults", {}, token),

  /** Update global alert defaults */
  updateAlertDefaults: (request: UpdateGlobalAlertDefaultsRequest, token?: string) =>
    apiRequest<GlobalAlertDefaultsResponse>("/settings/alert-defaults", {
      method: "PUT",
      body: JSON.stringify(request),
    }, token),

  /** Get alert configuration statistics */
  getAlertStats: (token?: string) =>
    apiRequest<AlertStatsResponse>("/settings/alert-stats", {}, token),

  // -------------------------------------------------------------------------
  // Auto-Update
  // -------------------------------------------------------------------------

  /** Get current version and update status */
  getVersionInfo: (token?: string) =>
    apiRequest<UpdateStatus>("/system/version", {}, token),

  /** Check for available updates */
  checkForUpdate: (token?: string) =>
    apiRequest<UpdateStatus>("/system/update/check", {
      method: "POST",
    }, token),

  /** Download the latest update */
  downloadUpdate: (token?: string) =>
    apiRequest<{ message: string; version: string }>("/system/update/download", {
      method: "POST",
    }, token),

  /** Apply a downloaded update (will restart the server) */
  applyUpdate: (token?: string) =>
    apiRequest<{ message: string }>("/system/update/apply", {
      method: "POST",
    }, token),

  // -------------------------------------------------------------------------
  // Instance Backup & Restore
  // -------------------------------------------------------------------------

  /** Create a backup and download it as a .tar.gz file */
  createBackup: async (token?: string): Promise<Blob> => {
    const response = await apiRawRequest("/system/backup", {
      method: "POST",
    }, token);
    return response.blob();
  },

  /** Create a full system backup (apps + env vars + databases + services + SQLite) */
  createFullBackup: async (teamId?: string, token?: string): Promise<Blob> => {
    const params = new URLSearchParams();
    if (teamId) params.append("team_id", teamId);
    const qs = params.toString();
    const response = await apiRawRequest(
      `/system/backup/full${qs ? `?${qs}` : ""}`,
      { method: "POST" },
      token
    );
    return response.blob();
  },

  /** List existing backups */
  listBackups: (token?: string) =>
    apiRequest<BackupInfo[]>("/system/backups", {}, token),

  /** Delete a specific backup */
  deleteBackup: (name: string, token?: string) =>
    apiRequest<{ message: string }>(`/system/backups/${encodeURIComponent(name)}`, {
      method: "DELETE",
    }, token),

  /** Download a specific backup file */
  downloadBackup: async (name: string, token?: string): Promise<Blob> => {
    const response = await apiRawRequest(
      `/system/backups/${encodeURIComponent(name)}/download`,
      {},
      token
    );
    return response.blob();
  },

  // -------------------------------------------------------------------------
  // Webhook Events
  // -------------------------------------------------------------------------

  /** List recent webhook audit events */
  listWebhookEvents: (
    params: { provider?: string; status?: string; limit?: number } = {},
    token?: string
  ) => {
    const query = new URLSearchParams();
    if (params.provider) query.append("provider", params.provider);
    if (params.status) query.append("status", params.status);
    if (params.limit) query.append("limit", params.limit.toString());
    const qs = query.toString();
    return apiRequest<WebhookEvent[]>(
      `/webhook-events${qs ? `?${qs}` : ""}`,
      {},
      token
    );
  },

  // -------------------------------------------------------------------------
  // Instance Settings
  // -------------------------------------------------------------------------

  /** Get instance settings (domain, name) */
  getInstanceSettings: (token?: string) =>
    apiRequest<{ instance_domain: string | null; instance_name: string | null }>(
      "/settings/instance",
      {},
      token
    ),

  /** Update instance settings */
  updateInstanceSettings: (
    req: { instance_domain?: string | null; instance_name?: string | null },
    token?: string
  ) =>
    apiRequest<{ instance_domain: string | null; instance_name: string | null }>(
      "/settings/instance",
      {
        method: "PUT",
        body: JSON.stringify(req),
      },
      token
    ),

  /** Restore from a backup file upload */
  restoreBackup: async (file: File, token?: string): Promise<RestoreResult> => {
    const formData = new FormData();
    formData.append("file", file);

    const authToken = token || getStoredToken();
    const headers: Record<string, string> = {};
    if (authToken) {
      headers["Authorization"] = `Bearer ${authToken}`;
    }

    const response = await fetch("/api/system/restore", {
      method: "POST",
      headers,
      body: formData,
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(error || `API error: ${response.status}`);
    }

    return response.json();
  },

  // -------------------------------------------------------------------------
  // Backup Schedules
  // -------------------------------------------------------------------------

  /** List all backup schedules */
  listBackupSchedules: (token?: string) =>
    apiRequest<BackupSchedule[]>("/backups/schedules", {}, token),

  /** Create a new backup schedule */
  createBackupSchedule: (
    req: CreateBackupScheduleRequest,
    token?: string
  ) =>
    apiRequest<BackupSchedule>("/backups/schedules", {
      method: "POST",
      body: JSON.stringify(req),
    }, token),

  /** Delete a backup schedule */
  deleteBackupSchedule: (id: string, token?: string) =>
    apiRequest<void>(`/backups/schedules/${id}`, { method: "DELETE" }, token),

  /** Toggle a backup schedule on/off */
  toggleBackupSchedule: (id: string, token?: string) =>
    apiRequest<BackupSchedule>(`/backups/schedules/${id}/toggle`, { method: "PUT" }, token),

  /** Manually trigger a backup schedule to run now */
  runBackupSchedule: (id: string, token?: string) =>
    apiRequest<{ message: string; last_run_at: string; next_run_at: string | null }>(
      `/backups/schedules/${id}/run`,
      { method: "POST" },
      token
    ),
};
