-- Alert configurations for per-app and global threshold alerts
-- Allows setting thresholds for CPU, memory, and disk usage alerts

-- Per-app alert configurations
-- When app_id is NULL, it's a global default
CREATE TABLE IF NOT EXISTS alert_configs (
    id TEXT PRIMARY KEY,
    app_id TEXT,  -- NULL for global defaults (deprecated, use global_alert_defaults instead)
    metric_type TEXT NOT NULL CHECK(metric_type IN ('cpu', 'memory', 'disk')),
    threshold_percent REAL NOT NULL CHECK(threshold_percent > 0 AND threshold_percent <= 100),
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (app_id) REFERENCES apps(id) ON DELETE CASCADE
);

-- Index for efficient queries by app_id
CREATE INDEX IF NOT EXISTS idx_alert_configs_app_id ON alert_configs(app_id);

-- Index for finding enabled alerts by metric type
CREATE INDEX IF NOT EXISTS idx_alert_configs_metric_enabled ON alert_configs(metric_type, enabled);

-- Unique constraint: one config per app per metric type
CREATE UNIQUE INDEX IF NOT EXISTS idx_alert_configs_unique ON alert_configs(app_id, metric_type) WHERE app_id IS NOT NULL;

-- Global alert default configurations
-- System-wide defaults that apply when no per-app config exists
CREATE TABLE IF NOT EXISTS global_alert_defaults (
    id TEXT PRIMARY KEY,
    metric_type TEXT NOT NULL UNIQUE CHECK(metric_type IN ('cpu', 'memory', 'disk')),
    threshold_percent REAL NOT NULL CHECK(threshold_percent > 0 AND threshold_percent <= 100),
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Seed default thresholds (80% for CPU, 85% for memory, 90% for disk)
INSERT OR IGNORE INTO global_alert_defaults (id, metric_type, threshold_percent, enabled)
VALUES
    ('default-cpu', 'cpu', 80.0, 1),
    ('default-memory', 'memory', 85.0, 1),
    ('default-disk', 'disk', 90.0, 1);
