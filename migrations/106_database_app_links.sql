-- Links between managed databases and apps so that DB connection details
-- (DATABASE_URL, HOST, PORT, USER, PASSWORD, DB) are auto-injected as env vars
-- into the app container at deploy time.
CREATE TABLE IF NOT EXISTS database_app_links (
    id TEXT PRIMARY KEY,
    database_id TEXT NOT NULL REFERENCES databases(id) ON DELETE CASCADE,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    env_prefix TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(database_id, app_id)
);

CREATE INDEX IF NOT EXISTS idx_database_app_links_app_id ON database_app_links(app_id);
CREATE INDEX IF NOT EXISTS idx_database_app_links_database_id ON database_app_links(database_id);
