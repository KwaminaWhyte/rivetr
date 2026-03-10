CREATE TABLE IF NOT EXISTS build_servers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    host TEXT NOT NULL,
    port INTEGER NOT NULL DEFAULT 22,
    username TEXT NOT NULL DEFAULT 'root',
    ssh_private_key TEXT,    -- encrypted
    status TEXT NOT NULL DEFAULT 'unknown' CHECK(status IN ('online', 'offline', 'unknown')),
    last_seen_at TEXT,
    docker_version TEXT,
    cpu_count INTEGER,
    memory_bytes INTEGER,
    concurrent_builds INTEGER NOT NULL DEFAULT 2,
    active_builds INTEGER NOT NULL DEFAULT 0,
    team_id TEXT REFERENCES teams(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS build_server_assignments (
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    build_server_id TEXT NOT NULL REFERENCES build_servers(id) ON DELETE CASCADE,
    PRIMARY KEY (app_id, build_server_id)
);

-- Add build_server_id to apps
ALTER TABLE apps ADD COLUMN build_server_id TEXT REFERENCES build_servers(id) ON DELETE SET NULL;
