-- Add is_secret and updated_at columns to env_vars table
ALTER TABLE env_vars ADD COLUMN is_secret INTEGER NOT NULL DEFAULT 0;
ALTER TABLE env_vars ADD COLUMN updated_at TEXT NOT NULL DEFAULT (datetime('now'));

-- Create index for faster lookups by app_id and is_secret
CREATE INDEX IF NOT EXISTS idx_env_vars_app_secret ON env_vars(app_id, is_secret);
