import type {
  App,
  AppStatus,
  ContainerStats,
  CreateAppRequest,
  CreateEnvVarRequest,
  CreateNotificationChannelRequest,
  CreateNotificationSubscriptionRequest,
  CreateProjectRequest,
  CreateSshKeyRequest,
  CreateTeamRequest,
  CreateVolumeRequest,
  Deployment,
  DeploymentLog,
  DiskStats,
  EnvVar,
  GitProvider,
  GitRepository,
  InviteMemberRequest,
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
  UpdateEnvVarRequest,
  UpdateMemberRoleRequest,
  UpdateNotificationChannelRequest,
  UpdateProjectRequest,
  UpdateSshKeyRequest,
  UpdateTeamRequest,
  UpdateVolumeRequest,
  Volume,
} from "@/types/api";

const API_BASE = process.env.API_BASE || "http://localhost:8080";

async function apiRequest<T>(
  path: string,
  token: string,
  options: RequestInit = {}
): Promise<T> {
  const response = await fetch(`${API_BASE}/api${path}`, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
      ...options.headers,
    },
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
  // Apps
  getApps: (token: string) => apiRequest<App[]>("/apps", token),
  getApp: (token: string, id: string) => apiRequest<App>(`/apps/${id}`, token),
  createApp: (token: string, data: CreateAppRequest) =>
    apiRequest<App>("/apps", token, {
      method: "POST",
      body: JSON.stringify(data),
    }),
  updateApp: (token: string, id: string, data: UpdateAppRequest) =>
    apiRequest<App>(`/apps/${id}`, token, {
      method: "PUT",
      body: JSON.stringify(data),
    }),
  deleteApp: (token: string, id: string) =>
    apiRequest<void>(`/apps/${id}`, token, { method: "DELETE" }),
  assignAppToProject: (token: string, appId: string, projectId: string | null) =>
    apiRequest<App>(`/apps/${appId}`, token, {
      method: "PUT",
      body: JSON.stringify({ project_id: projectId }),
    }),
  getAppStatus: (token: string, id: string) =>
    apiRequest<AppStatus>(`/apps/${id}/status`, token),
  startApp: (token: string, id: string) =>
    apiRequest<AppStatus>(`/apps/${id}/start`, token, { method: "POST" }),
  stopApp: (token: string, id: string) =>
    apiRequest<AppStatus>(`/apps/${id}/stop`, token, { method: "POST" }),

  // Projects
  getProjects: (token: string) => apiRequest<Project[]>("/projects", token),
  getProject: (token: string, id: string) =>
    apiRequest<ProjectWithApps>(`/projects/${id}`, token),
  createProject: (token: string, data: CreateProjectRequest) =>
    apiRequest<Project>("/projects", token, {
      method: "POST",
      body: JSON.stringify(data),
    }),
  updateProject: (token: string, id: string, data: UpdateProjectRequest) =>
    apiRequest<Project>(`/projects/${id}`, token, {
      method: "PUT",
      body: JSON.stringify(data),
    }),
  deleteProject: (token: string, id: string) =>
    apiRequest<void>(`/projects/${id}`, token, { method: "DELETE" }),

  // Deployments
  getDeployments: (token: string, appId: string) =>
    apiRequest<Deployment[]>(`/apps/${appId}/deployments`, token),
  getDeployment: (token: string, id: string) =>
    apiRequest<Deployment>(`/deployments/${id}`, token),
  getDeploymentLogs: (token: string, id: string) =>
    apiRequest<DeploymentLog[]>(`/deployments/${id}/logs`, token),
  triggerDeploy: (token: string, appId: string) =>
    apiRequest<Deployment>(`/apps/${appId}/deploy`, token, { method: "POST" }),
  rollbackDeployment: (token: string, id: string) =>
    apiRequest<Deployment>(`/deployments/${id}/rollback`, token, { method: "POST" }),

  // Container Stats
  getAppStats: (token: string, appId: string) =>
    apiRequest<ContainerStats>(`/apps/${appId}/stats`, token),

  // SSH Keys
  getSshKeys: (token: string) => apiRequest<SshKey[]>("/ssh-keys", token),
  getSshKey: (token: string, id: string) =>
    apiRequest<SshKey>(`/ssh-keys/${id}`, token),
  createSshKey: (token: string, data: CreateSshKeyRequest) =>
    apiRequest<SshKey>("/ssh-keys", token, {
      method: "POST",
      body: JSON.stringify(data),
    }),
  updateSshKey: (token: string, id: string, data: UpdateSshKeyRequest) =>
    apiRequest<SshKey>(`/ssh-keys/${id}`, token, {
      method: "PUT",
      body: JSON.stringify(data),
    }),
  deleteSshKey: (token: string, id: string) =>
    apiRequest<void>(`/ssh-keys/${id}`, token, { method: "DELETE" }),

  // Git Providers
  getGitProviders: (token: string) =>
    apiRequest<GitProvider[]>("/git-providers", token),
  getGitProvider: (token: string, id: string) =>
    apiRequest<GitProvider>(`/git-providers/${id}`, token),
  deleteGitProvider: (token: string, id: string) =>
    apiRequest<void>(`/git-providers/${id}`, token, { method: "DELETE" }),
  getProviderRepos: (token: string, providerId: string) =>
    apiRequest<GitRepository[]>(`/git-providers/${providerId}/repos`, token),

  // Environment Variables
  getEnvVars: (token: string, appId: string, reveal = false) => {
    const params = reveal ? "?reveal=true" : "";
    return apiRequest<EnvVar[]>(`/apps/${appId}/env-vars${params}`, token);
  },
  createEnvVar: (token: string, appId: string, data: CreateEnvVarRequest) =>
    apiRequest<EnvVar>(`/apps/${appId}/env-vars`, token, {
      method: "POST",
      body: JSON.stringify(data),
    }),
  updateEnvVar: (token: string, appId: string, key: string, data: UpdateEnvVarRequest) =>
    apiRequest<EnvVar>(`/apps/${appId}/env-vars/${encodeURIComponent(key)}`, token, {
      method: "PUT",
      body: JSON.stringify(data),
    }),
  deleteEnvVar: (token: string, appId: string, key: string) =>
    apiRequest<void>(`/apps/${appId}/env-vars/${encodeURIComponent(key)}`, token, {
      method: "DELETE",
    }),

  // System
  getSystemStats: (token: string) =>
    apiRequest<SystemStats>("/system/stats", token),
  getDiskStats: (token: string) =>
    apiRequest<DiskStats>("/system/disk", token),
  getRecentEvents: (token: string) =>
    apiRequest<RecentEvent[]>("/events/recent", token),
  getSystemHealth: (token: string) =>
    apiRequest<SystemHealthStatus>("/system/health", token),

  // Notification Channels
  getNotificationChannels: (token: string) =>
    apiRequest<NotificationChannel[]>("/notification-channels", token),
  getNotificationChannel: (token: string, id: string) =>
    apiRequest<NotificationChannel>(`/notification-channels/${id}`, token),
  createNotificationChannel: (token: string, data: CreateNotificationChannelRequest) =>
    apiRequest<NotificationChannel>("/notification-channels", token, {
      method: "POST",
      body: JSON.stringify(data),
    }),
  updateNotificationChannel: (token: string, id: string, data: UpdateNotificationChannelRequest) =>
    apiRequest<NotificationChannel>(`/notification-channels/${id}`, token, {
      method: "PUT",
      body: JSON.stringify(data),
    }),
  deleteNotificationChannel: (token: string, id: string) =>
    apiRequest<void>(`/notification-channels/${id}`, token, { method: "DELETE" }),
  testNotificationChannel: (token: string, id: string, data?: TestNotificationRequest) =>
    apiRequest<void>(`/notification-channels/${id}/test`, token, {
      method: "POST",
      body: JSON.stringify(data || {}),
    }),

  // Notification Subscriptions
  getNotificationSubscriptions: (token: string, channelId: string) =>
    apiRequest<NotificationSubscription[]>(
      `/notification-channels/${channelId}/subscriptions`,
      token
    ),
  createNotificationSubscription: (
    token: string,
    channelId: string,
    data: CreateNotificationSubscriptionRequest
  ) =>
    apiRequest<NotificationSubscription>(
      `/notification-channels/${channelId}/subscriptions`,
      token,
      {
        method: "POST",
        body: JSON.stringify(data),
      }
    ),
  deleteNotificationSubscription: (token: string, id: string) =>
    apiRequest<void>(`/notification-subscriptions/${id}`, token, { method: "DELETE" }),

  // Teams
  getTeams: (token: string) => apiRequest<TeamWithMemberCount[]>("/teams", token),
  getTeam: (token: string, id: string) =>
    apiRequest<TeamDetail>(`/teams/${id}`, token),
  createTeam: (token: string, data: CreateTeamRequest) =>
    apiRequest<Team>("/teams", token, {
      method: "POST",
      body: JSON.stringify(data),
    }),
  updateTeam: (token: string, id: string, data: UpdateTeamRequest) =>
    apiRequest<Team>(`/teams/${id}`, token, {
      method: "PUT",
      body: JSON.stringify(data),
    }),
  deleteTeam: (token: string, id: string) =>
    apiRequest<void>(`/teams/${id}`, token, { method: "DELETE" }),
  getTeamMembers: (token: string, teamId: string) =>
    apiRequest<TeamMemberWithUser[]>(`/teams/${teamId}/members`, token),
  inviteTeamMember: (token: string, teamId: string, data: InviteMemberRequest) =>
    apiRequest<TeamMemberWithUser>(`/teams/${teamId}/members`, token, {
      method: "POST",
      body: JSON.stringify(data),
    }),
  updateTeamMemberRole: (
    token: string,
    teamId: string,
    userId: string,
    data: UpdateMemberRoleRequest
  ) =>
    apiRequest<TeamMemberWithUser>(`/teams/${teamId}/members/${userId}`, token, {
      method: "PUT",
      body: JSON.stringify(data),
    }),
  removeTeamMember: (token: string, teamId: string, userId: string) =>
    apiRequest<void>(`/teams/${teamId}/members/${userId}`, token, {
      method: "DELETE",
    }),

  // Volumes
  getVolumes: (token: string, appId: string) =>
    apiRequest<Volume[]>(`/apps/${appId}/volumes`, token),
  getVolume: (token: string, volumeId: string) =>
    apiRequest<Volume>(`/volumes/${volumeId}`, token),
  createVolume: (token: string, appId: string, data: CreateVolumeRequest) =>
    apiRequest<Volume>(`/apps/${appId}/volumes`, token, {
      method: "POST",
      body: JSON.stringify(data),
    }),
  updateVolume: (token: string, volumeId: string, data: UpdateVolumeRequest) =>
    apiRequest<Volume>(`/volumes/${volumeId}`, token, {
      method: "PUT",
      body: JSON.stringify(data),
    }),
  deleteVolume: (token: string, volumeId: string) =>
    apiRequest<void>(`/volumes/${volumeId}`, token, { method: "DELETE" }),
};

// Public API methods (no auth required)
export async function login(email: string, password: string) {
  const response = await fetch(`${API_BASE}/api/auth/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email, password }),
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(error || "Login failed");
  }

  return response.json() as Promise<{ token: string; user: { id: string; email: string; name: string } }>;
}

export async function setup(data: { name: string; email: string; password: string }) {
  const response = await fetch(`${API_BASE}/api/auth/setup`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(error || "Setup failed");
  }

  return response.json() as Promise<{ token: string; user: { id: string; email: string; name: string } }>;
}
