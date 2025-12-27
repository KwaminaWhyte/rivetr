-- Add HTTP Basic Auth fields to apps table
-- Allows protecting applications with username/password authentication

ALTER TABLE apps ADD COLUMN basic_auth_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE apps ADD COLUMN basic_auth_username TEXT DEFAULT NULL;
ALTER TABLE apps ADD COLUMN basic_auth_password_hash TEXT DEFAULT NULL;

-- Create index for faster lookups when proxying requests
CREATE INDEX IF NOT EXISTS idx_apps_basic_auth ON apps(id, basic_auth_enabled);
