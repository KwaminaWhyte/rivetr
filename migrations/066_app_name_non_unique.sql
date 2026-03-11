-- Remove the UNIQUE constraint from apps.name
-- App names are display labels; the UUID primary key is the real identity.
-- Users should be free to give apps any name they like, including duplicates.
--
-- SQLite does not support DROP CONSTRAINT, so we recreate the table.

PRAGMA foreign_keys = OFF;

CREATE TABLE apps_new (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    git_url TEXT NOT NULL,
    branch TEXT NOT NULL DEFAULT 'main',
    dockerfile TEXT NOT NULL DEFAULT './Dockerfile',
    domain TEXT,
    port INTEGER NOT NULL DEFAULT 3000,
    healthcheck TEXT DEFAULT '/health',
    memory_limit TEXT DEFAULT '256mb',
    cpu_limit TEXT DEFAULT '0.5',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    ssh_key_id TEXT REFERENCES ssh_keys(id) ON DELETE SET NULL,
    git_provider_id TEXT REFERENCES git_providers(id) ON DELETE SET NULL,
    environment TEXT NOT NULL DEFAULT 'development',
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    dockerfile_path TEXT DEFAULT NULL,
    base_directory TEXT DEFAULT NULL,
    build_target TEXT DEFAULT NULL,
    watch_paths TEXT DEFAULT NULL,
    custom_docker_options TEXT DEFAULT NULL,
    domains TEXT DEFAULT NULL,
    auto_subdomain TEXT DEFAULT NULL,
    port_mappings TEXT DEFAULT NULL,
    network_aliases TEXT DEFAULT NULL,
    extra_hosts TEXT DEFAULT NULL,
    basic_auth_enabled INTEGER NOT NULL DEFAULT 0,
    basic_auth_username TEXT DEFAULT NULL,
    basic_auth_password_hash TEXT DEFAULT NULL,
    pre_deploy_commands TEXT DEFAULT NULL,
    post_deploy_commands TEXT DEFAULT NULL,
    docker_image TEXT DEFAULT NULL,
    docker_image_tag TEXT DEFAULT 'latest',
    registry_url TEXT DEFAULT NULL,
    registry_username TEXT DEFAULT NULL,
    registry_password TEXT DEFAULT NULL,
    team_id TEXT REFERENCES teams(id) ON DELETE SET NULL,
    container_labels TEXT DEFAULT NULL,
    build_type TEXT DEFAULT 'dockerfile',
    nixpacks_config TEXT DEFAULT NULL,
    publish_directory TEXT DEFAULT NULL,
    preview_enabled INTEGER DEFAULT 0,
    github_app_installation_id TEXT DEFAULT NULL,
    deployment_source TEXT DEFAULT 'git',
    auto_rollback_enabled INTEGER NOT NULL DEFAULT 0,
    registry_push_enabled INTEGER NOT NULL DEFAULT 0,
    max_rollback_versions INTEGER NOT NULL DEFAULT 5,
    environment_id TEXT REFERENCES environments(id) ON DELETE SET NULL,
    require_approval INTEGER NOT NULL DEFAULT 0,
    maintenance_mode INTEGER NOT NULL DEFAULT 0,
    maintenance_message TEXT DEFAULT 'Service temporarily unavailable',
    replica_count INTEGER NOT NULL DEFAULT 1,
    server_id TEXT REFERENCES servers(id) ON DELETE SET NULL,
    build_server_id TEXT REFERENCES build_servers(id) ON DELETE SET NULL,
    rollback_retention_count INTEGER NOT NULL DEFAULT 10
);

INSERT INTO apps_new (
    id, name, git_url, branch, dockerfile, domain, port, healthcheck,
    memory_limit, cpu_limit, created_at, updated_at,
    ssh_key_id, git_provider_id, environment, project_id,
    dockerfile_path, base_directory, build_target, watch_paths, custom_docker_options,
    domains, auto_subdomain, port_mappings, network_aliases, extra_hosts,
    basic_auth_enabled, basic_auth_username, basic_auth_password_hash,
    pre_deploy_commands, post_deploy_commands,
    docker_image, docker_image_tag, registry_url, registry_username, registry_password,
    team_id, container_labels, build_type, nixpacks_config, publish_directory,
    preview_enabled, github_app_installation_id, deployment_source,
    auto_rollback_enabled, registry_push_enabled, max_rollback_versions,
    environment_id, require_approval, maintenance_mode, maintenance_message,
    replica_count, server_id, build_server_id, rollback_retention_count
)
SELECT
    id, name, git_url, branch, dockerfile, domain, port, healthcheck,
    memory_limit, cpu_limit, created_at, updated_at,
    ssh_key_id, git_provider_id, environment, project_id,
    dockerfile_path, base_directory, build_target, watch_paths, custom_docker_options,
    domains, auto_subdomain, port_mappings, network_aliases, extra_hosts,
    basic_auth_enabled, basic_auth_username, basic_auth_password_hash,
    pre_deploy_commands, post_deploy_commands,
    docker_image, docker_image_tag, registry_url, registry_username, registry_password,
    team_id, container_labels, build_type, nixpacks_config, publish_directory,
    preview_enabled, github_app_installation_id, deployment_source,
    auto_rollback_enabled, registry_push_enabled, max_rollback_versions,
    environment_id, require_approval, maintenance_mode, maintenance_message,
    replica_count, server_id, build_server_id, rollback_retention_count
FROM apps;

DROP TABLE apps;
ALTER TABLE apps_new RENAME TO apps;

CREATE INDEX IF NOT EXISTS idx_apps_git_provider_id ON apps(git_provider_id);
CREATE INDEX IF NOT EXISTS idx_apps_environment ON apps(environment);
CREATE INDEX IF NOT EXISTS idx_apps_project_id ON apps(project_id);
CREATE INDEX IF NOT EXISTS idx_apps_basic_auth ON apps(id, basic_auth_enabled);
CREATE INDEX IF NOT EXISTS idx_apps_team_id ON apps(team_id);
CREATE INDEX IF NOT EXISTS idx_apps_github_app_installation_id ON apps(github_app_installation_id);

PRAGMA foreign_keys = ON;
