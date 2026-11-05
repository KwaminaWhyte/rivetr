-- One app across several servers behind the Rivetr proxy (gap #11).
--
-- app_server_placements: where an app runs. One row per target server
-- ('localhost' is the Rivetr host) with the number of containers on it.
-- apps.server_id stays the primary server for backward compatibility; when an
-- app has no placement rows it deploys exactly as before.
--
-- app_multi_server_settings: rollout and load balancing options for the app.
--
-- deployment_targets: one row per container slot of a multi-server deployment,
-- with its own status, address and log, shown on the deployment detail page.
--
-- Idempotent: every statement uses IF NOT EXISTS.

CREATE TABLE IF NOT EXISTS app_server_placements (
    id TEXT PRIMARY KEY,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    server_id TEXT NOT NULL,
    replicas INTEGER NOT NULL DEFAULT 1,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (app_id, server_id)
);

CREATE INDEX IF NOT EXISTS idx_app_server_placements_app
    ON app_server_placements(app_id);

CREATE TABLE IF NOT EXISTS app_multi_server_settings (
    app_id TEXT PRIMARY KEY REFERENCES apps(id) ON DELETE CASCADE,
    -- Containers replaced per rolling batch
    max_unavailable INTEGER NOT NULL DEFAULT 1,
    -- Roll the whole deployment back when at least this share of targets fail
    failure_threshold_pct INTEGER NOT NULL DEFAULT 50,
    -- auto | registry | ssh
    distribution TEXT NOT NULL DEFAULT 'auto',
    -- round_robin | least_conn
    lb_policy TEXT NOT NULL DEFAULT 'round_robin',
    sticky_sessions INTEGER NOT NULL DEFAULT 0,
    -- Publish remote ports on the server's Tailscale address when it has one
    bind_private INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS deployment_targets (
    id TEXT PRIMARY KEY,
    deployment_id TEXT NOT NULL REFERENCES deployments(id) ON DELETE CASCADE,
    app_id TEXT NOT NULL,
    server_id TEXT NOT NULL,
    server_name TEXT NOT NULL DEFAULT '',
    replica_index INTEGER NOT NULL DEFAULT 0,
    batch INTEGER NOT NULL DEFAULT 0,
    -- pending | distributing | starting | checking | healthy | failed | rolled_back | replaced
    status TEXT NOT NULL DEFAULT 'pending',
    container_name TEXT,
    container_id TEXT,
    -- Address the proxy uses for this container ("host:port")
    upstream TEXT,
    error_message TEXT,
    log TEXT NOT NULL DEFAULT '',
    started_at TEXT,
    finished_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_deployment_targets_deployment
    ON deployment_targets(deployment_id);
CREATE INDEX IF NOT EXISTS idx_deployment_targets_app_status
    ON deployment_targets(app_id, status);
