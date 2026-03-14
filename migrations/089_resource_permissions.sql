-- Per-member, per-resource permission overrides
CREATE TABLE IF NOT EXISTS team_resource_permissions (
    id TEXT PRIMARY KEY,
    team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    permission TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(team_id, user_id, resource_type, resource_id)
);
CREATE INDEX IF NOT EXISTS idx_trp_lookup ON team_resource_permissions(team_id, user_id, resource_type);
