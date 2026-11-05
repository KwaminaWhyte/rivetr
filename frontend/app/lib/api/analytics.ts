/**
 * Traffic analytics API (E3): rollups of proxied requests.
 */

import { apiRequest } from "./core";

export type AnalyticsRange = "1h" | "24h" | "7d" | "30d";

export interface AnalyticsTotals {
  requests: number;
  status_2xx: number;
  status_3xx: number;
  status_4xx: number;
  status_5xx: number;
  bytes_in: number;
  bytes_out: number;
  p50_ms: number | null;
  p95_ms: number | null;
  /** Sum of daily unique visitors over the days the range touches. */
  visitors: number;
}

export interface AnalyticsPoint {
  ts: number;
  requests: number;
  status_2xx: number;
  status_3xx: number;
  status_4xx: number;
  status_5xx: number;
  bytes_out: number;
  p50_ms: number | null;
  p95_ms: number | null;
}

export interface Ranked {
  value: string;
  count: number;
}

export interface AnalyticsHost {
  host: string;
  app_id: string | null;
  app_name: string | null;
  requests: number;
}

export interface AnalyticsReport {
  range: AnalyticsRange;
  step_secs: number;
  enabled: boolean;
  geoip: boolean;
  totals: AnalyticsTotals;
  series: AnalyticsPoint[];
  top_paths: Ranked[];
  top_referrers: Ranked[];
  top_user_agents: Ranked[];
  countries: Ranked[] | null;
  top_hosts: AnalyticsHost[];
}

export interface AnalyticsServerToggle {
  server_id: string;
  name: string;
  enabled: boolean;
}

export interface AnalyticsSettings {
  retention_days: number;
  geoip_path: string | null;
  geoip_error: string | null;
  servers: AnalyticsServerToggle[];
}

export const analyticsApi = {
  app: (appId: string, range: AnalyticsRange) =>
    apiRequest<AnalyticsReport>(`/apps/${appId}/analytics?range=${range}`),

  server: (serverId: string, range: AnalyticsRange) =>
    apiRequest<AnalyticsReport>(`/servers/${serverId}/analytics?range=${range}`),

  team: (range: AnalyticsRange, teamId?: string | null) =>
    apiRequest<AnalyticsReport>(
      `/analytics?range=${range}${teamId ? `&team_id=${encodeURIComponent(teamId)}` : ""}`
    ),

  setServerEnabled: (serverId: string, enabled: boolean) =>
    apiRequest<{ server_id: string; enabled: boolean }>(`/servers/${serverId}/analytics`, {
      method: "PUT",
      body: JSON.stringify({ enabled }),
    }),

  settings: () => apiRequest<AnalyticsSettings>("/analytics/settings"),

  updateSettings: (body: { retention_days?: number; geoip_path?: string }) =>
    apiRequest<AnalyticsSettings>("/analytics/settings", {
      method: "PUT",
      body: JSON.stringify(body),
    }),
};
