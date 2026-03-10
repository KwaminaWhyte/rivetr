CREATE TABLE IF NOT EXISTS swarm_nodes (
    id TEXT PRIMARY KEY,
    node_id TEXT NOT NULL,          -- Docker Swarm node ID
    hostname TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('manager', 'worker')),
    status TEXT NOT NULL DEFAULT 'unknown' CHECK(status IN ('ready', 'down', 'disconnected', 'unknown')),
    availability TEXT NOT NULL DEFAULT 'active' CHECK(availability IN ('active', 'pause', 'drain')),
    cpu_count INTEGER,
    memory_bytes INTEGER,
    docker_version TEXT,
    ip_address TEXT,
    join_token TEXT,                 -- encrypted worker join token
    last_seen_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS swarm_services (
    id TEXT PRIMARY KEY,
    app_id TEXT REFERENCES apps(id) ON DELETE CASCADE,
    service_name TEXT NOT NULL,      -- Docker service name in swarm
    service_id TEXT,                 -- Docker Swarm service ID
    replicas INTEGER NOT NULL DEFAULT 1,
    mode TEXT NOT NULL DEFAULT 'replicated' CHECK(mode IN ('replicated', 'global')),
    image TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'running', 'failed', 'stopped')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Track swarm config
CREATE TABLE IF NOT EXISTS swarm_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
