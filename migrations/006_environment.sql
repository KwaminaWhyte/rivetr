-- Add environment field to apps table
-- Valid values: 'development', 'staging', 'production'
-- Default: 'development'

ALTER TABLE apps ADD COLUMN environment TEXT NOT NULL DEFAULT 'development';

-- Add index for filtering by environment
CREATE INDEX IF NOT EXISTS idx_apps_environment ON apps(environment);
