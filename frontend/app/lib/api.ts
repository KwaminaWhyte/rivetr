// Client-side API utility for browser-only fetching
// Uses the same API proxy configured in vite.config.ts

import type {
  App,
  AppStatus,
  BasicAuthStatus,
  ContainerStats,
  CreateEnvVarRequest,
  Deployment,
  DeploymentLog,
  DiskStats,
  EnvVar,
  Project,
  ProjectWithApps,
  RecentEvent,
  SshKey,
  SystemStats,
  UpdateAppRequest,
  UpdateBasicAuthRequest,
  UpdateEnvVarRequest,
} from "@/types/api";

async function apiRequest<T>(
  path: string,
  options: RequestInit = {},
  token?: string
): Promise<T> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(options.headers as Record<string, string>),
  };

  // Add Authorization header if token is provided
  if (token) {
    headers["Authorization"] = `Bearer ${token}`;
  }

  const response = await fetch(`/api${path}`, {
    ...options,
    headers,
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
  getApps: (token?: string) => apiRequest<App[]>("/apps", {}, token),
  getApp: (id: string, token?: string) => apiRequest<App>(`/apps/${id}`, {}, token),
  updateApp: (id: string, data: UpdateAppRequest, token?: string) =>
    apiRequest<App>(
      `/apps/${id}`,
      {
        method: "PUT",
        body: JSON.stringify(data),
      },
      token
    ),
  getAppStatus: (id: string, token?: string) =>
    apiRequest<AppStatus>(`/apps/${id}/status`, {}, token),
  startApp: (id: string, token?: string) =>
    apiRequest<AppStatus>(`/apps/${id}/start`, { method: "POST" }, token),
  stopApp: (id: string, token?: string) =>
    apiRequest<AppStatus>(`/apps/${id}/stop`, { method: "POST" }, token),

  // SSH Keys
  getSshKeys: () => apiRequest<SshKey[]>("/ssh-keys"),

  // Deployments
  getDeployments: (appId: string, token?: string) =>
    apiRequest<Deployment[]>(`/apps/${appId}/deployments`, {}, token),
  getDeploymentLogs: (id: string, token?: string) =>
    apiRequest<DeploymentLog[]>(`/deployments/${id}/logs`, {}, token),
  triggerDeploy: (appId: string, token?: string) =>
    apiRequest<Deployment>(`/apps/${appId}/deploy`, { method: "POST" }, token),
  rollbackDeployment: (id: string, token?: string) =>
    apiRequest<Deployment>(`/deployments/${id}/rollback`, { method: "POST" }, token),

  // Container Stats
  getAppStats: (appId: string, token?: string) =>
    apiRequest<ContainerStats>(`/apps/${appId}/stats`, {}, token),

  // Environment Variables
  getEnvVars: (appId: string, reveal = false, token?: string) => {
    const params = reveal ? "?reveal=true" : "";
    return apiRequest<EnvVar[]>(`/apps/${appId}/env-vars${params}`, {}, token);
  },
  getEnvVar: (appId: string, key: string, reveal = false, token?: string) => {
    const params = reveal ? "?reveal=true" : "";
    return apiRequest<EnvVar>(
      `/apps/${appId}/env-vars/${encodeURIComponent(key)}${params}`,
      {},
      token
    );
  },
  createEnvVar: (appId: string, data: CreateEnvVarRequest, token?: string) =>
    apiRequest<EnvVar>(
      `/apps/${appId}/env-vars`,
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),
  updateEnvVar: (appId: string, key: string, data: UpdateEnvVarRequest, token?: string) =>
    apiRequest<EnvVar>(
      `/apps/${appId}/env-vars/${encodeURIComponent(key)}`,
      {
        method: "PUT",
        body: JSON.stringify(data),
      },
      token
    ),
  deleteEnvVar: (appId: string, key: string, token?: string) =>
    apiRequest<void>(
      `/apps/${appId}/env-vars/${encodeURIComponent(key)}`,
      {
        method: "DELETE",
      },
      token
    ),

  // HTTP Basic Auth
  getBasicAuth: (appId: string, token?: string) =>
    apiRequest<BasicAuthStatus>(`/apps/${appId}/basic-auth`, {}, token),
  updateBasicAuth: (appId: string, data: UpdateBasicAuthRequest, token?: string) =>
    apiRequest<BasicAuthStatus>(
      `/apps/${appId}/basic-auth`,
      {
        method: "PUT",
        body: JSON.stringify(data),
      },
      token
    ),
  deleteBasicAuth: (appId: string, token?: string) =>
    apiRequest<void>(
      `/apps/${appId}/basic-auth`,
      {
        method: "DELETE",
      },
      token
    ),

  // System
  getSystemStats: (token?: string) => apiRequest<SystemStats>("/system/stats", {}, token),
  getDiskStats: (token?: string) => apiRequest<DiskStats>("/system/disk", {}, token),
  getRecentEvents: (token?: string) => apiRequest<RecentEvent[]>("/events/recent", {}, token),

  // WebSocket URLs
  getRuntimeLogsWsUrl: (appId: string, token: string): string => {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    return `${protocol}//${window.location.host}/api/apps/${appId}/logs/stream?token=${encodeURIComponent(token)}`;
  },
  getTerminalWsUrl: (appId: string, token: string): string => {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    return `${protocol}//${window.location.host}/api/apps/${appId}/terminal?token=${encodeURIComponent(token)}`;
  },
};

export default api;
