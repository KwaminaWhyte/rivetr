-- Database backups table
CREATE TABLE IF NOT EXISTS database_backups (
    id TEXT PRIMARY KEY,
    database_id TEXT NOT NULL REFERENCES databases(id) ON DELETE CASCADE,
    backup_type TEXT NOT NULL DEFAULT 'manual',  -- 'manual' or 'scheduled'
    status TEXT NOT NULL DEFAULT 'pending',       -- pending, running, completed, failed
    file_path TEXT,                               -- path to backup file
    file_size INTEGER,                            -- size in bytes
    backup_format TEXT,                           -- sql, dump, archive, rdb
    started_at TEXT,
    completed_at TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Database backup schedules table
CREATE TABLE IF NOT EXISTS database_backup_schedules (
    id TEXT PRIMARY KEY,
    database_id TEXT NOT NULL UNIQUE REFERENCES databases(id) ON DELETE CASCADE,
    enabled INTEGER NOT NULL DEFAULT 1,
    schedule_type TEXT NOT NULL DEFAULT 'daily', -- hourly, daily, weekly
    schedule_hour INTEGER NOT NULL DEFAULT 2,    -- hour of day (0-23) for daily/weekly
    schedule_day INTEGER DEFAULT 0,              -- day of week (0-6, 0=Sunday) for weekly
    retention_count INTEGER NOT NULL DEFAULT 5,  -- number of backups to keep
    last_run_at TEXT,
    next_run_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_database_backups_database_id ON database_backups(database_id);
CREATE INDEX IF NOT EXISTS idx_database_backups_status ON database_backups(status);
CREATE INDEX IF NOT EXISTS idx_database_backups_created_at ON database_backups(created_at);
CREATE INDEX IF NOT EXISTS idx_database_backup_schedules_next_run ON database_backup_schedules(next_run_at);
CREATE INDEX IF NOT EXISTS idx_database_backup_schedules_enabled ON database_backup_schedules(enabled);
