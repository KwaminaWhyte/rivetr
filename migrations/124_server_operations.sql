-- Server operations: per-server maintenance settings, Docker cleanup runs and
-- package patch jobs. `server_id` is either a row id from `servers` or the
-- literal 'localhost' for the runtime Rivetr itself runs on, so there is no
-- foreign key; rows for deleted servers are removed by the API.

CREATE TABLE IF NOT EXISTS server_maintenance_settings (
    server_id TEXT PRIMARY KEY,
    -- Docker cleanup schedule
    cleanup_enabled INTEGER NOT NULL DEFAULT 0,
    cleanup_cron TEXT NOT NULL DEFAULT '0 3 * * *',
    -- 'schedule' (run on every schedule) or 'threshold' (only when disk usage is above cleanup_threshold_pct)
    cleanup_trigger TEXT NOT NULL DEFAULT 'schedule',
    cleanup_threshold_pct INTEGER NOT NULL DEFAULT 80,
    remove_unused_volumes INTEGER NOT NULL DEFAULT 0,
    remove_unused_networks INTEGER NOT NULL DEFAULT 0,
    remove_retained_images INTEGER NOT NULL DEFAULT 0,
    -- Package patching
    patch_check_enabled INTEGER NOT NULL DEFAULT 1,
    patch_check_cron TEXT NOT NULL DEFAULT '0 0 * * *',
    auto_security_updates INTEGER NOT NULL DEFAULT 0,
    maintenance_window TEXT NOT NULL DEFAULT '03:00',
    -- Cached result of the last package check (JSON, see PatchState)
    last_patch_check_at TEXT,
    patch_state TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS docker_cleanup_runs (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL,
    -- 'running' | 'success' | 'failed' | 'skipped'
    status TEXT NOT NULL DEFAULT 'running',
    -- 'manual' | 'schedule' | 'threshold'
    trigger_type TEXT NOT NULL DEFAULT 'manual',
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    finished_at TEXT,
    duration_ms INTEGER,
    freed_bytes INTEGER NOT NULL DEFAULT 0,
    log TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_docker_cleanup_runs_server
    ON docker_cleanup_runs(server_id, started_at DESC);

CREATE TABLE IF NOT EXISTS server_patch_jobs (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL,
    -- 'check' | 'update_package' | 'update_all' | 'reboot' | 'configure_auto_updates'
    kind TEXT NOT NULL,
    package TEXT,
    -- 'running' | 'success' | 'failed'
    status TEXT NOT NULL DEFAULT 'running',
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    finished_at TEXT,
    log TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_server_patch_jobs_server
    ON server_patch_jobs(server_id, started_at DESC);
