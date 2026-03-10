-- Add preferred server to apps
ALTER TABLE apps ADD COLUMN server_id TEXT REFERENCES servers(id) ON DELETE SET NULL;
