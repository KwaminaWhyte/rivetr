// Client-side API utility for browser-only fetching
// Uses the same API proxy configured in vite.config.ts

import type {
  App,
  ContainerStats,
  CreateEnvVarRequest,
  Deployment,
  DeploymentLog,
  EnvVar,
  Project,
  ProjectWithApps,
  RecentEvent,
  SshKey,
  SystemStats,
  UpdateAppRequest,
  UpdateEnvVarRequest,
} from "@/types/api";

async function apiRequest<T>(
  path: string,
  options: RequestInit = {}
): Promise<T> {
  const response = await fetch(`/api${path}`, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...options.headers,
    },
    credentials: "include", // Send cookies for session-based auth
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(error || `API error: ${response.status}`);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

export const api = {
  // Projects
  getProjects: () => apiRequest<Project[]>("/projects"),
  getProject: (id: string) => apiRequest<ProjectWithApps>(`/projects/${id}`),

  // Apps
  getApps: () => apiRequest<App[]>("/apps"),
  getApp: (id: string) => apiRequest<App>(`/apps/${id}`),
  updateApp: (id: string, data: UpdateAppRequest) =>
    apiRequest<App>(`/apps/${id}`, {
      method: "PUT",
      body: JSON.stringify(data),
    }),

  // SSH Keys
  getSshKeys: () => apiRequest<SshKey[]>("/ssh-keys"),

  // Deployments
  getDeployments: (appId: string) =>
    apiRequest<Deployment[]>(`/apps/${appId}/deployments`),
  getDeploymentLogs: (id: string) =>
    apiRequest<DeploymentLog[]>(`/deployments/${id}/logs`),
  triggerDeploy: (appId: string) =>
    apiRequest<Deployment>(`/apps/${appId}/deploy`, { method: "POST" }),
  rollbackDeployment: (id: string) =>
    apiRequest<Deployment>(`/deployments/${id}/rollback`, { method: "POST" }),

  // Container Stats
  getAppStats: (appId: string) =>
    apiRequest<ContainerStats>(`/apps/${appId}/stats`),

  // Environment Variables
  getEnvVars: (appId: string, reveal = false) => {
    const params = reveal ? "?reveal=true" : "";
    return apiRequest<EnvVar[]>(`/apps/${appId}/env-vars${params}`);
  },
  getEnvVar: (appId: string, key: string, reveal = false) => {
    const params = reveal ? "?reveal=true" : "";
    return apiRequest<EnvVar>(`/apps/${appId}/env-vars/${encodeURIComponent(key)}${params}`);
  },
  createEnvVar: (appId: string, data: CreateEnvVarRequest) =>
    apiRequest<EnvVar>(`/apps/${appId}/env-vars`, {
      method: "POST",
      body: JSON.stringify(data),
    }),
  updateEnvVar: (appId: string, key: string, data: UpdateEnvVarRequest) =>
    apiRequest<EnvVar>(`/apps/${appId}/env-vars/${encodeURIComponent(key)}`, {
      method: "PUT",
      body: JSON.stringify(data),
    }),
  deleteEnvVar: (appId: string, key: string) =>
    apiRequest<void>(`/apps/${appId}/env-vars/${encodeURIComponent(key)}`, {
      method: "DELETE",
    }),

  // System
  getSystemStats: () => apiRequest<SystemStats>("/system/stats"),
  getRecentEvents: () => apiRequest<RecentEvent[]>("/events/recent"),

  // WebSocket URLs
  getRuntimeLogsWsUrl: (appId: string): string => {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    return `${protocol}//${window.location.host}/api/apps/${appId}/logs/stream`;
  },
};

export default api;
