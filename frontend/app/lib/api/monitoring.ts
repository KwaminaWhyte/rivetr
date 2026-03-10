/**
 * Monitoring API module.
 * Handles log search, log retention, uptime tracking, and scheduled restarts.
 */

import { apiRequest } from "./core";
import type {
  LogSearchResult,
  LogRetentionPolicy,
  UpdateLogRetentionRequest,
  LogCleanupResult,
  UptimeSummary,
  UptimeCheck,
  ScheduledRestart,
  CreateScheduledRestartRequest,
  UpdateScheduledRestartRequest,
} from "@/types/api";

export const monitoringApi = {
  // -- Log Search --

  /** Search deployment logs for an app */
  searchLogs: (
    appId: string,
    params?: { q?: string; from?: string; to?: string; level?: string; limit?: number },
    token?: string
  ) => {
    const query = new URLSearchParams();
    if (params?.q) query.append("q", params.q);
    if (params?.from) query.append("from", params.from);
    if (params?.to) query.append("to", params.to);
    if (params?.level) query.append("level", params.level);
    if (params?.limit) query.append("limit", params.limit.toString());
    const qs = query.toString();
    return apiRequest<LogSearchResult[]>(
      `/apps/${appId}/logs/search${qs ? `?${qs}` : ""}`,
      {},
      token
    );
  },

  // -- Log Retention --

  /** Get log retention policy for an app */
  getLogRetention: (appId: string, token?: string) =>
    apiRequest<LogRetentionPolicy>(`/apps/${appId}/log-retention`, {}, token),

  /** Update log retention policy for an app */
  updateLogRetention: (appId: string, data: UpdateLogRetentionRequest, token?: string) =>
    apiRequest<LogRetentionPolicy>(`/apps/${appId}/log-retention`, {
      method: "PUT",
      body: JSON.stringify(data),
    }, token),

  /** Trigger system-wide log cleanup */
  triggerLogCleanup: (token?: string) =>
    apiRequest<LogCleanupResult>("/system/log-cleanup", {
      method: "POST",
    }, token),

  // -- Uptime --

  /** Get uptime summary for an app */
  getUptime: (appId: string, token?: string) =>
    apiRequest<UptimeSummary>(`/apps/${appId}/uptime`, {}, token),

  /** Get uptime history for an app */
  getUptimeHistory: (appId: string, period?: "24h" | "7d" | "30d", token?: string) => {
    const query = period ? `?period=${period}` : "";
    return apiRequest<UptimeCheck[]>(
      `/apps/${appId}/uptime/history${query}`,
      {},
      token
    );
  },

  // -- Scheduled Restarts --

  /** List scheduled restarts for an app */
  getScheduledRestarts: (appId: string, token?: string) =>
    apiRequest<ScheduledRestart[]>(`/apps/${appId}/scheduled-restarts`, {}, token),

  /** Create a scheduled restart */
  createScheduledRestart: (appId: string, data: CreateScheduledRestartRequest, token?: string) =>
    apiRequest<ScheduledRestart>(`/apps/${appId}/scheduled-restarts`, {
      method: "POST",
      body: JSON.stringify(data),
    }, token),

  /** Update a scheduled restart */
  updateScheduledRestart: (
    appId: string,
    restartId: string,
    data: UpdateScheduledRestartRequest,
    token?: string
  ) =>
    apiRequest<ScheduledRestart>(`/apps/${appId}/scheduled-restarts/${restartId}`, {
      method: "PUT",
      body: JSON.stringify(data),
    }, token),

  /** Delete a scheduled restart */
  deleteScheduledRestart: (appId: string, restartId: string, token?: string) =>
    apiRequest<void>(`/apps/${appId}/scheduled-restarts/${restartId}`, {
      method: "DELETE",
    }, token),
};
