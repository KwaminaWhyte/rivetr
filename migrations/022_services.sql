-- Migration 022: Docker Compose Services
-- Allows deploying multi-container applications from docker-compose.yml

CREATE TABLE IF NOT EXISTS services (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    -- Associated project (optional)
    project_id TEXT,
    -- The full docker-compose.yml content
    compose_content TEXT NOT NULL,
    -- Status: pending, running, stopped, failed
    status TEXT NOT NULL DEFAULT 'pending',
    -- Error message if status is 'failed'
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE SET NULL
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_services_status ON services(status);
CREATE INDEX IF NOT EXISTS idx_services_project_id ON services(project_id);
CREATE INDEX IF NOT EXISTS idx_services_name ON services(name);
