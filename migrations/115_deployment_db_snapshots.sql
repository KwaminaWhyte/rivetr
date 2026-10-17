-- Rollback with data: pre-deploy logical snapshots of an app's linked managed databases.
-- apps.snapshot_before_deploy opts an app in (default off).
ALTER TABLE apps ADD COLUMN snapshot_before_deploy INTEGER NOT NULL DEFAULT 0;

-- One row per (deployment, database). The dump itself is a normal database_backups row
-- (backup_type 'pre_deploy'); deleting that backup cascades the snapshot away.
-- deployment_id is the deployment that was about to start when the snapshot was taken.
-- source_deployment_id is the deployment that was running at that moment, i.e. the one
-- whose code last used the data; rolling back to it restores this snapshot.
-- No FK to deployments: deployment rows are trimmed independently, and the retention
-- cleaner removes snapshots (and their files) whose deployment is gone.
CREATE TABLE IF NOT EXISTS deployment_db_snapshots (
    id TEXT PRIMARY KEY,
    deployment_id TEXT NOT NULL,
    app_id TEXT NOT NULL,
    source_deployment_id TEXT,
    database_id TEXT NOT NULL REFERENCES databases(id) ON DELETE CASCADE,
    backup_id TEXT NOT NULL REFERENCES database_backups(id) ON DELETE CASCADE,
    size_bytes INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_deployment_db_snapshots_deployment ON deployment_db_snapshots(deployment_id);
CREATE INDEX IF NOT EXISTS idx_deployment_db_snapshots_source ON deployment_db_snapshots(source_deployment_id);
CREATE INDEX IF NOT EXISTS idx_deployment_db_snapshots_app ON deployment_db_snapshots(app_id);
