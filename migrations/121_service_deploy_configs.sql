-- Migration 121: optional per-service deploy configuration.
-- A service without a row (or with source_type 'none') deploys by pulling its
-- images, exactly as before. With a source (a git repo + branch, or a path on
-- the server) Rivetr builds the listed images, tags them with the revision and
-- `latest`, pushes them to their registry, runs the pre-deploy hooks and then
-- `docker compose up -d`.
CREATE TABLE IF NOT EXISTS service_deploy_configs (
    service_id TEXT PRIMARY KEY NOT NULL REFERENCES services(id) ON DELETE CASCADE,
    -- none | git | path
    source_type TEXT NOT NULL DEFAULT 'none',
    git_url TEXT,
    branch TEXT NOT NULL DEFAULT 'main',
    github_app_installation_id TEXT,
    git_provider_id TEXT,
    -- Absolute path on the server (source_type 'path')
    source_path TEXT,
    -- File holding the revision for a path source that is not a git checkout
    revision_file TEXT,
    -- JSON array: [{name, image, dockerfile, context, build_args: {}, target}]
    images TEXT NOT NULL DEFAULT '[]',
    push_images INTEGER NOT NULL DEFAULT 1,
    registry_username TEXT,
    -- Encrypted like env vars
    registry_password TEXT,
    -- JSON arrays: [{name, service, command, timeout_secs}]
    pre_deploy_hooks TEXT NOT NULL DEFAULT '[]',
    post_deploy_hooks TEXT NOT NULL DEFAULT '[]',
    -- Deploy on a push to the branch (git source)
    auto_deploy INTEGER NOT NULL DEFAULT 0,
    -- JSON object: compose service -> image reference pinned by a rollback.
    -- Cleared by the next regular deployment.
    pinned_images TEXT,
    -- Add the Rivetr-managed .env file to every compose service's env_file
    inject_env_file INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_service_deploy_configs_auto
    ON service_deploy_configs(source_type, auto_deploy, branch);
