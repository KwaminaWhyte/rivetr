/**
 * Databases API module.
 * Handles managed database operations including backups.
 */

import { apiRequest } from "./core";
import type {
  ManagedDatabase,
  CreateManagedDatabaseRequest,
  UpdateManagedDatabaseRequest,
  DatabaseLogEntry,
  ContainerStats,
  DatabaseBackup,
  DatabaseBackupSchedule,
  CreateBackupScheduleRequest,
} from "@/types/api";

export const databasesApi = {
  // -------------------------------------------------------------------------
  // Database CRUD
  // -------------------------------------------------------------------------

  /** List all databases */
  getDatabases: (reveal = false, token?: string) => {
    const params = reveal ? "?reveal=true" : "";
    return apiRequest<ManagedDatabase[]>(`/databases${params}`, {}, token);
  },

  /** Get a single database by ID */
  getDatabase: (id: string, reveal = false, token?: string) => {
    const params = reveal ? "?reveal=true" : "";
    return apiRequest<ManagedDatabase>(`/databases/${id}${params}`, {}, token);
  },

  /** Create a new managed database */
  createDatabase: (data: CreateManagedDatabaseRequest, token?: string) =>
    apiRequest<ManagedDatabase>(
      "/databases",
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Update a managed database */
  updateDatabase: (
    id: string,
    data: UpdateManagedDatabaseRequest,
    token?: string
  ) =>
    apiRequest<ManagedDatabase>(
      `/databases/${id}`,
      {
        method: "PUT",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Delete a managed database */
  deleteDatabase: (id: string, token?: string) =>
    apiRequest<void>(
      `/databases/${id}`,
      {
        method: "DELETE",
      },
      token
    ),

  // -------------------------------------------------------------------------
  // Database Control
  // -------------------------------------------------------------------------

  /** Start a database */
  startDatabase: (id: string, token?: string) =>
    apiRequest<ManagedDatabase>(
      `/databases/${id}/start`,
      {
        method: "POST",
      },
      token
    ),

  /** Stop a database */
  stopDatabase: (id: string, token?: string) =>
    apiRequest<ManagedDatabase>(
      `/databases/${id}/stop`,
      {
        method: "POST",
      },
      token
    ),

  // -------------------------------------------------------------------------
  // Database Logs & Stats
  // -------------------------------------------------------------------------

  /** Get database container logs */
  getDatabaseLogs: (id: string, lines = 100, token?: string) =>
    apiRequest<DatabaseLogEntry[]>(
      `/databases/${id}/logs?lines=${lines}`,
      {},
      token
    ),

  /** Get database container stats */
  getDatabaseStats: (id: string, token?: string) =>
    apiRequest<ContainerStats>(`/databases/${id}/stats`, {}, token),

  // -------------------------------------------------------------------------
  // Database Backups
  // -------------------------------------------------------------------------

  /** List backups for a database */
  getDatabaseBackups: (databaseId: string, limit = 50, token?: string) =>
    apiRequest<DatabaseBackup[]>(
      `/databases/${databaseId}/backups?limit=${limit}`,
      {},
      token
    ),

  /** Get a single backup */
  getDatabaseBackup: (databaseId: string, backupId: string, token?: string) =>
    apiRequest<DatabaseBackup>(
      `/databases/${databaseId}/backups/${backupId}`,
      {},
      token
    ),

  /** Create a manual backup */
  createDatabaseBackup: (databaseId: string, token?: string) =>
    apiRequest<DatabaseBackup>(
      `/databases/${databaseId}/backups`,
      { method: "POST" },
      token
    ),

  /** Delete a backup */
  deleteDatabaseBackup: (
    databaseId: string,
    backupId: string,
    token?: string
  ) =>
    apiRequest<void>(
      `/databases/${databaseId}/backups/${backupId}`,
      { method: "DELETE" },
      token
    ),

  // -------------------------------------------------------------------------
  // Backup Schedules
  // -------------------------------------------------------------------------

  /** Get backup schedule for a database */
  getDatabaseBackupSchedule: (databaseId: string, token?: string) =>
    apiRequest<DatabaseBackupSchedule | null>(
      `/databases/${databaseId}/backups/schedule`,
      {},
      token
    ),

  /** Create or update backup schedule */
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

  /** Delete backup schedule */
  deleteDatabaseBackupSchedule: (databaseId: string, token?: string) =>
    apiRequest<void>(
      `/databases/${databaseId}/backups/schedule`,
      { method: "DELETE" },
      token
    ),

  // -------------------------------------------------------------------------
  // Backup Download
  // -------------------------------------------------------------------------

  /** Download a database backup */
  downloadDatabaseBackup: async (
    databaseId: string,
    backupId: string,
    token?: string
  ) => {
    const headers: Record<string, string> = {};
    if (token) {
      headers["Authorization"] = `Bearer ${token}`;
    }
    const response = await fetch(
      `/api/databases/${databaseId}/backups/${backupId}/download`,
      {
        headers,
        credentials: "include",
      }
    );
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
