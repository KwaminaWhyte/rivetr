-- Initial schema for Rivetr

-- Apps table
CREATE TABLE IF NOT EXISTS apps (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    git_url TEXT NOT NULL,
    branch TEXT NOT NULL DEFAULT 'main',
    dockerfile TEXT NOT NULL DEFAULT './Dockerfile',
    domain TEXT,
    port INTEGER NOT NULL DEFAULT 3000,
    healthcheck TEXT DEFAULT '/health',
    memory_limit TEXT DEFAULT '256mb',
    cpu_limit TEXT DEFAULT '0.5',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Environment variables (encrypted values stored)
CREATE TABLE IF NOT EXISTS env_vars (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(app_id, key)
);

-- Deployments table
CREATE TABLE IF NOT EXISTS deployments (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    commit_sha TEXT,
    commit_message TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    container_id TEXT,
    error_message TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    finished_at TEXT
);

-- Deployment logs
CREATE TABLE IF NOT EXISTS deployment_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    deployment_id TEXT NOT NULL REFERENCES deployments(id) ON DELETE CASCADE,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    level TEXT NOT NULL DEFAULT 'info',
    message TEXT NOT NULL
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_deployments_app_id ON deployments(app_id);
CREATE INDEX IF NOT EXISTS idx_deployments_status ON deployments(status);
CREATE INDEX IF NOT EXISTS idx_deployment_logs_deployment_id ON deployment_logs(deployment_id);
CREATE INDEX IF NOT EXISTS idx_env_vars_app_id ON env_vars(app_id);
