-- Migration 125 (idempotent part): migrate the legacy healthcheck path into the
-- structured config, and create the clone job table.
UPDATE apps
SET healthcheck_config = json_object(
    'enabled', json('true'),
    'check_type', 'http',
    'method', 'GET',
    'scheme', 'http',
    'host', 'localhost',
    'port', NULL,
    'path', healthcheck,
    'expected_status', 200,
    'expected_body', NULL,
    'command', NULL,
    'interval_secs', 5,
    'timeout_secs', 5,
    'retries', 10,
    'start_period_secs', 5
)
WHERE healthcheck_config IS NULL
  AND healthcheck IS NOT NULL
  AND TRIM(healthcheck) != '';

CREATE TABLE IF NOT EXISTS resource_clone_jobs (
    id TEXT PRIMARY KEY,
    resource_type TEXT NOT NULL,
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    clone_volume_data INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'pending',
    progress INTEGER NOT NULL DEFAULT 0,
    log TEXT NOT NULL DEFAULT '',
    error TEXT,
    created_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_resource_clone_jobs_target ON resource_clone_jobs(target_id);
