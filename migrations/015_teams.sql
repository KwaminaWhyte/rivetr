-- Teams table for multi-user support with role-based access control
CREATE TABLE IF NOT EXISTS teams (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

-- Team members table with role-based access
CREATE TABLE IF NOT EXISTS team_members (
    id TEXT PRIMARY KEY,
    team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK(role IN ('owner', 'admin', 'developer', 'viewer')),
    created_at TEXT DEFAULT (datetime('now')),
    UNIQUE(team_id, user_id)
);

-- Add team_id to apps table (nullable for backward compatibility)
ALTER TABLE apps ADD COLUMN team_id TEXT REFERENCES teams(id) ON DELETE SET NULL;

-- Add team_id to projects table (nullable for backward compatibility)
ALTER TABLE projects ADD COLUMN team_id TEXT REFERENCES teams(id) ON DELETE SET NULL;

-- Create indexes for faster lookups
CREATE INDEX IF NOT EXISTS idx_team_members_team_id ON team_members(team_id);
CREATE INDEX IF NOT EXISTS idx_team_members_user_id ON team_members(user_id);
CREATE INDEX IF NOT EXISTS idx_apps_team_id ON apps(team_id);
CREATE INDEX IF NOT EXISTS idx_projects_team_id ON projects(team_id);
CREATE INDEX IF NOT EXISTS idx_teams_slug ON teams(slug);
