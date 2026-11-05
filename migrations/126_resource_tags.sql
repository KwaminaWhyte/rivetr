-- Migration 126: resource tags.
--
-- Tags belong to a team (`team_id`), or are global when `team_id` is NULL
-- (single-user installs and resources without a team). A tag can be attached to
-- apps, databases and services through `resource_tags`.
--
-- Idempotent: CREATE ... IF NOT EXISTS everywhere.

CREATE TABLE IF NOT EXISTS tags (
    id TEXT PRIMARY KEY,
    team_id TEXT,
    name TEXT NOT NULL,
    -- Hex color such as '#6366f1'
    color TEXT NOT NULL DEFAULT '#6366f1',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- One name per team (case-insensitive); global tags share the '' bucket.
CREATE UNIQUE INDEX IF NOT EXISTS idx_tags_team_name
    ON tags (COALESCE(team_id, ''), name COLLATE NOCASE);

CREATE TABLE IF NOT EXISTS resource_tags (
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    -- 'app' | 'database' | 'service'
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (tag_id, resource_type, resource_id)
);

CREATE INDEX IF NOT EXISTS idx_resource_tags_resource
    ON resource_tags (resource_type, resource_id);

-- Assignments whose resource was deleted are hidden by the API (it joins on
-- the resource tables) and swept here on every start. No triggers: the
-- migration runner splits statements on semicolons.
DELETE FROM resource_tags
WHERE (resource_type = 'app' AND resource_id NOT IN (SELECT id FROM apps))
   OR (resource_type = 'database' AND resource_id NOT IN (SELECT id FROM databases))
   OR (resource_type = 'service' AND resource_id NOT IN (SELECT id FROM services));
