-- Git providers (GitHub, GitLab, Bitbucket OAuth connections)
CREATE TABLE IF NOT EXISTS git_providers (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider TEXT NOT NULL, -- 'github', 'gitlab', 'bitbucket'
    provider_user_id TEXT NOT NULL,
    username TEXT NOT NULL,
    display_name TEXT,
    email TEXT,
    avatar_url TEXT,
    access_token TEXT NOT NULL,
    refresh_token TEXT,
    token_expires_at TEXT,
    scopes TEXT, -- comma-separated list of granted scopes
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id, provider)
);

-- Add git_provider_id to apps for OAuth-based cloning
ALTER TABLE apps ADD COLUMN git_provider_id TEXT REFERENCES git_providers(id) ON DELETE SET NULL;

-- Indexes
CREATE INDEX IF NOT EXISTS idx_git_providers_user_id ON git_providers(user_id);
CREATE INDEX IF NOT EXISTS idx_git_providers_provider ON git_providers(provider);
CREATE INDEX IF NOT EXISTS idx_apps_git_provider_id ON apps(git_provider_id);
