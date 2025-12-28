-- Migration 019: Managed databases for one-click database deployments
-- Supports PostgreSQL, MySQL, MongoDB, and Redis

CREATE TABLE IF NOT EXISTS databases (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    -- Database type: postgres, mysql, mongodb, redis
    db_type TEXT NOT NULL,
    -- Version tag (e.g., "16", "8.0", "7", "7.2")
    version TEXT NOT NULL,
    -- Container ID when running
    container_id TEXT,
    -- Status: pending, pulling, starting, running, stopped, failed
    status TEXT NOT NULL DEFAULT 'pending',
    -- Internal port (container port)
    internal_port INTEGER NOT NULL,
    -- External/host port (0 = not exposed publicly)
    external_port INTEGER NOT NULL DEFAULT 0,
    -- Whether to expose publicly (via host port binding)
    public_access INTEGER NOT NULL DEFAULT 0,
    -- Credentials (stored as JSON)
    -- Format: {"username": "...", "password": "...", "database": "...", "root_password": "..."}
    credentials TEXT NOT NULL,
    -- Volume name for data persistence
    volume_name TEXT,
    -- Host path for volume mount
    volume_path TEXT,
    -- Memory limit (e.g., "512mb", "1gb")
    memory_limit TEXT DEFAULT '512mb',
    -- CPU limit (e.g., "0.5", "1")
    cpu_limit TEXT DEFAULT '0.5',
    -- Error message if status is 'failed'
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_databases_status ON databases(status);
CREATE INDEX IF NOT EXISTS idx_databases_db_type ON databases(db_type);
CREATE INDEX IF NOT EXISTS idx_databases_name ON databases(name);
