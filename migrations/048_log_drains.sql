-- Log drains for forwarding container logs to external services
CREATE TABLE IF NOT EXISTS log_drains (
  id TEXT PRIMARY KEY,
  app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  provider TEXT NOT NULL CHECK(provider IN ('axiom', 'newrelic', 'http', 'datadog', 'logtail')),
  config TEXT NOT NULL, -- JSON: { endpoint, api_key, dataset, etc. }
  enabled INTEGER NOT NULL DEFAULT 1,
  last_sent_at TEXT,
  error_count INTEGER NOT NULL DEFAULT 0,
  last_error TEXT,
  team_id TEXT REFERENCES teams(id),
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_log_drains_app_id ON log_drains(app_id);
CREATE INDEX IF NOT EXISTS idx_log_drains_team_id ON log_drains(team_id);
