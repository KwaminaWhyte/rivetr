// -------------------------------------------------------------------------
// Backup catalog, restore jobs and volume backup schedules
// -------------------------------------------------------------------------

/** Where a backup lives */
export type BackupSource = "local" | "s3";

/** One backup in a database or volume catalog (local and S3 together) */
export interface BackupCatalogEntry {
  /** Local backup id or S3 backup id, depending on `source` */
  id: string;
  source: BackupSource;
  /** manual, scheduled, pre_deploy, pre_restore (null for objects found in a bucket scan) */
  trigger: string | null;
  status: string;
  size_bytes: number | null;
  size_human: string | null;
  duration_ms: number | null;
  created_at: string;
  error_message: string | null;
  file_name: string | null;
  storage_config_id: string | null;
  storage_config_name: string | null;
  s3_key: string | null;
  checksum_sha256: string | null;
  snapshot_mode: string | null;
}

export interface BackupCatalogResponse {
  entries: BackupCatalogEntry[];
  /** Objects found by a bucket scan and added to the catalog */
  imported: number;
}

/** A restore job (runs in the background) */
export interface RestoreJob {
  id: string;
  resource_type: "database" | "volume" | "instance";
  resource_id: string;
  source: BackupSource;
  backup_id: string;
  status: "running" | "completed" | "failed";
  safety_backup_id: string | null;
  message: string | null;
  error_message: string | null;
  started_at: string;
  completed_at: string | null;
}

export type VolumeSnapshotMode = "live" | "stop";

/** A volume backup run */
export interface VolumeBackup {
  id: string;
  volume_id: string;
  trigger: string;
  destination: BackupSource;
  status: string;
  file_path: string | null;
  s3_backup_id: string | null;
  size_bytes: number | null;
  checksum_sha256: string | null;
  snapshot_mode: VolumeSnapshotMode;
  started_at: string | null;
  completed_at: string | null;
  duration_ms: number | null;
  error_message: string | null;
  created_at: string;
}

/** Per-volume backup schedule */
export interface VolumeBackupSchedule {
  id: string;
  volume_id: string;
  enabled: number;
  cron_expression: string;
  retention_count: number;
  destination: BackupSource;
  s3_config_id: string | null;
  snapshot_mode: VolumeSnapshotMode;
  last_run_at: string | null;
  next_run_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface UpsertVolumeBackupScheduleRequest {
  enabled: boolean;
  cron_expression: string;
  retention_count: number;
  destination: BackupSource;
  s3_config_id?: string | null;
  snapshot_mode: VolumeSnapshotMode;
}

export interface CreateVolumeBackupRequest {
  destination: BackupSource;
  s3_config_id?: string;
  snapshot_mode?: VolumeSnapshotMode;
}
