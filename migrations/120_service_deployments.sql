-- Migration 120: deployment history for Docker Compose services.
-- A service deployment pulls (or builds) the service's images and runs
-- `docker compose up -d`. Each run is recorded with who started it, what
-- triggered it, its duration, and the image digests per compose service before
-- and after, so a later rollback can pin the exact images by digest.
CREATE TABLE IF NOT EXISTS service_deployments (
    id TEXT PRIMARY KEY NOT NULL,
    service_id TEXT NOT NULL REFERENCES services(id) ON DELETE CASCADE,
    -- manual | webhook | rollback | api
    trigger TEXT NOT NULL DEFAULT 'manual',
    triggered_by TEXT,
    triggered_by_name TEXT,
    -- pending | building | deploying | succeeded | failed
    status TEXT NOT NULL DEFAULT 'pending',
    commit_sha TEXT,
    commit_message TEXT,
    image_tag TEXT,
    -- JSON object: compose service name -> {image, image_id, digest}
    images_before TEXT,
    images_after TEXT,
    rollback_of TEXT,
    error_message TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    finished_at TEXT,
    duration_ms INTEGER
);

CREATE INDEX IF NOT EXISTS idx_service_deployments_service_id
    ON service_deployments(service_id, started_at);

CREATE TABLE IF NOT EXISTS service_deployment_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    deployment_id TEXT NOT NULL REFERENCES service_deployments(id) ON DELETE CASCADE,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    level TEXT NOT NULL DEFAULT 'info',
    message TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_service_deployment_logs_deployment_id
    ON service_deployment_logs(deployment_id);
