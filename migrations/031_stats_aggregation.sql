-- Stats aggregation tables for efficient historical queries
-- Stores hourly and daily aggregated metrics with automatic retention

-- Hourly aggregated stats (keeps 30 days of hourly data)
CREATE TABLE IF NOT EXISTS stats_hourly (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    hour_timestamp TEXT NOT NULL,  -- ISO 8601 format, truncated to hour
    avg_cpu_percent REAL NOT NULL DEFAULT 0,
    max_cpu_percent REAL NOT NULL DEFAULT 0,
    min_cpu_percent REAL NOT NULL DEFAULT 0,
    avg_memory_used_bytes INTEGER NOT NULL DEFAULT 0,
    max_memory_used_bytes INTEGER NOT NULL DEFAULT 0,
    avg_memory_total_bytes INTEGER NOT NULL DEFAULT 0,
    avg_running_apps REAL NOT NULL DEFAULT 0,
    avg_running_databases REAL NOT NULL DEFAULT 0,
    avg_running_services REAL NOT NULL DEFAULT 0,
    sample_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Index for efficient time-based queries on hourly stats
CREATE INDEX IF NOT EXISTS idx_stats_hourly_timestamp ON stats_hourly(hour_timestamp);

-- Unique constraint to prevent duplicate hourly records
CREATE UNIQUE INDEX IF NOT EXISTS idx_stats_hourly_unique ON stats_hourly(hour_timestamp);

-- Daily aggregated stats (keeps 365 days of daily data)
CREATE TABLE IF NOT EXISTS stats_daily (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    day_timestamp TEXT NOT NULL,  -- ISO 8601 format, date only (YYYY-MM-DD)
    avg_cpu_percent REAL NOT NULL DEFAULT 0,
    max_cpu_percent REAL NOT NULL DEFAULT 0,
    min_cpu_percent REAL NOT NULL DEFAULT 0,
    avg_memory_used_bytes INTEGER NOT NULL DEFAULT 0,
    max_memory_used_bytes INTEGER NOT NULL DEFAULT 0,
    avg_memory_total_bytes INTEGER NOT NULL DEFAULT 0,
    avg_running_apps REAL NOT NULL DEFAULT 0,
    max_running_apps INTEGER NOT NULL DEFAULT 0,
    avg_running_databases REAL NOT NULL DEFAULT 0,
    max_running_databases INTEGER NOT NULL DEFAULT 0,
    avg_running_services REAL NOT NULL DEFAULT 0,
    max_running_services INTEGER NOT NULL DEFAULT 0,
    sample_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Index for efficient time-based queries on daily stats
CREATE INDEX IF NOT EXISTS idx_stats_daily_timestamp ON stats_daily(day_timestamp);

-- Unique constraint to prevent duplicate daily records
CREATE UNIQUE INDEX IF NOT EXISTS idx_stats_daily_unique ON stats_daily(day_timestamp);

-- Add index on stats_history for efficient aggregation queries
CREATE INDEX IF NOT EXISTS idx_stats_history_timestamp_cpu ON stats_history(timestamp, cpu_percent);
