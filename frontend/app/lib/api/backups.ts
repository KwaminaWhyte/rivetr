/**
 * Backup catalog and restore API (databases and volumes, local and S3).
 */

import { apiRequest } from "./core";
import type { S3BackupResponse } from "@/types/api";
import type {
  BackupCatalogResponse,
  BackupSource,
  CreateVolumeBackupRequest,
  RestoreJob,
  UpsertVolumeBackupScheduleRequest,
  VolumeBackup,
  VolumeBackupSchedule,
} from "@/types/backups";

export type BackupResourceKind = "database" | "volume";

const base = (kind: BackupResourceKind, id: string) =>
  kind === "database" ? `/databases/${id}/backups` : `/volumes/${id}/backups`;

export const backupsApi = {
  /** Local and S3 backups of a database or volume. `scan` lists the S3 buckets too. */
  catalog: (kind: BackupResourceKind, id: string, scan = false) =>
    apiRequest<BackupCatalogResponse>(
      kind === "database"
        ? `/databases/${id}/backups/catalog${scan ? "?scan=true" : ""}`
        : `/volumes/${id}/backups${scan ? "?scan=true" : ""}`,
    ),

  /** Start a restore; returns the background job */
  restore: (kind: BackupResourceKind, id: string, backupId: string, source: BackupSource) =>
    apiRequest<RestoreJob>(`${base(kind, id)}/${backupId}/restore`, {
      method: "POST",
      body: JSON.stringify({ source }),
    }),

  /** Recent restore jobs */
  restores: (kind: BackupResourceKind, id: string) =>
    apiRequest<RestoreJob[]>(`${base(kind, id)}/restores`),

  /** Copy a local database backup to S3 */
  uploadDatabaseBackup: (databaseId: string, backupId: string, storageConfigId: string) =>
    apiRequest<S3BackupResponse>(`/databases/${databaseId}/backups/${backupId}/upload-s3`, {
      method: "POST",
      body: JSON.stringify({ storage_config_id: storageConfigId }),
    }),

  /** Start a volume backup */
  createVolumeBackup: (volumeId: string, data: CreateVolumeBackupRequest) =>
    apiRequest<VolumeBackup>(`/volumes/${volumeId}/backups`, {
      method: "POST",
      body: JSON.stringify(data),
    }),

  /** Delete a volume backup (local archive or S3 object) */
  deleteVolumeBackup: (volumeId: string, backupId: string, source: BackupSource) =>
    apiRequest<void>(`/volumes/${volumeId}/backups/${backupId}?source=${source}`, {
      method: "DELETE",
    }),

  getVolumeSchedule: (volumeId: string) =>
    apiRequest<VolumeBackupSchedule | null>(`/volumes/${volumeId}/backup-schedule`),

  upsertVolumeSchedule: (volumeId: string, data: UpsertVolumeBackupScheduleRequest) =>
    apiRequest<VolumeBackupSchedule>(`/volumes/${volumeId}/backup-schedule`, {
      method: "PUT",
      body: JSON.stringify(data),
    }),

  deleteVolumeSchedule: (volumeId: string) =>
    apiRequest<void>(`/volumes/${volumeId}/backup-schedule`, { method: "DELETE" }),
};
