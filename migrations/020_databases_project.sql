-- Migration 020: Add project_id to databases table
-- Links databases to projects like apps are

ALTER TABLE databases ADD COLUMN project_id TEXT REFERENCES projects(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_databases_project_id ON databases(project_id);
