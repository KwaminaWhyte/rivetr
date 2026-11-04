-- Migration 122: environment variables for Docker Compose services.
-- Values are encrypted like app env vars. On every start/deploy Rivetr writes
-- them to a `.env` file next to the service's docker-compose.yml: compose
-- reads it for ${VAR} interpolation, and a compose service can load it with
-- `env_file: .env` (or Rivetr adds that to every service when
-- service_deploy_configs.inject_env_file is on). Services that reference an
-- external env_file path keep working unchanged.
CREATE TABLE IF NOT EXISTS service_env_vars (
    id TEXT PRIMARY KEY NOT NULL,
    service_id TEXT NOT NULL REFERENCES services(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    is_secret INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(service_id, key)
);

CREATE INDEX IF NOT EXISTS idx_service_env_vars_service_id ON service_env_vars(service_id);
