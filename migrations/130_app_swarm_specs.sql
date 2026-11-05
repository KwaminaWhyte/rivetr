-- Advanced Docker Swarm service spec per app: placement constraints and
-- preferences, update and rollback config, resource reservations and restart
-- policy. Stored as JSON (see SwarmSpecConfig in src/api/swarm_spec.rs) and
-- applied to every Swarm service bound to the app.

CREATE TABLE IF NOT EXISTS app_swarm_specs (
    app_id TEXT PRIMARY KEY REFERENCES apps(id) ON DELETE CASCADE,
    spec TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
