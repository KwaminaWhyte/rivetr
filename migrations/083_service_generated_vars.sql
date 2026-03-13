-- Migration 083: Service Generated Variables
-- Stores auto-generated magic variables (SERVICE_PASSWORD_*, SERVICE_BASE64_*) for compose services
-- so they are stable across restarts (generated once, reused thereafter).

CREATE TABLE IF NOT EXISTS service_generated_vars (
    id TEXT PRIMARY KEY NOT NULL,
    service_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(service_id, key),
    FOREIGN KEY (service_id) REFERENCES services(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_service_generated_vars_service_id ON service_generated_vars(service_id);
