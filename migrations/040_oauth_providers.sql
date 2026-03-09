-- Migration 040: Add OAuth login providers and user OAuth connections
-- Stores configured OAuth providers (GitHub, Google) for social login
-- and links between OAuth accounts and Rivetr user accounts

CREATE TABLE IF NOT EXISTS oauth_providers (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL CHECK(provider IN ('github', 'google')),
    client_id TEXT NOT NULL,
    client_secret TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(provider)
);

CREATE TABLE IF NOT EXISTS user_oauth_connections (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    provider_user_id TEXT NOT NULL,
    provider_email TEXT,
    provider_name TEXT,
    access_token TEXT,
    refresh_token TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(provider, provider_user_id)
);

CREATE INDEX IF NOT EXISTS idx_user_oauth_connections_user_id ON user_oauth_connections(user_id);
CREATE INDEX IF NOT EXISTS idx_user_oauth_connections_provider ON user_oauth_connections(provider, provider_user_id);
