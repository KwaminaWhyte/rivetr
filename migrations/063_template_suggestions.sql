-- Migration 063: Add template_suggestions table for community template submissions
CREATE TABLE IF NOT EXISTS template_suggestions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    docker_image TEXT NOT NULL,
    category TEXT NOT NULL,
    website_url TEXT,
    notes TEXT,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'approved', 'rejected')),
    submitted_by TEXT,
    reviewed_by TEXT,
    reviewed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
