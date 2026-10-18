-- Migration 118: server-side store for GitHub App manifest registration state.
-- The callback used to trust an unsigned, client-supplied team_id / is_system_wide.
-- Now the values are recorded when the registration starts (by an authorized user)
-- and consumed once by the callback.
CREATE TABLE IF NOT EXISTS github_manifest_states (
    state TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    team_id TEXT,
    is_system_wide INTEGER NOT NULL DEFAULT 0,
    expires_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_github_manifest_states_expires ON github_manifest_states(expires_at);
