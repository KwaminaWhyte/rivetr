-- Migration 032: Add team_id to databases table for team scoping

-- Add team_id column (nullable for backward compatibility)
ALTER TABLE databases ADD COLUMN team_id TEXT REFERENCES teams(id) ON DELETE SET NULL;

-- Create index for faster lookups
CREATE INDEX IF NOT EXISTS idx_databases_team_id ON databases(team_id);
