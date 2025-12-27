-- Volumes table for persistent storage management
CREATE TABLE IF NOT EXISTS volumes (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    host_path TEXT NOT NULL,
    container_path TEXT NOT NULL,
    read_only INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(app_id, name),
    UNIQUE(app_id, container_path)
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_volumes_app_id ON volumes(app_id);
CREATE INDEX IF NOT EXISTS idx_volumes_name ON volumes(name);
