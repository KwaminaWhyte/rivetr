-- Team invitations table for email-based invitations
CREATE TABLE IF NOT EXISTS team_invitations (
    id TEXT PRIMARY KEY,
    team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('owner', 'admin', 'developer', 'viewer')),
    token TEXT NOT NULL UNIQUE,
    expires_at TEXT NOT NULL,
    accepted_at TEXT,
    created_by TEXT NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    created_at TEXT DEFAULT (datetime('now'))
);

-- Create indexes for faster lookups
CREATE INDEX IF NOT EXISTS idx_team_invitations_team_id ON team_invitations(team_id);
CREATE INDEX IF NOT EXISTS idx_team_invitations_token ON team_invitations(token);
CREATE INDEX IF NOT EXISTS idx_team_invitations_email ON team_invitations(email);
-- Prevent duplicate pending invitations for same email+team
CREATE UNIQUE INDEX IF NOT EXISTS idx_team_invitations_pending
    ON team_invitations(team_id, email) WHERE accepted_at IS NULL;
