/**
 * Servers API module.
 * Handles remote server registration and management for multi-server deployments.
 */

import { apiRequest, getStoredToken } from "./core";

export interface Server {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  status: "online" | "offline" | "unknown";
  last_seen_at?: string;
  cpu_usage?: number;
  memory_usage?: number;
  disk_usage?: number;
  memory_total?: number;
  disk_total?: number;
  os_info?: string;
  docker_version?: string;
  team_id?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateServerRequest {
  name: string;
  host: string;
  port?: number;
  username?: string;
  ssh_private_key?: string;
  ssh_password?: string;
  team_id?: string;
}

export interface UpdateServerRequest {
  name?: string;
  host?: string;
  port?: number;
  username?: string;
  ssh_private_key?: string;
}

export interface PatchesResponse {
  security_updates: number;
  total_updates: number;
  packages: string[];
  checked_at: string;
}

export interface SecurityCheckItem {
  id: string;
  name: string;
  description: string;
  /** "pass" | "fail" | "warn" | "unknown" */
  status: string;
  details?: string;
}

export interface SecurityCheckResponse {
  items: SecurityCheckItem[];
  checked_at: string;
}

export const serversApi = {
  /** List all servers, optionally filtered by team_id */
  list: (options: { teamId?: string } = {}, token?: string) => {
    const params = new URLSearchParams();
    if (options.teamId) params.append("team_id", options.teamId);
    const qs = params.toString();
    return apiRequest<Server[]>(`/servers${qs ? `?${qs}` : ""}`, {}, token);
  },

  /** Create a new server */
  create: (data: CreateServerRequest, token?: string) =>
    apiRequest<Server>("/servers", {
      method: "POST",
      body: JSON.stringify(data),
    }, token),

  /** Get a server by ID */
  get: (id: string, token?: string) =>
    apiRequest<Server>(`/servers/${id}`, {}, token),

  /** Update a server */
  update: (id: string, data: UpdateServerRequest, token?: string) =>
    apiRequest<Server>(`/servers/${id}`, {
      method: "PUT",
      body: JSON.stringify(data),
    }, token),

  /** Delete a server */
  delete: (id: string, token?: string) =>
    apiRequest<void>(`/servers/${id}`, { method: "DELETE" }, token),

  /** Trigger a health check on a server */
  check: (id: string, token?: string) =>
    apiRequest<Server>(`/servers/${id}/check`, { method: "POST" }, token),

  /** List apps assigned to this server */
  listApps: (id: string, token?: string) =>
    apiRequest<{ id: string; name: string; server_id?: string }[]>(
      `/servers/${id}/apps`,
      {},
      token
    ),

  /** Assign an app to a server */
  assignApp: (serverId: string, appId: string, token?: string) =>
    apiRequest<void>(
      `/servers/${serverId}/apps/${appId}`,
      { method: "POST" },
      token
    ),

  /** Unassign an app from a server */
  unassignApp: (serverId: string, appId: string, token?: string) =>
    apiRequest<void>(
      `/servers/${serverId}/apps/${appId}`,
      { method: "DELETE" },
      token
    ),

  /** Check for pending OS/security updates on a server */
  checkPatches: (id: string, token?: string) =>
    apiRequest<PatchesResponse>(`/servers/${id}/patches`, {}, token),

  /** Run a security checklist against a server */
  checkSecurity: (id: string, token?: string) =>
    apiRequest<SecurityCheckResponse>(`/servers/${id}/security-check`, {}, token),

  /** Get the WebSocket URL for an SSH terminal session on a server */
  getTerminalWsUrl: (serverId: string, token?: string): string => {
    const authToken = token || getStoredToken() || "";
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    return `${protocol}//${window.location.host}/api/servers/${serverId}/terminal?token=${encodeURIComponent(authToken)}`;
  },
};
