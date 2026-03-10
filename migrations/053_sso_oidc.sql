-- Migration 053: Add OIDC/SSO provider support for enterprise login

CREATE TABLE IF NOT EXISTS oidc_providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    client_id TEXT NOT NULL,
    client_secret TEXT NOT NULL,
    discovery_url TEXT NOT NULL,
    redirect_uri TEXT NOT NULL,
    scopes TEXT NOT NULL DEFAULT 'openid email profile',
    enabled INTEGER NOT NULL DEFAULT 1,
    team_id TEXT REFERENCES teams(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS sso_states (
    state TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL REFERENCES oidc_providers(id) ON DELETE CASCADE,
    redirect_to TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_sso_states_created_at ON sso_states(created_at);
