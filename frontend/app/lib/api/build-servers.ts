/**
 * Build Servers API module.
 * Handles dedicated remote build server registration and management.
 */

import { apiRequest } from "./core";

export interface BuildServer {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  status: "online" | "offline" | "unknown";
  last_seen_at?: string;
  docker_version?: string;
  cpu_count?: number;
  memory_bytes?: number;
  concurrent_builds: number;
  active_builds: number;
  team_id?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateBuildServerRequest {
  name: string;
  host: string;
  port?: number;
  username?: string;
  ssh_private_key?: string;
  ssh_password?: string;
  concurrent_builds?: number;
  team_id?: string;
}

export interface UpdateBuildServerRequest {
  name?: string;
  host?: string;
  port?: number;
  username?: string;
  ssh_private_key?: string;
  concurrent_builds?: number;
}

export const buildServersApi = {
  /** List all build servers, optionally filtered by team_id */
  list: (options: { teamId?: string } = {}, token?: string) => {
    const params = new URLSearchParams();
    if (options.teamId) params.append("team_id", options.teamId);
    const qs = params.toString();
    return apiRequest<BuildServer[]>(`/build-servers${qs ? `?${qs}` : ""}`, {}, token);
  },

  /** Create a new build server */
  create: (data: CreateBuildServerRequest, token?: string) =>
    apiRequest<BuildServer>("/build-servers", {
      method: "POST",
      body: JSON.stringify(data),
    }, token),

  /** Get a build server by ID */
  get: (id: string, token?: string) =>
    apiRequest<BuildServer>(`/build-servers/${id}`, {}, token),

  /** Update a build server */
  update: (id: string, data: UpdateBuildServerRequest, token?: string) =>
    apiRequest<BuildServer>(`/build-servers/${id}`, {
      method: "PUT",
      body: JSON.stringify(data),
    }, token),

  /** Delete a build server */
  delete: (id: string, token?: string) =>
    apiRequest<void>(`/build-servers/${id}`, { method: "DELETE" }, token),

  /** Trigger a health check on a build server */
  check: (id: string, token?: string) =>
    apiRequest<BuildServer>(`/build-servers/${id}/check`, { method: "POST" }, token),
};
