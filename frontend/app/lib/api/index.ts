/**
 * API module index.
 * Re-exports all domain-specific APIs and provides a unified api object.
 */

// Re-export core utilities
export { apiRequest, apiRawRequest, getStoredToken } from "./core";

// Re-export domain-specific APIs
export { projectsApi } from "./projects";
export { appsApi } from "./apps";
export { databasesApi } from "./databases";
export { servicesApi } from "./services";
export { teamsApi } from "./teams";
export { notificationsApi } from "./notifications";
export { gitApi } from "./git";
export { systemApi } from "./system";
export { previewsApi } from "./previews";

// Import all for combined api object
import { projectsApi } from "./projects";
import { appsApi } from "./apps";
import { databasesApi } from "./databases";
import { servicesApi } from "./services";
import { teamsApi } from "./teams";
import { notificationsApi } from "./notifications";
import { gitApi } from "./git";
import { systemApi } from "./system";
import { previewsApi } from "./previews";

/**
 * Combined API object for backward compatibility.
 * Provides a single entry point for all API methods.
 *
 * For new code, consider importing domain-specific APIs directly:
 * ```ts
 * import { appsApi } from "@/lib/api/apps";
 * ```
 */
export const api = {
  // Projects
  getProjects: projectsApi.getProjects,
  getProject: projectsApi.getProject,
  createProject: projectsApi.createProject,
  updateProject: projectsApi.updateProject,
  deleteProject: projectsApi.deleteProject,

  // Apps
  getApps: appsApi.getApps,
  getApp: appsApi.getApp,
  createApp: appsApi.createApp,
  updateApp: appsApi.updateApp,
  deleteApp: appsApi.deleteApp,
  assignAppToProject: appsApi.assignAppToProject,
  getAppStatus: appsApi.getAppStatus,
  startApp: appsApi.startApp,
  stopApp: appsApi.stopApp,
  restartApp: appsApi.restartApp,
  getDeployments: appsApi.getDeployments,
  getDeploymentLogs: appsApi.getDeploymentLogs,
  triggerDeploy: appsApi.triggerDeploy,
  rollbackDeployment: appsApi.rollbackDeployment,
  getAppStats: appsApi.getAppStats,
  getEnvVars: appsApi.getEnvVars,
  getEnvVar: appsApi.getEnvVar,
  createEnvVar: appsApi.createEnvVar,
  updateEnvVar: appsApi.updateEnvVar,
  deleteEnvVar: appsApi.deleteEnvVar,
  getBasicAuth: appsApi.getBasicAuth,
  updateBasicAuth: appsApi.updateBasicAuth,
  deleteBasicAuth: appsApi.deleteBasicAuth,
  getVolumes: appsApi.getVolumes,
  getVolume: appsApi.getVolume,
  createVolume: appsApi.createVolume,
  updateVolume: appsApi.updateVolume,
  deleteVolume: appsApi.deleteVolume,
  backupVolume: appsApi.backupVolume,
  getRuntimeLogsWsUrl: appsApi.getRuntimeLogsWsUrl,
  getRuntimeLogsStreamUrl: appsApi.getRuntimeLogsStreamUrl,
  getTerminalWsUrl: appsApi.getTerminalWsUrl,
  // Upload deployments
  uploadDeploy: appsApi.uploadDeploy,
  detectBuildType: appsApi.detectBuildType,
  uploadCreateApp: appsApi.uploadCreateApp,

  // Alert Configurations
  getAlerts: appsApi.getAlerts,
  getAlert: appsApi.getAlert,
  createAlert: appsApi.createAlert,
  updateAlert: appsApi.updateAlert,
  deleteAlert: appsApi.deleteAlert,
  getAlertEvents: appsApi.getAlertEvents,

  // SSH Keys
  getSshKeys: gitApi.getSshKeys,
  createSshKey: gitApi.createSshKey,
  deleteSshKey: gitApi.deleteSshKey,

  // Git Providers
  getGitProviders: gitApi.getGitProviders,
  getGitProvider: gitApi.getGitProvider,
  deleteGitProvider: gitApi.deleteGitProvider,
  getGitProviderRepos: gitApi.getGitProviderRepos,
  getGitProviderAuthUrl: gitApi.getGitProviderAuthUrl,

  // GitHub Apps
  getGitHubApps: gitApi.getGitHubApps,
  getGitHubApp: gitApi.getGitHubApp,
  deleteGitHubApp: gitApi.deleteGitHubApp,
  createGitHubAppManifest: gitApi.createGitHubAppManifest,
  getGitHubAppInstallations: gitApi.getGitHubAppInstallations,
  getAllGitHubAppInstallations: gitApi.getAllGitHubAppInstallations,
  getGitHubAppRepositories: gitApi.getGitHubAppRepositories,
  getGitHubAppRepoBranches: gitApi.getGitHubAppRepoBranches,
  getGitHubAppInstallUrl: gitApi.getGitHubAppInstallUrl,

  // Databases
  getDatabases: databasesApi.getDatabases,
  getDatabase: databasesApi.getDatabase,
  createDatabase: databasesApi.createDatabase,
  updateDatabase: databasesApi.updateDatabase,
  deleteDatabase: databasesApi.deleteDatabase,
  startDatabase: databasesApi.startDatabase,
  stopDatabase: databasesApi.stopDatabase,
  getDatabaseLogs: databasesApi.getDatabaseLogs,
  getDatabaseStats: databasesApi.getDatabaseStats,
  getDatabaseBackups: databasesApi.getDatabaseBackups,
  getDatabaseBackup: databasesApi.getDatabaseBackup,
  createDatabaseBackup: databasesApi.createDatabaseBackup,
  deleteDatabaseBackup: databasesApi.deleteDatabaseBackup,
  getDatabaseBackupSchedule: databasesApi.getDatabaseBackupSchedule,
  upsertDatabaseBackupSchedule: databasesApi.upsertDatabaseBackupSchedule,
  deleteDatabaseBackupSchedule: databasesApi.deleteDatabaseBackupSchedule,
  downloadDatabaseBackup: databasesApi.downloadDatabaseBackup,

  // Services
  getServices: servicesApi.getServices,
  getService: servicesApi.getService,
  createService: servicesApi.createService,
  updateService: servicesApi.updateService,
  deleteService: servicesApi.deleteService,
  startService: servicesApi.startService,
  stopService: servicesApi.stopService,
  getServiceLogs: servicesApi.getServiceLogs,
  getServiceLogsStreamUrl: servicesApi.getServiceLogsStreamUrl,

  // Templates
  getTemplates: servicesApi.getTemplates,
  getTemplate: servicesApi.getTemplate,
  getTemplateCategories: servicesApi.getTemplateCategories,
  deployTemplate: servicesApi.deployTemplate,

  // Teams
  getTeams: teamsApi.getTeams,
  getTeam: teamsApi.getTeam,
  createTeam: teamsApi.createTeam,
  updateTeam: teamsApi.updateTeam,
  deleteTeam: teamsApi.deleteTeam,
  getTeamMembers: teamsApi.getTeamMembers,
  inviteTeamMember: teamsApi.inviteTeamMember,
  updateTeamMemberRole: teamsApi.updateTeamMemberRole,
  removeTeamMember: teamsApi.removeTeamMember,

  // Notifications
  getNotificationChannels: notificationsApi.getNotificationChannels,
  getNotificationChannel: notificationsApi.getNotificationChannel,
  createNotificationChannel: notificationsApi.createNotificationChannel,
  updateNotificationChannel: notificationsApi.updateNotificationChannel,
  deleteNotificationChannel: notificationsApi.deleteNotificationChannel,
  testNotificationChannel: notificationsApi.testNotificationChannel,
  getNotificationSubscriptions: notificationsApi.getNotificationSubscriptions,
  createNotificationSubscription: notificationsApi.createNotificationSubscription,
  deleteNotificationSubscription: notificationsApi.deleteNotificationSubscription,

  // Team Notification Channels
  getTeamNotificationChannels: notificationsApi.getTeamNotificationChannels,
  getTeamNotificationChannel: notificationsApi.getTeamNotificationChannel,
  createTeamNotificationChannel: notificationsApi.createTeamNotificationChannel,
  updateTeamNotificationChannel: notificationsApi.updateTeamNotificationChannel,
  deleteTeamNotificationChannel: notificationsApi.deleteTeamNotificationChannel,
  testTeamNotificationChannel: notificationsApi.testTeamNotificationChannel,

  // System
  getSystemStats: systemApi.getSystemStats,
  getDiskStats: systemApi.getDiskStats,
  getSystemHealth: systemApi.getSystemHealth,
  getRecentEvents: systemApi.getRecentEvents,
  getDashboardCosts: systemApi.getDashboardCosts,
  getTeamCosts: systemApi.getTeamCosts,
  getProjectCosts: systemApi.getProjectCosts,
  getAppCosts: systemApi.getAppCosts,

  // Audit Logs
  getAuditLogs: systemApi.getAuditLogs,
  getAuditActionTypes: systemApi.getAuditActionTypes,
  getAuditResourceTypes: systemApi.getAuditResourceTypes,

  // Alert Defaults (Settings)
  getAlertDefaults: systemApi.getAlertDefaults,
  updateAlertDefaults: systemApi.updateAlertDefaults,
  getAlertStats: systemApi.getAlertStats,

  // Preview Deployments
  getAppPreviews: previewsApi.getAppPreviews,
  getPreview: previewsApi.getPreview,
  deletePreview: previewsApi.deletePreview,
  redeployPreview: previewsApi.redeployPreview,
};

export default api;
