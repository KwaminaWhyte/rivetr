-- SSH keys for private repository authentication

-- SSH keys table (can be global or per-app)
CREATE TABLE IF NOT EXISTS ssh_keys (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    private_key TEXT NOT NULL,
    public_key TEXT,
    app_id TEXT REFERENCES apps(id) ON DELETE CASCADE,
    is_global INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Add ssh_key_id to apps table to link an app to a specific SSH key
ALTER TABLE apps ADD COLUMN ssh_key_id TEXT REFERENCES ssh_keys(id) ON DELETE SET NULL;

-- Indexes
CREATE INDEX IF NOT EXISTS idx_ssh_keys_app_id ON ssh_keys(app_id);
CREATE INDEX IF NOT EXISTS idx_ssh_keys_is_global ON ssh_keys(is_global);
