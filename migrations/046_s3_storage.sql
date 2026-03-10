-- S3 storage configuration for remote backups
CREATE TABLE IF NOT EXISTS s3_storage_configs (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  endpoint TEXT, -- NULL = AWS default
  bucket TEXT NOT NULL,
  region TEXT NOT NULL DEFAULT 'us-east-1',
  access_key TEXT NOT NULL, -- encrypted
  secret_key TEXT NOT NULL, -- encrypted
  path_prefix TEXT DEFAULT '',
  is_default INTEGER NOT NULL DEFAULT 0,
  team_id TEXT REFERENCES teams(id),
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- S3 backup records
CREATE TABLE IF NOT EXISTS s3_backups (
  id TEXT PRIMARY KEY,
  storage_config_id TEXT NOT NULL REFERENCES s3_storage_configs(id),
  backup_type TEXT NOT NULL CHECK(backup_type IN ('instance', 'database', 'volume')),
  source_id TEXT, -- app_id or database_id
  s3_key TEXT NOT NULL,
  size_bytes INTEGER,
  status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'uploading', 'completed', 'failed')),
  error_message TEXT,
  team_id TEXT REFERENCES teams(id),
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
