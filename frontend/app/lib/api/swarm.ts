/**
 * Docker Swarm API module.
 * Handles swarm initialization, node management, and service management.
 */

import { apiRequest } from "./core";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface SwarmNode {
  id: string;
  node_id: string;
  hostname: string;
  role: "manager" | "worker";
  status: "ready" | "down" | "disconnected" | "unknown";
  availability: "active" | "pause" | "drain";
  cpu_count?: number;
  memory_bytes?: number;
  docker_version?: string;
  ip_address?: string;
  last_seen_at?: string;
  created_at: string;
}

export interface SwarmService {
  id: string;
  app_id?: string;
  service_name: string;
  service_id?: string;
  replicas: number;
  mode: "replicated" | "global";
  image: string;
  status: "pending" | "running" | "failed" | "stopped";
  created_at: string;
  updated_at: string;
}

export interface SwarmInitResponse {
  node_id: string;
  manager_token: string;
  worker_token: string;
}

export interface SwarmStatusResponse {
  node_id?: string;
  is_manager: boolean;
  node_count: number;
  managers: number;
  workers: number;
  local_node_state: string;
}

export interface CreateServiceRequest {
  app_id?: string;
  service_name: string;
  image: string;
  replicas?: number;
  mode?: "replicated" | "global";
}

// ---------------------------------------------------------------------------
// API methods
// ---------------------------------------------------------------------------

export const swarmApi = {
  /** Initialize Docker Swarm on the current node */
  init: (token?: string) =>
    apiRequest<SwarmInitResponse>("/swarm/init", { method: "POST" }, token),

  /** Get current swarm status */
  getStatus: (token?: string) =>
    apiRequest<SwarmStatusResponse>("/swarm/status", {}, token),

  /** Leave the swarm (force) */
  leave: (token?: string) =>
    apiRequest<void>("/swarm/leave", { method: "POST" }, token),

  /** List all swarm nodes (from DB) */
  listNodes: (token?: string) =>
    apiRequest<SwarmNode[]>("/swarm/nodes", {}, token),

  /** Sync node list from docker node ls */
  syncNodes: (token?: string) =>
    apiRequest<SwarmNode[]>("/swarm/sync-nodes", { method: "POST" }, token),

  /** Update node availability (drain / activate / pause) */
  updateNodeAvailability: (
    id: string,
    availability: "active" | "pause" | "drain",
    token?: string
  ) =>
    apiRequest<SwarmNode>(
      `/swarm/nodes/${id}/availability`,
      {
        method: "PUT",
        body: JSON.stringify({ availability }),
      },
      token
    ),

  /** List all swarm services */
  listServices: (token?: string) =>
    apiRequest<SwarmService[]>("/swarm/services", {}, token),

  /** Create a swarm service */
  createService: (data: CreateServiceRequest, token?: string) =>
    apiRequest<SwarmService>("/swarm/services", {
      method: "POST",
      body: JSON.stringify(data),
    }, token),

  /** Remove a swarm service */
  deleteService: (id: string, token?: string) =>
    apiRequest<void>(`/swarm/services/${id}`, { method: "DELETE" }, token),

  /** Scale a swarm service */
  scaleService: (id: string, replicas: number, token?: string) =>
    apiRequest<SwarmService>(
      `/swarm/services/${id}/scale`,
      {
        method: "POST",
        body: JSON.stringify({ replicas }),
      },
      token
    ),

  /** Get logs for a swarm service */
  getServiceLogs: (id: string, token?: string) =>
    apiRequest<{ logs: string[] }>(`/swarm/services/${id}/logs`, {}, token),
};
