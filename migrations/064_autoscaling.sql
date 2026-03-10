-- Migration 064: Add autoscaling_rules table
CREATE TABLE IF NOT EXISTS autoscaling_rules (
    id TEXT PRIMARY KEY,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    metric TEXT NOT NULL CHECK(metric IN ('cpu', 'memory', 'request_rate')),
    scale_up_threshold REAL NOT NULL,
    scale_down_threshold REAL NOT NULL,
    min_replicas INTEGER NOT NULL DEFAULT 1,
    max_replicas INTEGER NOT NULL DEFAULT 10,
    cooldown_seconds INTEGER NOT NULL DEFAULT 300,
    enabled INTEGER NOT NULL DEFAULT 1,
    last_scaled_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
