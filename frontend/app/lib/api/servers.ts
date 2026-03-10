/**
 * Servers API module.
 * Handles remote server registration and management for multi-server deployments.
 */

import { apiRequest } from "./core";

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
  team_id?: string;
}

export interface UpdateServerRequest {
  name?: string;
  host?: string;
  port?: number;
  username?: string;
  ssh_private_key?: string;
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
};
