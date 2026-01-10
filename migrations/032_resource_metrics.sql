-- Resource metrics for per-app resource usage tracking
-- Stores periodic CPU, memory, and disk metrics for each app/deployment

CREATE TABLE IF NOT EXISTS resource_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    app_id TEXT NOT NULL,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    cpu_percent REAL NOT NULL DEFAULT 0,
    memory_bytes INTEGER NOT NULL DEFAULT 0,
    memory_limit_bytes INTEGER NOT NULL DEFAULT 0,
    disk_bytes INTEGER NOT NULL DEFAULT 0,
    disk_limit_bytes INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (app_id) REFERENCES apps(id) ON DELETE CASCADE
);

-- Index for efficient time-based queries per app
CREATE INDEX IF NOT EXISTS idx_resource_metrics_app_timestamp ON resource_metrics(app_id, timestamp);

-- Index for cleanup queries (retention)
CREATE INDEX IF NOT EXISTS idx_resource_metrics_timestamp ON resource_metrics(timestamp);
