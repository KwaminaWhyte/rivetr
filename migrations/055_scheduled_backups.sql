CREATE TABLE IF NOT EXISTS backup_schedules (
    id TEXT PRIMARY KEY,
    backup_type TEXT NOT NULL CHECK(backup_type IN ('instance', 's3_database', 's3_volume')),
    cron_expression TEXT NOT NULL,  -- e.g. "0 2 * * *" = 2am daily
    target_id TEXT,                  -- database_id or volume name (null for instance backups)
    s3_config_id TEXT REFERENCES s3_storage_configs(id) ON DELETE SET NULL,
    retention_days INTEGER NOT NULL DEFAULT 30,
    enabled INTEGER NOT NULL DEFAULT 1,
    last_run_at TEXT,
    next_run_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
