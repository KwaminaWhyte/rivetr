-- Migration 036: Add app_shares table for sharing apps between teams
-- This allows teams to grant other teams view-only access to their apps

CREATE TABLE IF NOT EXISTS app_shares (
    id TEXT PRIMARY KEY,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    shared_with_team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    permission TEXT NOT NULL DEFAULT 'view',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_by TEXT REFERENCES users(id) ON DELETE SET NULL
);

-- Index for fast lookups by app
CREATE INDEX IF NOT EXISTS idx_app_shares_app_id ON app_shares(app_id);

-- Index for fast lookups by team
CREATE INDEX IF NOT EXISTS idx_app_shares_team_id ON app_shares(shared_with_team_id);

-- Unique constraint: can only share once per app-team combination
CREATE UNIQUE INDEX IF NOT EXISTS idx_app_shares_unique ON app_shares(app_id, shared_with_team_id);
