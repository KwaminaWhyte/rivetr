CREATE TABLE IF NOT EXISTS servers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    host TEXT NOT NULL,
    port INTEGER NOT NULL DEFAULT 22,
    username TEXT NOT NULL DEFAULT 'root',
    ssh_private_key TEXT,         -- encrypted with AES-256-GCM
    status TEXT NOT NULL DEFAULT 'unknown' CHECK(status IN ('online', 'offline', 'unknown')),
    last_seen_at TEXT,
    cpu_usage REAL,
    memory_usage REAL,
    disk_usage REAL,
    memory_total INTEGER,
    disk_total INTEGER,
    os_info TEXT,
    docker_version TEXT,
    team_id TEXT REFERENCES teams(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS app_server_assignments (
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    server_id TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    PRIMARY KEY (app_id, server_id)
);
