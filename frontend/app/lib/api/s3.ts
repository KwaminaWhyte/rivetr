/**
 * S3 Storage API module.
 * Handles S3 storage configurations and S3-based backup management.
 */

import { apiRequest } from "./core";
import type {
  S3StorageConfig,
  S3StorageConfigResponse,
  CreateS3StorageConfigRequest,
  UpdateS3StorageConfigRequest,
  S3BackupResponse,
  TriggerS3BackupRequest,
  S3TestConnectionResult,
} from "@/types/api";

export const s3Api = {
  // -------------------------------------------------------------------------
  // S3 Storage Configs
  // -------------------------------------------------------------------------

  /** Create a new S3 storage configuration */
  createConfig: (config: CreateS3StorageConfigRequest, token?: string) =>
    apiRequest<S3StorageConfigResponse>("/s3/configs", {
      method: "POST",
      body: JSON.stringify(config),
    }, token),

  /** List all S3 storage configurations */
  listConfigs: (options: { reveal?: boolean; teamId?: string } = {}, token?: string) => {
    const params = new URLSearchParams();
    if (options.reveal) params.append("reveal", "true");
    if (options.teamId) params.append("team_id", options.teamId);
    const queryString = params.toString();
    return apiRequest<S3StorageConfigResponse[]>(
      `/s3/configs${queryString ? `?${queryString}` : ""}`,
      {},
      token
    );
  },

  /** Update an S3 storage configuration */
  updateConfig: (id: string, config: UpdateS3StorageConfigRequest, token?: string) =>
    apiRequest<S3StorageConfigResponse>(`/s3/configs/${id}`, {
      method: "PUT",
      body: JSON.stringify(config),
    }, token),

  /** Delete an S3 storage configuration */
  deleteConfig: (id: string, token?: string) =>
    apiRequest<{ message: string }>(`/s3/configs/${id}`, {
      method: "DELETE",
    }, token),

  /** Test an S3 storage configuration's connection */
  testConfig: (id: string, token?: string) =>
    apiRequest<S3TestConnectionResult>(`/s3/configs/${id}/test`, {
      method: "POST",
    }, token),

  // -------------------------------------------------------------------------
  // S3 Backups
  // -------------------------------------------------------------------------

  /** Trigger a backup to S3 */
  triggerBackup: (request: TriggerS3BackupRequest, token?: string) =>
    apiRequest<S3BackupResponse>("/s3/backup", {
      method: "POST",
      body: JSON.stringify(request),
    }, token),

  /** List all S3 backups */
  listBackups: (options: { teamId?: string } = {}, token?: string) => {
    const params = new URLSearchParams();
    if (options.teamId) params.append("team_id", options.teamId);
    const queryString = params.toString();
    return apiRequest<S3BackupResponse[]>(
      `/s3/backups${queryString ? `?${queryString}` : ""}`,
      {},
      token
    );
  },

  /** Restore from an S3 backup */
  restoreBackup: (id: string, token?: string) =>
    apiRequest<{
      message: string;
      database_restored: boolean;
      config_restored: boolean;
      certs_restored: boolean;
      warnings: string[];
    }>(`/s3/backups/${id}/restore`, {
      method: "POST",
    }, token),

  /** Delete an S3 backup */
  deleteBackup: (id: string, token?: string) =>
    apiRequest<{ message: string }>(`/s3/backups/${id}`, {
      method: "DELETE",
    }, token),
};
