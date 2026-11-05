-- Migration 134: scoped API tokens and granular team RBAC (idempotent part).
-- The api_tokens column additions live in 134_scoped_tokens_columns.sql and run
-- only when the `scopes` column is missing (SQLite has no ADD COLUMN IF NOT EXISTS).

-- Custom team roles: a named set of abilities. Members reference them with
-- team_members.role = 'custom:<id>'. Preset roles (owner, admin, developer,
-- deployer, viewer) are defined in code; the legacy values owner/admin/developer/
-- viewer are preset keys, so existing team_members rows need no rewrite.
CREATE TABLE IF NOT EXISTS team_roles (
    id TEXT PRIMARY KEY,
    team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    abilities TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(team_id, name)
);
CREATE INDEX IF NOT EXISTS idx_team_roles_team ON team_roles(team_id);

-- Normalise legacy role values to lower case preset keys.
UPDATE team_members SET role = lower(role)
    WHERE role <> lower(role) AND lower(role) IN ('owner', 'admin', 'developer', 'viewer');
UPDATE team_invitations SET role = lower(role)
    WHERE role <> lower(role) AND lower(role) IN ('owner', 'admin', 'developer', 'viewer');
