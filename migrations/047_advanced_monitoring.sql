-- Log retention policies per app
CREATE TABLE IF NOT EXISTS log_retention_policies (
  id TEXT PRIMARY KEY,
  app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
  retention_days INTEGER NOT NULL DEFAULT 30,
  max_size_mb INTEGER DEFAULT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE(app_id)
);

-- Uptime tracking
CREATE TABLE IF NOT EXISTS uptime_checks (
  id TEXT PRIMARY KEY,
  app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
  status TEXT NOT NULL CHECK(status IN ('up', 'down', 'degraded')),
  response_time_ms INTEGER,
  status_code INTEGER,
  error_message TEXT,
  checked_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_uptime_checks_app_id ON uptime_checks(app_id);
CREATE INDEX IF NOT EXISTS idx_uptime_checks_checked_at ON uptime_checks(checked_at);

-- Scheduled container restarts
CREATE TABLE IF NOT EXISTS scheduled_restarts (
  id TEXT PRIMARY KEY,
  app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
  cron_expression TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  last_restart TEXT,
  next_restart TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
