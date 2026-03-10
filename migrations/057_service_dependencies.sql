-- Service dependency graph: tracks which apps depend on other apps, databases, or services
CREATE TABLE IF NOT EXISTS service_dependencies (
    id TEXT PRIMARY KEY,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    depends_on_app_id TEXT REFERENCES apps(id) ON DELETE CASCADE,
    depends_on_database_id TEXT REFERENCES databases(id) ON DELETE CASCADE,
    depends_on_service_id TEXT REFERENCES services(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK (
        (depends_on_app_id IS NOT NULL AND depends_on_database_id IS NULL AND depends_on_service_id IS NULL) OR
        (depends_on_app_id IS NULL AND depends_on_database_id IS NOT NULL AND depends_on_service_id IS NULL) OR
        (depends_on_app_id IS NULL AND depends_on_database_id IS NULL AND depends_on_service_id IS NOT NULL)
    )
);
