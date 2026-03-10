/**
 * Bulk operations API module.
 * Handles bulk start/stop/restart/deploy, clone, snapshots, export/import,
 * and maintenance mode.
 */

import { apiRequest } from "./core";
import type {
  BulkAppIdsRequest,
  BulkOperationResponse,
  CloneAppRequest,
  CloneAppResponse,
  ConfigSnapshot,
  CreateSnapshotRequest,
  App,
  MaintenanceModeRequest,
  MaintenanceModeResponse,
  ProjectExport,
  ProjectImportResponse,
} from "@/types/api";

export const bulkApi = {
  // -------------------------------------------------------------------------
  // Bulk Start / Stop / Restart / Deploy
  // -------------------------------------------------------------------------

  /** Start multiple apps */
  bulkStart: (data: BulkAppIdsRequest, token?: string) =>
    apiRequest<BulkOperationResponse>("/bulk/start", {
      method: "POST",
      body: JSON.stringify(data),
    }, token),

  /** Stop multiple apps */
  bulkStop: (data: BulkAppIdsRequest, token?: string) =>
    apiRequest<BulkOperationResponse>("/bulk/stop", {
      method: "POST",
      body: JSON.stringify(data),
    }, token),

  /** Restart multiple apps */
  bulkRestart: (data: BulkAppIdsRequest, token?: string) =>
    apiRequest<BulkOperationResponse>("/bulk/restart", {
      method: "POST",
      body: JSON.stringify(data),
    }, token),

  /** Trigger deploy for multiple apps */
  bulkDeploy: (data: BulkAppIdsRequest, token?: string) =>
    apiRequest<BulkOperationResponse>("/bulk/deploy", {
      method: "POST",
      body: JSON.stringify(data),
    }, token),

  // -------------------------------------------------------------------------
  // Clone App
  // -------------------------------------------------------------------------

  /** Clone an app (deep copy of config, env vars, volumes) */
  cloneApp: (appId: string, data?: CloneAppRequest, token?: string) =>
    apiRequest<CloneAppResponse>(`/apps/${appId}/clone`, {
      method: "POST",
      body: data ? JSON.stringify(data) : undefined,
    }, token),

  // -------------------------------------------------------------------------
  // Config Snapshots
  // -------------------------------------------------------------------------

  /** Save current config as a named snapshot */
  createSnapshot: (appId: string, data: CreateSnapshotRequest, token?: string) =>
    apiRequest<ConfigSnapshot>(`/apps/${appId}/snapshots`, {
      method: "POST",
      body: JSON.stringify(data),
    }, token),

  /** List config snapshots for an app */
  listSnapshots: (appId: string, token?: string) =>
    apiRequest<ConfigSnapshot[]>(`/apps/${appId}/snapshots`, {}, token),

  /** Restore an app's config from a snapshot */
  restoreSnapshot: (appId: string, snapshotId: string, token?: string) =>
    apiRequest<App>(`/apps/${appId}/snapshots/${snapshotId}/restore`, {
      method: "POST",
    }, token),

  /** Delete a config snapshot */
  deleteSnapshot: (appId: string, snapshotId: string, token?: string) =>
    apiRequest<void>(`/apps/${appId}/snapshots/${snapshotId}`, {
      method: "DELETE",
    }, token),

  // -------------------------------------------------------------------------
  // Project Export / Import
  // -------------------------------------------------------------------------

  /** Export a project as JSON (all apps, env vars, domains) */
  exportProject: (projectId: string, token?: string) =>
    apiRequest<ProjectExport>(`/projects/${projectId}/export`, {}, token),

  /** Import a project from JSON */
  importProject: (projectId: string, data: ProjectExport, token?: string) =>
    apiRequest<ProjectImportResponse>(`/projects/${projectId}/import`, {
      method: "POST",
      body: JSON.stringify(data),
    }, token),

  // -------------------------------------------------------------------------
  // Maintenance Mode
  // -------------------------------------------------------------------------

  /** Toggle maintenance mode for an app */
  setMaintenanceMode: (
    appId: string,
    data: MaintenanceModeRequest,
    token?: string,
  ) =>
    apiRequest<MaintenanceModeResponse>(`/apps/${appId}/maintenance`, {
      method: "PUT",
      body: JSON.stringify(data),
    }, token),
};
