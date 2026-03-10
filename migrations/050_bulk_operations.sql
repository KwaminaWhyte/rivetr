-- Config snapshots for saving/restoring app configurations
CREATE TABLE IF NOT EXISTS config_snapshots (
  id TEXT PRIMARY KEY,
  app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  description TEXT,
  config_json TEXT NOT NULL,
  env_vars_json TEXT NOT NULL,
  created_by TEXT REFERENCES users(id),
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Maintenance mode columns on apps
ALTER TABLE apps ADD COLUMN maintenance_mode INTEGER NOT NULL DEFAULT 0;
ALTER TABLE apps ADD COLUMN maintenance_message TEXT;
