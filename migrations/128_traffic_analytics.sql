-- Migration 128: traffic analytics rollups.
--
-- The proxy sends request events to an in-memory aggregator that flushes every
-- 60 seconds into these tables. Nothing here stores raw IPs: visitors are a
-- truncated SHA-256 of (daily salt, IP, user agent) and the salt is deleted
-- after a day, so hashes cannot be linked across days.
--
-- `server_id` is a row id from `servers` or 'localhost'. Idempotent.

-- Per-minute counters per host. l0..l11 are latency histogram bins
-- (upper bounds 10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000, 30000 ms, then overflow).
CREATE TABLE IF NOT EXISTS traffic_minute (
    minute INTEGER NOT NULL,
    host TEXT NOT NULL,
    app_id TEXT,
    server_id TEXT NOT NULL DEFAULT 'localhost',
    requests INTEGER NOT NULL DEFAULT 0,
    status_2xx INTEGER NOT NULL DEFAULT 0,
    status_3xx INTEGER NOT NULL DEFAULT 0,
    status_4xx INTEGER NOT NULL DEFAULT 0,
    status_5xx INTEGER NOT NULL DEFAULT 0,
    bytes_in INTEGER NOT NULL DEFAULT 0,
    bytes_out INTEGER NOT NULL DEFAULT 0,
    l0 INTEGER NOT NULL DEFAULT 0,
    l1 INTEGER NOT NULL DEFAULT 0,
    l2 INTEGER NOT NULL DEFAULT 0,
    l3 INTEGER NOT NULL DEFAULT 0,
    l4 INTEGER NOT NULL DEFAULT 0,
    l5 INTEGER NOT NULL DEFAULT 0,
    l6 INTEGER NOT NULL DEFAULT 0,
    l7 INTEGER NOT NULL DEFAULT 0,
    l8 INTEGER NOT NULL DEFAULT 0,
    l9 INTEGER NOT NULL DEFAULT 0,
    l10 INTEGER NOT NULL DEFAULT 0,
    l11 INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (minute, host)
);

CREATE INDEX IF NOT EXISTS idx_traffic_minute_app ON traffic_minute (app_id, minute);
CREATE INDEX IF NOT EXISTS idx_traffic_minute_server ON traffic_minute (server_id, minute);

-- Hourly top-N breakdowns: dim is 'path' | 'referrer' | 'ua' | 'country'.
CREATE TABLE IF NOT EXISTS traffic_dimensions (
    hour INTEGER NOT NULL,
    host TEXT NOT NULL,
    app_id TEXT,
    server_id TEXT NOT NULL DEFAULT 'localhost',
    dim TEXT NOT NULL,
    value TEXT NOT NULL,
    count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (hour, host, dim, value)
);

CREATE INDEX IF NOT EXISTS idx_traffic_dimensions_app ON traffic_dimensions (app_id, hour);
CREATE INDEX IF NOT EXISTS idx_traffic_dimensions_server ON traffic_dimensions (server_id, hour);

-- Daily unique visitors (salted hashes, see above). day is 'YYYY-MM-DD' UTC.
CREATE TABLE IF NOT EXISTS traffic_visitors (
    day TEXT NOT NULL,
    host TEXT NOT NULL,
    app_id TEXT,
    server_id TEXT NOT NULL DEFAULT 'localhost',
    visitor TEXT NOT NULL,
    PRIMARY KEY (day, host, visitor)
);

CREATE INDEX IF NOT EXISTS idx_traffic_visitors_app ON traffic_visitors (app_id, day);

CREATE TABLE IF NOT EXISTS traffic_salts (
    day TEXT PRIMARY KEY,
    salt TEXT NOT NULL
);

-- Per-server "Enable traffic analytics". No row = default (localhost on, remote off).
CREATE TABLE IF NOT EXISTS traffic_analytics_servers (
    server_id TEXT PRIMARY KEY,
    enabled INTEGER NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO instance_settings (key, value, updated_at)
VALUES ('analytics_retention_days', '30', datetime('now'));
