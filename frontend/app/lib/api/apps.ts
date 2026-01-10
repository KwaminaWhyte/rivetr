/**
 * Apps API module.
 * Handles application CRUD, deployment, and management operations.
 */

import { apiRequest } from "./core";
import type {
  App,
  AppStatus,
  AppShare,
  AppWithSharing,
  CreateAppRequest,
  CreateAppShareRequest,
  UpdateAppRequest,
  Deployment,
  DeploymentListResponse,
  DeploymentQuery,
  DeploymentLog,
  ContainerStats,
  EnvVar,
  CreateEnvVarRequest,
  UpdateEnvVarRequest,
  BasicAuthStatus,
  UpdateBasicAuthRequest,
  Volume,
  CreateVolumeRequest,
  UpdateVolumeRequest,
  BuildDetectionResult,
  UploadDeployResponse,
  AlertConfigResponse,
  CreateAlertConfigRequest,
  UpdateAlertConfigRequest,
  AlertEventResponse,
  UploadAppResponse,
} from "@/types/api";
import { getStoredToken } from "./core";

export const appsApi = {
  // -------------------------------------------------------------------------
  // App CRUD
  // -------------------------------------------------------------------------

  /** List all apps, optionally filtered by team */
  getApps: (options?: { teamId?: string }, token?: string) => {
    const params = new URLSearchParams();
    if (options?.teamId) {
      params.append("team_id", options.teamId);
    }
    const queryString = params.toString();
    const url = queryString ? `/apps?${queryString}` : "/apps";
    return apiRequest<App[]>(url, { teamId: options?.teamId }, token);
  },

  /** Get a single app by ID */
  getApp: (id: string, token?: string) =>
    apiRequest<App>(`/apps/${id}`, {}, token),

  /** Create a new app */
  createApp: (data: CreateAppRequest, token?: string) =>
    apiRequest<App>(
      "/apps",
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token,
    ),

  /** Update an existing app */
  updateApp: (id: string, data: UpdateAppRequest, token?: string) =>
    apiRequest<App>(
      `/apps/${id}`,
      {
        method: "PUT",
        body: JSON.stringify(data),
      },
      token,
    ),

  /** Delete an app */
  deleteApp: (id: string, password: string, token?: string) =>
    apiRequest<void>(
      `/apps/${id}`,
      {
        method: "DELETE",
        body: JSON.stringify({ password }),
      },
      token,
    ),

  /** Assign an app to a project */
  assignAppToProject: (
    appId: string,
    projectId: string | null,
    token?: string,
  ) =>
    apiRequest<App>(
      `/apps/${appId}`,
      {
        method: "PUT",
        body: JSON.stringify({ project_id: projectId }),
      },
      token,
    ),

  // -------------------------------------------------------------------------
  // App Status & Control
  // -------------------------------------------------------------------------

  /** Get the current status of an app */
  getAppStatus: (id: string, token?: string) =>
    apiRequest<AppStatus>(`/apps/${id}/status`, {}, token),

  /** Start an app */
  startApp: (id: string, token?: string) =>
    apiRequest<AppStatus>(`/apps/${id}/start`, { method: "POST" }, token),

  /** Stop an app */
  stopApp: (id: string, token?: string) =>
    apiRequest<AppStatus>(`/apps/${id}/stop`, { method: "POST" }, token),

  /** Restart an app */
  restartApp: (id: string, token?: string) =>
    apiRequest<AppStatus>(`/apps/${id}/restart`, { method: "POST" }, token),

  // -------------------------------------------------------------------------
  // Deployments
  // -------------------------------------------------------------------------

  /** Get all deployments for an app with pagination */
  getDeployments: (
    appId: string,
    query: DeploymentQuery = {},
    token?: string,
  ) => {
    const params = new URLSearchParams();
    if (query.page) params.append("page", String(query.page));
    if (query.per_page) params.append("per_page", String(query.per_page));
    const queryString = params.toString();
    const url = queryString
      ? `/apps/${appId}/deployments?${queryString}`
      : `/apps/${appId}/deployments`;
    return apiRequest<DeploymentListResponse>(url, {}, token);
  },

  /** Get logs for a specific deployment */
  getDeploymentLogs: (id: string, token?: string) =>
    apiRequest<DeploymentLog[]>(`/deployments/${id}/logs`, {}, token),

  /** Trigger a new deployment */
  triggerDeploy: (appId: string, token?: string) =>
    apiRequest<Deployment>(`/apps/${appId}/deploy`, { method: "POST" }, token),

  /** Rollback to a previous deployment */
  rollbackDeployment: (id: string, token?: string) =>
    apiRequest<Deployment>(
      `/deployments/${id}/rollback`,
      { method: "POST" },
      token,
    ),

  // -------------------------------------------------------------------------
  // Upload Deployments (ZIP file upload)
  // -------------------------------------------------------------------------

  /**
   * Deploy an app from an uploaded ZIP file.
   * Auto-detects build type and triggers deployment.
   */
  uploadDeploy: async (
    appId: string,
    file: File,
    token?: string,
  ): Promise<UploadDeployResponse> => {
    const authToken = token || getStoredToken();
    const formData = new FormData();
    formData.append("file", file);

    const headers: Record<string, string> = {};
    if (authToken) {
      headers["Authorization"] = `Bearer ${authToken}`;
    }

    const response = await fetch(`/api/apps/${appId}/deploy/upload`, {
      method: "POST",
      headers,
      body: formData,
      credentials: "include",
    });

    if (!response.ok) {
      const errorText = await response.text();
      let errorMessage: string;
      try {
        const errorJson = JSON.parse(errorText);
        errorMessage = errorJson.error || errorJson.message || errorText;
      } catch {
        errorMessage = errorText || `HTTP ${response.status}`;
      }
      throw new Error(errorMessage);
    }

    return response.json();
  },

  /**
   * Detect build type from an uploaded ZIP file without deploying.
   * Useful for previewing detection results before deployment.
   */
  detectBuildType: async (
    file: File,
    token?: string,
  ): Promise<BuildDetectionResult> => {
    const authToken = token || getStoredToken();
    const formData = new FormData();
    formData.append("file", file);

    const headers: Record<string, string> = {};
    if (authToken) {
      headers["Authorization"] = `Bearer ${authToken}`;
    }

    const response = await fetch("/api/build/detect", {
      method: "POST",
      headers,
      body: formData,
      credentials: "include",
    });

    if (!response.ok) {
      const errorText = await response.text();
      let errorMessage: string;
      try {
        const errorJson = JSON.parse(errorText);
        errorMessage = errorJson.error || errorJson.message || errorText;
      } catch {
        errorMessage = errorText || `HTTP ${response.status}`;
      }
      throw new Error(errorMessage);
    }

    return response.json();
  },

  /**
   * Create an app and deploy from uploaded ZIP file in one step.
   * This is the preferred way to deploy from a ZIP file.
   */
  uploadCreateApp: async (
    projectId: string,
    file: File,
    config: {
      name: string;
      port?: number;
      domain?: string;
      healthcheck?: string;
      cpu_limit?: string;
      memory_limit?: string;
      environment?: string;
      build_type?: string;
      publish_directory?: string;
    },
    token?: string,
  ): Promise<UploadAppResponse> => {
    const authToken = token || getStoredToken();
    const formData = new FormData();
    formData.append("file", file);
    formData.append("config", JSON.stringify(config));

    const headers: Record<string, string> = {};
    if (authToken) {
      headers["Authorization"] = `Bearer ${authToken}`;
    }

    const response = await fetch(`/api/projects/${projectId}/apps/upload`, {
      method: "POST",
      headers,
      body: formData,
      credentials: "include",
    });

    if (!response.ok) {
      const errorText = await response.text();
      let errorMessage: string;
      try {
        const errorJson = JSON.parse(errorText);
        errorMessage = errorJson.error || errorJson.message || errorText;
      } catch {
        errorMessage = errorText || `HTTP ${response.status}`;
      }
      throw new Error(errorMessage);
    }

    return response.json();
  },

  // -------------------------------------------------------------------------
  // Container Stats
  // -------------------------------------------------------------------------

  /** Get resource statistics for an app's container */
  getAppStats: (appId: string, token?: string) =>
    apiRequest<ContainerStats>(`/apps/${appId}/stats`, {}, token),

  // -------------------------------------------------------------------------
  // Environment Variables
  // -------------------------------------------------------------------------

  /** Get all environment variables for an app */
  getEnvVars: (appId: string, reveal = false, token?: string) => {
    const params = reveal ? "?reveal=true" : "";
    return apiRequest<EnvVar[]>(`/apps/${appId}/env-vars${params}`, {}, token);
  },

  /** Get a single environment variable */
  getEnvVar: (appId: string, key: string, reveal = false, token?: string) => {
    const params = reveal ? "?reveal=true" : "";
    return apiRequest<EnvVar>(
      `/apps/${appId}/env-vars/${encodeURIComponent(key)}${params}`,
      {},
      token,
    );
  },

  /** Create a new environment variable */
  createEnvVar: (appId: string, data: CreateEnvVarRequest, token?: string) =>
    apiRequest<EnvVar>(
      `/apps/${appId}/env-vars`,
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token,
    ),

  /** Update an existing environment variable */
  updateEnvVar: (
    appId: string,
    key: string,
    data: UpdateEnvVarRequest,
    token?: string,
  ) =>
    apiRequest<EnvVar>(
      `/apps/${appId}/env-vars/${encodeURIComponent(key)}`,
      {
        method: "PUT",
        body: JSON.stringify(data),
      },
      token,
    ),

  /** Delete an environment variable */
  deleteEnvVar: (appId: string, key: string, token?: string) =>
    apiRequest<void>(
      `/apps/${appId}/env-vars/${encodeURIComponent(key)}`,
      {
        method: "DELETE",
      },
      token,
    ),

  // -------------------------------------------------------------------------
  // HTTP Basic Auth
  // -------------------------------------------------------------------------

  /** Get basic auth status for an app */
  getBasicAuth: (appId: string, token?: string) =>
    apiRequest<BasicAuthStatus>(`/apps/${appId}/basic-auth`, {}, token),

  /** Update basic auth settings */
  updateBasicAuth: (
    appId: string,
    data: UpdateBasicAuthRequest,
    token?: string,
  ) =>
    apiRequest<BasicAuthStatus>(
      `/apps/${appId}/basic-auth`,
      {
        method: "PUT",
        body: JSON.stringify(data),
      },
      token,
    ),

  /** Disable basic auth */
  deleteBasicAuth: (appId: string, token?: string) =>
    apiRequest<void>(
      `/apps/${appId}/basic-auth`,
      {
        method: "DELETE",
      },
      token,
    ),

  // -------------------------------------------------------------------------
  // Volumes
  // -------------------------------------------------------------------------

  /** Get all volumes for an app */
  getVolumes: (appId: string, token?: string) =>
    apiRequest<Volume[]>(`/apps/${appId}/volumes`, {}, token),

  /** Get a single volume */
  getVolume: (volumeId: string, token?: string) =>
    apiRequest<Volume>(`/volumes/${volumeId}`, {}, token),

  /** Create a new volume */
  createVolume: (appId: string, data: CreateVolumeRequest, token?: string) =>
    apiRequest<Volume>(
      `/apps/${appId}/volumes`,
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token,
    ),

  /** Update an existing volume */
  updateVolume: (volumeId: string, data: UpdateVolumeRequest, token?: string) =>
    apiRequest<Volume>(
      `/volumes/${volumeId}`,
      {
        method: "PUT",
        body: JSON.stringify(data),
      },
      token,
    ),

  /** Delete a volume */
  deleteVolume: (volumeId: string, token?: string) =>
    apiRequest<void>(
      `/volumes/${volumeId}`,
      {
        method: "DELETE",
      },
      token,
    ),

  /** Backup a volume (returns raw Response for file download) */
  backupVolume: (volumeId: string, token?: string) => {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };
    if (token) {
      headers["Authorization"] = `Bearer ${token}`;
    }
    return fetch(`/api/volumes/${volumeId}/backup`, {
      method: "POST",
      headers,
      credentials: "include",
    });
  },

  // -------------------------------------------------------------------------
  // Alert Configurations
  // -------------------------------------------------------------------------

  /** Get all alert configurations for an app */
  getAlerts: (appId: string, token?: string) =>
    apiRequest<AlertConfigResponse[]>(`/apps/${appId}/alerts`, {}, token),

  /** Get a single alert configuration */
  getAlert: (appId: string, alertId: string, token?: string) =>
    apiRequest<AlertConfigResponse>(`/apps/${appId}/alerts/${alertId}`, {}, token),

  /** Create a new alert configuration */
  createAlert: (appId: string, data: CreateAlertConfigRequest, token?: string) =>
    apiRequest<AlertConfigResponse>(
      `/apps/${appId}/alerts`,
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Update an alert configuration */
  updateAlert: (
    appId: string,
    alertId: string,
    data: UpdateAlertConfigRequest,
    token?: string
  ) =>
    apiRequest<AlertConfigResponse>(
      `/apps/${appId}/alerts/${alertId}`,
      {
        method: "PUT",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Delete an alert configuration */
  deleteAlert: (appId: string, alertId: string, token?: string) =>
    apiRequest<void>(
      `/apps/${appId}/alerts/${alertId}`,
      {
        method: "DELETE",
      },
      token
    ),

  /** Get alert events (triggered alerts) for an app */
  getAlertEvents: (appId: string, limit?: number, token?: string) => {
    const params = limit ? `?limit=${limit}` : "";
    return apiRequest<AlertEventResponse[]>(`/apps/${appId}/alert-events${params}`, {}, token);
  },

  // -------------------------------------------------------------------------
  // App Sharing
  // -------------------------------------------------------------------------

  /** Get list of teams an app is shared with */
  getAppShares: (appId: string, token?: string) =>
    apiRequest<AppShare[]>(`/apps/${appId}/shares`, {}, token),

  /** Share an app with a team */
  createAppShare: (appId: string, data: CreateAppShareRequest, token?: string) =>
    apiRequest<AppShare>(
      `/apps/${appId}/shares`,
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Remove sharing for a team */
  deleteAppShare: (appId: string, teamId: string, token?: string) =>
    apiRequest<void>(
      `/apps/${appId}/shares/${teamId}`,
      {
        method: "DELETE",
      },
      token
    ),

  /** Get apps with sharing information (owned + shared) */
  getAppsWithSharing: (teamId: string, token?: string) =>
    apiRequest<AppWithSharing[]>(`/apps/with-sharing?team_id=${encodeURIComponent(teamId)}`, {}, token),

  // -------------------------------------------------------------------------
  // WebSocket URLs
  // -------------------------------------------------------------------------

  /** Get WebSocket URL for runtime logs streaming (deprecated, use SSE instead) */
  getRuntimeLogsWsUrl: (appId: string, token: string): string => {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    return `${protocol}//${window.location.host}/api/apps/${appId}/logs/stream?token=${encodeURIComponent(token)}`;
  },

  /** Get SSE URL for runtime logs streaming */
  getRuntimeLogsStreamUrl: (appId: string): string => {
    return `${window.location.origin}/api/apps/${appId}/logs/stream`;
  },

  /** Get WebSocket URL for terminal access */
  getTerminalWsUrl: (appId: string, token: string): string => {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    return `${protocol}//${window.location.host}/api/apps/${appId}/terminal?token=${encodeURIComponent(token)}`;
  },
};
