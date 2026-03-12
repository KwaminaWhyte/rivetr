// -------------------------------------------------------------------------
// Managed Database types
// -------------------------------------------------------------------------

/** Supported database types for managed databases */
export type DatabaseType = "postgres" | "mysql" | "mongodb" | "redis";

/** Database deployment status */
export type DatabaseStatus =
  | "pending"
  | "pulling"
  | "starting"
  | "running"
  | "stopped"
  | "failed";

/** Database credentials */
export interface DatabaseCredentials {
  username: string;
  password: string;
  database?: string;
  root_password?: string;
}

/** Managed database response */
export interface ManagedDatabase {
  id: string;
  name: string;
  db_type: DatabaseType;
  version: string;
  container_id: string | null;
  container_slug: string | null;
  status: DatabaseStatus;
  internal_port: number;
  external_port: number;
  public_access: boolean;
  credentials: DatabaseCredentials;
  volume_name: string | null;
  volume_path: string | null;
  memory_limit: string | null;
  cpu_limit: string | null;
  internal_connection_string: string | null;
  external_connection_string: string | null;
  error_message: string | null;
  project_id: string | null;
  team_id: string | null;
  created_at: string;
  updated_at: string;
}

/** Request to create a managed database */
export interface CreateManagedDatabaseRequest {
  name: string;
  db_type: DatabaseType;
  version?: string;
  public_access?: boolean;
  /** Custom username (optional, auto-generated if not provided) */
  username?: string;
  /** Custom password (optional, auto-generated if not provided) */
  password?: string;
  /** Custom database name (optional, defaults to username) */
  database?: string;
  /** Custom root password for MySQL (optional, auto-generated if not provided) */
  root_password?: string;
  memory_limit?: string;
  cpu_limit?: string;
  project_id?: string;
  team_id?: string;
}

/** Request to update a managed database */
export interface UpdateManagedDatabaseRequest {
  /** Enable/disable public access (internet-accessible) */
  public_access?: boolean;
  /** Custom external port (0 = auto-assign, 1024-65535 = specific port) */
  external_port?: number;
  /** Memory limit (e.g., "512mb", "1g") */
  memory_limit?: string;
  /** CPU limit (e.g., "0.5", "1", "2") */
  cpu_limit?: string;
}

/** Database type configuration (for UI) */
export interface DatabaseTypeInfo {
  type: DatabaseType;
  name: string;
  description: string;
  defaultPort: number;
  versions: string[];
  defaultVersion: string;
}

/** Database log entry */
export interface DatabaseLogEntry {
  timestamp: string;
  message: string;
  stream: "stdout" | "stderr";
}

// -------------------------------------------------------------------------
// Database Backup types
// -------------------------------------------------------------------------

/** Database backup status */
export type BackupStatus = "pending" | "running" | "completed" | "failed";

/** Database backup type */
export type BackupType = "manual" | "scheduled";

/** Schedule type for backups */
export type ScheduleType = "hourly" | "daily" | "weekly";

/** Database backup record */
export interface DatabaseBackup {
  id: string;
  database_id: string;
  backup_type: BackupType;
  status: BackupStatus;
  file_path?: string;
  file_size?: number;
  file_size_human?: string;
  backup_format?: string;
  started_at?: string;
  completed_at?: string;
  duration_seconds?: number;
  error_message?: string;
  created_at: string;
}

/** Database backup schedule */
export interface DatabaseBackupSchedule {
  id: string;
  database_id: string;
  enabled: boolean;
  schedule_type: ScheduleType;
  schedule_hour: number;
  schedule_day?: number;
  retention_count: number;
  last_run_at?: string;
  next_run_at?: string;
  created_at: string;
}

/** Request to create/update backup schedule */
export interface CreateBackupScheduleRequest {
  enabled?: boolean;
  schedule_type?: ScheduleType;
  schedule_hour?: number;
  schedule_day?: number;
  retention_count?: number;
}

/** Available database configurations */
export const DATABASE_TYPES: DatabaseTypeInfo[] = [
  {
    type: "postgres",
    name: "PostgreSQL",
    description: "The world's most advanced open source relational database",
    defaultPort: 5432,
    versions: ["18", "17", "16", "15", "14", "13", "12"],
    defaultVersion: "17",
  },
  {
    type: "mysql",
    name: "MySQL",
    description: "The most popular open source relational database",
    defaultPort: 3306,
    versions: ["8.0", "8.4", "5.7"],
    defaultVersion: "8.0",
  },
  {
    type: "mongodb",
    name: "MongoDB",
    description: "A document-oriented NoSQL database",
    defaultPort: 27017,
    versions: ["7", "6", "5", "4.4"],
    defaultVersion: "7",
  },
  {
    type: "redis",
    name: "Redis",
    description: "In-memory data structure store for caching and messaging",
    defaultPort: 6379,
    versions: ["7", "7.2", "6", "6.2"],
    defaultVersion: "7",
  },
];
