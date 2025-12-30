-- Stats history for dashboard charts
-- Stores periodic snapshots of system resource usage

CREATE TABLE IF NOT EXISTS stats_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    cpu_percent REAL NOT NULL DEFAULT 0,
    memory_used_bytes INTEGER NOT NULL DEFAULT 0,
    memory_total_bytes INTEGER NOT NULL DEFAULT 0,
    running_apps INTEGER NOT NULL DEFAULT 0,
    running_databases INTEGER NOT NULL DEFAULT 0,
    running_services INTEGER NOT NULL DEFAULT 0
);

-- Index for efficient time-based queries
CREATE INDEX IF NOT EXISTS idx_stats_history_timestamp ON stats_history(timestamp);

-- Clean up old stats (keep last 7 days) - will be done by cleanup task
