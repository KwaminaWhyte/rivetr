/**
 * API module index.
 * Re-exports all domain-specific APIs and provides a unified api object.
 */

// Re-export core utilities
export { apiRequest, apiRawRequest, getStoredToken } from "./core";
export type { ApiRequestOptions } from "./core";

// Re-export domain-specific APIs
export { projectsApi } from "./projects";
export { bulkApi } from "./bulk";
export { appsApi } from "./apps";
export { databasesApi } from "./databases";
export { servicesApi } from "./services";
export { teamsApi } from "./teams";
export { notificationsApi } from "./notifications";
export { gitApi } from "./git";
export { systemApi } from "./system";
export { previewsApi } from "./previews";
export { oauthApi } from "./oauth";
export { environmentsApi } from "./environments";
export { twoFactorApi } from "./two-factor";
export { jobsApi } from "./jobs";
export { logDrainsApi } from "./log-drains";
export { s3Api } from "./s3";
export { monitoringApi } from "./monitoring";
export { sharedEnvVarsApi } from "./shared-env-vars";

// Import all for combined api object
import { projectsApi } from "./projects";
import { bulkApi } from "./bulk";
import { appsApi } from "./apps";
import { databasesApi } from "./databases";
import { servicesApi } from "./services";
import { teamsApi } from "./teams";
import { notificationsApi } from "./notifications";
import { gitApi } from "./git";
import { systemApi } from "./system";
import { previewsApi } from "./previews";
import { oauthApi } from "./oauth";
import { environmentsApi } from "./environments";
import { twoFactorApi } from "./two-factor";
import { jobsApi } from "./jobs";
import { logDrainsApi } from "./log-drains";
import { s3Api } from "./s3";
import { monitoringApi } from "./monitoring";
import { sharedEnvVarsApi } from "./shared-env-vars";

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
  // Bulk Operations
  bulkStart: bulkApi.bulkStart,
  bulkStop: bulkApi.bulkStop,
  bulkRestart: bulkApi.bulkRestart,
  bulkDeploy: bulkApi.bulkDeploy,
  cloneApp: bulkApi.cloneApp,
  createSnapshot: bulkApi.createSnapshot,
  listSnapshots: bulkApi.listSnapshots,
  restoreSnapshot: bulkApi.restoreSnapshot,
  deleteSnapshot: bulkApi.deleteSnapshot,
  exportProject: bulkApi.exportProject,
  importProject: bulkApi.importProject,
  setMaintenanceMode: bulkApi.setMaintenanceMode,

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
  getCommits: appsApi.getCommits,
  getTags: appsApi.getTags,
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

  // App Sharing
  getAppShares: appsApi.getAppShares,
  createAppShare: appsApi.createAppShare,
  deleteAppShare: appsApi.deleteAppShare,
  getAppsWithSharing: appsApi.getAppsWithSharing,

  // Deployment Approval Workflow
  approveDeployment: appsApi.approveDeployment,
  rejectDeployment: appsApi.rejectDeployment,
  listPendingDeployments: appsApi.listPendingDeployments,

  // Deployment Freeze Windows
  getFreezeWindows: appsApi.getFreezeWindows,
  createFreezeWindow: appsApi.createFreezeWindow,
  deleteFreezeWindow: appsApi.deleteFreezeWindow,

  // SSH Keys
  getSshKeys: gitApi.getSshKeys,
  createSshKey: gitApi.createSshKey,
  deleteSshKey: gitApi.deleteSshKey,

  // Git Providers
  getGitProviders: gitApi.getGitProviders,
  getGitProvider: gitApi.getGitProvider,
  addGitProvider: gitApi.addGitProvider,
  deleteGitProvider: gitApi.deleteGitProvider,
  getGitProviderRepos: gitApi.getGitProviderRepos,

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
  // Team Invitations
  getTeamInvitations: teamsApi.getTeamInvitations,
  createTeamInvitation: teamsApi.createTeamInvitation,
  deleteTeamInvitation: teamsApi.deleteTeamInvitation,
  resendTeamInvitation: teamsApi.resendTeamInvitation,
  validateInvitation: teamsApi.validateInvitation,
  acceptInvitation: teamsApi.acceptInvitation,
  // Team Audit Logs
  getTeamAuditLogs: teamsApi.getTeamAuditLogs,

  // Notifications
  getNotificationChannels: notificationsApi.getNotificationChannels,
  getNotificationChannel: notificationsApi.getNotificationChannel,
  createNotificationChannel: notificationsApi.createNotificationChannel,
  updateNotificationChannel: notificationsApi.updateNotificationChannel,
  deleteNotificationChannel: notificationsApi.deleteNotificationChannel,
  testNotificationChannel: notificationsApi.testNotificationChannel,
  getNotificationSubscriptions: notificationsApi.getNotificationSubscriptions,
  createNotificationSubscription:
    notificationsApi.createNotificationSubscription,
  deleteNotificationSubscription:
    notificationsApi.deleteNotificationSubscription,

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

  // Auto-Update
  getVersionInfo: systemApi.getVersionInfo,
  checkForUpdate: systemApi.checkForUpdate,
  downloadUpdate: systemApi.downloadUpdate,
  applyUpdate: systemApi.applyUpdate,

  // Instance Backup & Restore
  createBackup: systemApi.createBackup,
  listBackups: systemApi.listBackups,
  deleteBackup: systemApi.deleteBackup,
  downloadBackup: systemApi.downloadBackup,
  restoreBackup: systemApi.restoreBackup,

  // Preview Deployments
  getAppPreviews: previewsApi.getAppPreviews,
  getPreview: previewsApi.getPreview,
  deletePreview: previewsApi.deletePreview,
  redeployPreview: previewsApi.redeployPreview,

  // OAuth Login Providers
  getEnabledOAuthProviders: oauthApi.getEnabledProviders,
  getOAuthLoginAuthorizeUrl: oauthApi.getLoginAuthorizeUrl,
  getOAuthProviders: oauthApi.getOAuthProviders,
  createOAuthProvider: oauthApi.createOAuthProvider,
  deleteOAuthProvider: oauthApi.deleteOAuthProvider,
  getOAuthConnections: oauthApi.getOAuthConnections,
  deleteOAuthConnection: oauthApi.deleteOAuthConnection,

  // Two-Factor Authentication
  setup2FA: twoFactorApi.setup,
  verify2FA: twoFactorApi.verify,
  disable2FA: twoFactorApi.disable,
  get2FAStatus: twoFactorApi.getStatus,
  validate2FA: twoFactorApi.validate,

  // Scheduled Jobs
  getJobs: jobsApi.getJobs,
  getJob: jobsApi.getJob,
  createJob: jobsApi.createJob,
  updateJob: jobsApi.updateJob,
  deleteJob: jobsApi.deleteJob,
  triggerJobRun: jobsApi.triggerJobRun,
  getJobRuns: jobsApi.getJobRuns,

  // Log Drains
  getLogDrains: logDrainsApi.getLogDrains,
  createLogDrain: logDrainsApi.createLogDrain,
  updateLogDrain: logDrainsApi.updateLogDrain,
  deleteLogDrain: logDrainsApi.deleteLogDrain,
  testLogDrain: logDrainsApi.testLogDrain,

  // Monitoring
  searchLogs: monitoringApi.searchLogs,
  getLogRetention: monitoringApi.getLogRetention,
  updateLogRetention: monitoringApi.updateLogRetention,
  triggerLogCleanup: monitoringApi.triggerLogCleanup,
  getUptime: monitoringApi.getUptime,
  getUptimeHistory: monitoringApi.getUptimeHistory,
  getScheduledRestarts: monitoringApi.getScheduledRestarts,
  createScheduledRestart: monitoringApi.createScheduledRestart,
  updateScheduledRestart: monitoringApi.updateScheduledRestart,
  deleteScheduledRestart: monitoringApi.deleteScheduledRestart,

  // Project Environments
  getEnvironments: environmentsApi.getEnvironments,
  createEnvironment: environmentsApi.createEnvironment,
  updateEnvironment: environmentsApi.updateEnvironment,
  deleteEnvironment: environmentsApi.deleteEnvironment,
  getEnvironmentEnvVars: environmentsApi.getEnvironmentEnvVars,
  createEnvironmentEnvVar: environmentsApi.createEnvironmentEnvVar,
  updateEnvironmentEnvVar: environmentsApi.updateEnvironmentEnvVar,
  deleteEnvironmentEnvVar: environmentsApi.deleteEnvironmentEnvVar,

  // S3 Storage & Backups
  getS3Configs: s3Api.listConfigs,
  createS3Config: s3Api.createConfig,
  updateS3Config: s3Api.updateConfig,
  deleteS3Config: s3Api.deleteConfig,
  testS3Config: s3Api.testConfig,
  triggerS3Backup: s3Api.triggerBackup,
  getS3Backups: s3Api.listBackups,
  restoreS3Backup: s3Api.restoreBackup,
  deleteS3Backup: s3Api.deleteBackup,

  // Shared Environment Variables
  getTeamEnvVars: sharedEnvVarsApi.getTeamEnvVars,
  createTeamEnvVar: sharedEnvVarsApi.createTeamEnvVar,
  updateTeamEnvVar: sharedEnvVarsApi.updateTeamEnvVar,
  deleteTeamEnvVar: sharedEnvVarsApi.deleteTeamEnvVar,
  getProjectEnvVars: sharedEnvVarsApi.getProjectEnvVars,
  createProjectEnvVar: sharedEnvVarsApi.createProjectEnvVar,
  updateProjectEnvVar: sharedEnvVarsApi.updateProjectEnvVar,
  deleteProjectEnvVar: sharedEnvVarsApi.deleteProjectEnvVar,
  getResolvedEnvVars: sharedEnvVarsApi.getResolvedEnvVars,
};

export default api;
