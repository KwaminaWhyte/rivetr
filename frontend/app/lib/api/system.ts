/**
 * System API module.
 * Handles system stats, health, events, audit logs, and costs.
 */

import { apiRequest } from "./core";
import type {
  SystemStats,
  DiskStats,
  RecentEvent,
  SystemHealthStatus,
  AuditLogListResponse,
  AuditLogQuery,
  DashboardCostResponse,
} from "@/types/api";

export const systemApi = {
  // -------------------------------------------------------------------------
  // System Stats & Health
  // -------------------------------------------------------------------------

  /** Get system-wide statistics */
  getSystemStats: (token?: string) =>
    apiRequest<SystemStats>("/system/stats", {}, token),

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
};
