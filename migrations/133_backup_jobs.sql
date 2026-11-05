-- Migration 133 (part 2): scheduled volume backups, backup history and restore jobs.
-- Idempotent: CREATE ... IF NOT EXISTS only. Column additions to existing tables
-- (s3_backups.checksum_sha256, database_backup_schedules.s3_config_id) are guarded
-- in src/db/mod.rs.

-- One schedule per volume. destination is 'local' or 's3' (then s3_config_id is set).
-- snapshot_mode: 'live' tars the volume read-only from a helper container while the
-- app keeps running; 'stop' stops the app's containers for the duration of the tar.
CREATE TABLE IF NOT EXISTS volume_backup_schedules (
    id TEXT PRIMARY KEY NOT NULL,
    volume_id TEXT NOT NULL UNIQUE REFERENCES volumes(id) ON DELETE CASCADE,
    enabled INTEGER NOT NULL DEFAULT 1,
    cron_expression TEXT NOT NULL,
    retention_count INTEGER NOT NULL DEFAULT 7,
    destination TEXT NOT NULL DEFAULT 'local' CHECK(destination IN ('local', 's3')),
    s3_config_id TEXT REFERENCES s3_storage_configs(id) ON DELETE SET NULL,
    snapshot_mode TEXT NOT NULL DEFAULT 'live' CHECK(snapshot_mode IN ('live', 'stop')),
    last_run_at TEXT,
    next_run_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_volume_backup_schedules_next_run ON volume_backup_schedules(next_run_at);

-- History of volume backup runs. A run to S3 also gets an s3_backups row
-- (s3_backup_id); the local archive is removed after a successful upload.
CREATE TABLE IF NOT EXISTS volume_backups (
    id TEXT PRIMARY KEY NOT NULL,
    volume_id TEXT NOT NULL REFERENCES volumes(id) ON DELETE CASCADE,
    trigger TEXT NOT NULL DEFAULT 'manual' CHECK(trigger IN ('manual', 'scheduled', 'pre_restore')),
    destination TEXT NOT NULL DEFAULT 'local' CHECK(destination IN ('local', 's3')),
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'running', 'completed', 'failed')),
    file_path TEXT,
    s3_backup_id TEXT,
    size_bytes INTEGER,
    checksum_sha256 TEXT,
    snapshot_mode TEXT NOT NULL DEFAULT 'live',
    started_at TEXT,
    completed_at TEXT,
    duration_ms INTEGER,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_volume_backups_volume_id ON volume_backups(volume_id, created_at);

-- Restore jobs for databases and volumes, local or from S3. Restores run in the
-- background; the dashboard polls this table.
CREATE TABLE IF NOT EXISTS backup_restores (
    id TEXT PRIMARY KEY NOT NULL,
    resource_type TEXT NOT NULL CHECK(resource_type IN ('database', 'volume', 'instance')),
    resource_id TEXT NOT NULL,
    source TEXT NOT NULL CHECK(source IN ('local', 's3')),
    backup_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running', 'completed', 'failed')),
    safety_backup_id TEXT,
    message TEXT,
    error_message TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_backup_restores_resource ON backup_restores(resource_type, resource_id, started_at);

-- One row per missed scheduled run that was already reported, so the periodic
-- check notifies once per missed run.
CREATE TABLE IF NOT EXISTS backup_missed_alerts (
    schedule_kind TEXT NOT NULL,
    schedule_id TEXT NOT NULL,
    expected_at TEXT NOT NULL,
    notified_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (schedule_kind, schedule_id, expected_at)
);
