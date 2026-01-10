-- Migration 033: Add team_id to services table
-- Allows services to be scoped to teams

ALTER TABLE services ADD COLUMN team_id TEXT REFERENCES teams(id) ON DELETE SET NULL;

-- Index for efficient team-based queries
CREATE INDEX IF NOT EXISTS idx_services_team_id ON services(team_id);
