// Client-side API utility for browser-only fetching
// Uses the same API proxy configured in vite.config.ts

import type {
  App,
  AppStatus,
  BasicAuthStatus,
  ContainerStats,
  CreateBackupScheduleRequest,
  CreateEnvVarRequest,
  CreateManagedDatabaseRequest,
  CreateNotificationChannelRequest,
  CreateNotificationSubscriptionRequest,
  CreateTeamRequest,
  CreateVolumeRequest,
  DatabaseBackup,
  DatabaseBackupSchedule,
  DatabaseLogEntry,
  Deployment,
  DeploymentLog,
  DiskStats,
  EnvVar,
  InviteMemberRequest,
  ManagedDatabase,
  NotificationChannel,
  NotificationSubscription,
  Project,
  ProjectWithApps,
  RecentEvent,
  SshKey,
  SystemHealthStatus,
  SystemStats,
  Team,
  TeamDetail,
  TeamMemberWithUser,
  TeamWithMemberCount,
  TestNotificationRequest,
  UpdateAppRequest,
  UpdateBasicAuthRequest,
  UpdateEnvVarRequest,
  UpdateMemberRoleRequest,
  UpdateNotificationChannelRequest,
  UpdateTeamRequest,
  UpdateVolumeRequest,
  Volume,
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
  getSystemHealth: (token?: string) => apiRequest<SystemHealthStatus>("/system/health", {}, token),

  // WebSocket URLs
  getRuntimeLogsWsUrl: (appId: string, token: string): string => {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    return `${protocol}//${window.location.host}/api/apps/${appId}/logs/stream?token=${encodeURIComponent(token)}`;
  },
  getTerminalWsUrl: (appId: string, token: string): string => {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    return `${protocol}//${window.location.host}/api/apps/${appId}/terminal?token=${encodeURIComponent(token)}`;
  },

  // Notification Channels
  getNotificationChannels: (token?: string) =>
    apiRequest<NotificationChannel[]>("/notification-channels", {}, token),
  getNotificationChannel: (id: string, token?: string) =>
    apiRequest<NotificationChannel>(`/notification-channels/${id}`, {}, token),
  createNotificationChannel: (data: CreateNotificationChannelRequest, token?: string) =>
    apiRequest<NotificationChannel>(
      "/notification-channels",
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),
  updateNotificationChannel: (id: string, data: UpdateNotificationChannelRequest, token?: string) =>
    apiRequest<NotificationChannel>(
      `/notification-channels/${id}`,
      {
        method: "PUT",
        body: JSON.stringify(data),
      },
      token
    ),
  deleteNotificationChannel: (id: string, token?: string) =>
    apiRequest<void>(
      `/notification-channels/${id}`,
      {
        method: "DELETE",
      },
      token
    ),
  testNotificationChannel: (id: string, data?: TestNotificationRequest, token?: string) =>
    apiRequest<void>(
      `/notification-channels/${id}/test`,
      {
        method: "POST",
        body: JSON.stringify(data || {}),
      },
      token
    ),

  // Notification Subscriptions
  getNotificationSubscriptions: (channelId: string, token?: string) =>
    apiRequest<NotificationSubscription[]>(
      `/notification-channels/${channelId}/subscriptions`,
      {},
      token
    ),
  createNotificationSubscription: (
    channelId: string,
    data: CreateNotificationSubscriptionRequest,
    token?: string
  ) =>
    apiRequest<NotificationSubscription>(
      `/notification-channels/${channelId}/subscriptions`,
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),
  deleteNotificationSubscription: (id: string, token?: string) =>
    apiRequest<void>(
      `/notification-subscriptions/${id}`,
      {
        method: "DELETE",
      },
      token
    ),

  // Teams
  getTeams: (token?: string) =>
    apiRequest<TeamWithMemberCount[]>("/teams", {}, token),
  getTeam: (id: string, token?: string) =>
    apiRequest<TeamDetail>(`/teams/${id}`, {}, token),
  createTeam: (data: CreateTeamRequest, token?: string) =>
    apiRequest<Team>(
      "/teams",
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),
  updateTeam: (id: string, data: UpdateTeamRequest, token?: string) =>
    apiRequest<Team>(
      `/teams/${id}`,
      {
        method: "PUT",
        body: JSON.stringify(data),
      },
      token
    ),
  deleteTeam: (id: string, token?: string) =>
    apiRequest<void>(
      `/teams/${id}`,
      {
        method: "DELETE",
      },
      token
    ),
  getTeamMembers: (teamId: string, token?: string) =>
    apiRequest<TeamMemberWithUser[]>(`/teams/${teamId}/members`, {}, token),
  inviteTeamMember: (teamId: string, data: InviteMemberRequest, token?: string) =>
    apiRequest<TeamMemberWithUser>(
      `/teams/${teamId}/members`,
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),
  updateTeamMemberRole: (teamId: string, userId: string, data: UpdateMemberRoleRequest, token?: string) =>
    apiRequest<TeamMemberWithUser>(
      `/teams/${teamId}/members/${userId}`,
      {
        method: "PUT",
        body: JSON.stringify(data),
      },
      token
    ),
  removeTeamMember: (teamId: string, userId: string, token?: string) =>
    apiRequest<void>(
      `/teams/${teamId}/members/${userId}`,
      {
        method: "DELETE",
      },
      token
    ),

  // Volumes
  getVolumes: (appId: string, token?: string) =>
    apiRequest<Volume[]>(`/apps/${appId}/volumes`, {}, token),
  getVolume: (volumeId: string, token?: string) =>
    apiRequest<Volume>(`/volumes/${volumeId}`, {}, token),
  createVolume: (appId: string, data: CreateVolumeRequest, token?: string) =>
    apiRequest<Volume>(
      `/apps/${appId}/volumes`,
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),
  updateVolume: (volumeId: string, data: UpdateVolumeRequest, token?: string) =>
    apiRequest<Volume>(
      `/volumes/${volumeId}`,
      {
        method: "PUT",
        body: JSON.stringify(data),
      },
      token
    ),
  deleteVolume: (volumeId: string, token?: string) =>
    apiRequest<void>(
      `/volumes/${volumeId}`,
      {
        method: "DELETE",
      },
      token
    ),
  backupVolume: (volumeId: string, token?: string) => {
    // For backup, we need to handle the file download differently
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

  // Managed Databases
  getDatabases: (reveal = false, token?: string) => {
    const params = reveal ? "?reveal=true" : "";
    return apiRequest<ManagedDatabase[]>(`/databases${params}`, {}, token);
  },
  getDatabase: (id: string, reveal = false, token?: string) => {
    const params = reveal ? "?reveal=true" : "";
    return apiRequest<ManagedDatabase>(`/databases/${id}${params}`, {}, token);
  },
  createDatabase: (data: CreateManagedDatabaseRequest, token?: string) =>
    apiRequest<ManagedDatabase>(
      "/databases",
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),
  deleteDatabase: (id: string, token?: string) =>
    apiRequest<void>(
      `/databases/${id}`,
      {
        method: "DELETE",
      },
      token
    ),
  startDatabase: (id: string, token?: string) =>
    apiRequest<ManagedDatabase>(
      `/databases/${id}/start`,
      {
        method: "POST",
      },
      token
    ),
  stopDatabase: (id: string, token?: string) =>
    apiRequest<ManagedDatabase>(
      `/databases/${id}/stop`,
      {
        method: "POST",
      },
      token
    ),
  getDatabaseLogs: (id: string, lines = 100, token?: string) =>
    apiRequest<DatabaseLogEntry[]>(
      `/databases/${id}/logs?lines=${lines}`,
      {},
      token
    ),
  getDatabaseStats: (id: string, token?: string) =>
    apiRequest<ContainerStats>(`/databases/${id}/stats`, {}, token),

  // Database Backups
  getDatabaseBackups: (databaseId: string, limit = 50, token?: string) =>
    apiRequest<DatabaseBackup[]>(
      `/databases/${databaseId}/backups?limit=${limit}`,
      {},
      token
    ),
  getDatabaseBackup: (databaseId: string, backupId: string, token?: string) =>
    apiRequest<DatabaseBackup>(
      `/databases/${databaseId}/backups/${backupId}`,
      {},
      token
    ),
  createDatabaseBackup: (databaseId: string, token?: string) =>
    apiRequest<DatabaseBackup>(
      `/databases/${databaseId}/backups`,
      { method: "POST" },
      token
    ),
  deleteDatabaseBackup: (databaseId: string, backupId: string, token?: string) =>
    apiRequest<void>(
      `/databases/${databaseId}/backups/${backupId}`,
      { method: "DELETE" },
      token
    ),
  getDatabaseBackupSchedule: (databaseId: string, token?: string) =>
    apiRequest<DatabaseBackupSchedule | null>(
      `/databases/${databaseId}/backups/schedule`,
      {},
      token
    ),
  upsertDatabaseBackupSchedule: (
    databaseId: string,
    data: CreateBackupScheduleRequest,
    token?: string
  ) =>
    apiRequest<DatabaseBackupSchedule>(
      `/databases/${databaseId}/backups/schedule`,
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),
  deleteDatabaseBackupSchedule: (databaseId: string, token?: string) =>
    apiRequest<void>(
      `/databases/${databaseId}/backups/schedule`,
      { method: "DELETE" },
      token
    ),

  // Database Backup Download
  downloadDatabaseBackup: async (databaseId: string, backupId: string, token?: string) => {
    const headers: Record<string, string> = {};
    if (token) {
      headers["Authorization"] = `Bearer ${token}`;
    }
    const response = await fetch(`/api/databases/${databaseId}/backups/${backupId}/download`, {
      headers,
      credentials: "include",
    });
    if (!response.ok) {
      const error = await response.text();
      throw new Error(error || `Download failed: ${response.status}`);
    }
    // Get filename from Content-Disposition header
    const contentDisposition = response.headers.get("Content-Disposition");
    let filename = "backup.sql";
    if (contentDisposition) {
      const match = contentDisposition.match(/filename="?([^"]+)"?/);
      if (match) {
        filename = match[1];
      }
    }
    // Convert to blob and trigger download
    const blob = await response.blob();
    const url = window.URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = filename;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    window.URL.revokeObjectURL(url);
  },
};

export default api;
