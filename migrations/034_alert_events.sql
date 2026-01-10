-- Alert events table for tracking triggered alerts
-- Records when alerts fire and resolve with hysteresis tracking

-- Alert events track when thresholds are breached
CREATE TABLE IF NOT EXISTS alert_events (
    id TEXT PRIMARY KEY,
    app_id TEXT NOT NULL,
    metric_type TEXT NOT NULL CHECK(metric_type IN ('cpu', 'memory', 'disk')),
    threshold_percent REAL NOT NULL,
    current_value REAL NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('firing', 'resolved')) DEFAULT 'firing',
    -- Consecutive breach count for hysteresis (triggers at 2+)
    consecutive_breaches INTEGER NOT NULL DEFAULT 1,
    fired_at TEXT NOT NULL DEFAULT (datetime('now')),
    resolved_at TEXT,
    -- Last notification sent time to prevent duplicate alerts within 15-minute window
    last_notified_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (app_id) REFERENCES apps(id) ON DELETE CASCADE
);

-- Index for querying active alerts by app
CREATE INDEX IF NOT EXISTS idx_alert_events_app_status ON alert_events(app_id, status);

-- Index for querying by metric type and status
CREATE INDEX IF NOT EXISTS idx_alert_events_metric_status ON alert_events(metric_type, status);

-- Index for finding recent events by time
CREATE INDEX IF NOT EXISTS idx_alert_events_fired_at ON alert_events(fired_at);

-- Table to track consecutive breach counts per app/metric (for hysteresis)
CREATE TABLE IF NOT EXISTS alert_breach_counts (
    app_id TEXT NOT NULL,
    metric_type TEXT NOT NULL CHECK(metric_type IN ('cpu', 'memory', 'disk')),
    consecutive_count INTEGER NOT NULL DEFAULT 0,
    last_checked_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (app_id, metric_type),
    FOREIGN KEY (app_id) REFERENCES apps(id) ON DELETE CASCADE
);
