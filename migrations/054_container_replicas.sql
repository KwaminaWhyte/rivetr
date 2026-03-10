-- Add replica configuration to apps
ALTER TABLE apps ADD COLUMN replica_count INTEGER NOT NULL DEFAULT 1;

-- Track individual replica containers
CREATE TABLE IF NOT EXISTS app_replicas (
    id TEXT PRIMARY KEY,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    replica_index INTEGER NOT NULL,
    container_id TEXT,
    status TEXT NOT NULL DEFAULT 'stopped' CHECK(status IN ('running', 'stopped', 'error', 'starting')),
    started_at TEXT,
    stopped_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_app_replicas_app_id ON app_replicas(app_id);
