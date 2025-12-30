-- Migration 028: GitHub Apps configuration
-- System-wide apps registered via manifest flow

CREATE TABLE IF NOT EXISTS github_apps (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,                    -- App name on GitHub
    app_id INTEGER NOT NULL,               -- GitHub App ID
    client_id TEXT NOT NULL,               -- OAuth client ID
    client_secret TEXT NOT NULL,           -- OAuth client secret (encrypted)
    private_key TEXT NOT NULL,             -- PEM private key (encrypted)
    webhook_secret TEXT NOT NULL,          -- Webhook secret (encrypted)

    -- Optional fields
    slug TEXT,                             -- GitHub App slug
    owner TEXT,                            -- Owner (user or org)
    permissions TEXT,                      -- JSON: granted permissions
    events TEXT,                           -- JSON: subscribed events

    -- Sharing settings
    is_system_wide INTEGER NOT NULL DEFAULT 0,  -- Available to all teams
    team_id TEXT REFERENCES teams(id) ON DELETE SET NULL,  -- Owner team

    -- Metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_by TEXT NOT NULL               -- User who registered the app
);

-- GitHub App installations (per org/user account)
CREATE TABLE IF NOT EXISTS github_app_installations (
    id TEXT PRIMARY KEY NOT NULL,
    github_app_id TEXT NOT NULL REFERENCES github_apps(id) ON DELETE CASCADE,
    installation_id INTEGER NOT NULL,       -- GitHub installation ID
    account_type TEXT NOT NULL,             -- 'user' or 'organization'
    account_login TEXT NOT NULL,            -- GitHub username or org name
    account_id INTEGER NOT NULL,            -- GitHub account ID

    -- Token management (installation access tokens)
    access_token TEXT,                      -- Current installation access token (encrypted)
    token_expires_at TEXT,                  -- Token expiration timestamp

    -- Permissions snapshot
    permissions TEXT,                       -- JSON: granted permissions at install time
    repository_selection TEXT,              -- 'all' or 'selected'

    -- Metadata
    suspended_at TEXT,                      -- If installation is suspended
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    UNIQUE(github_app_id, installation_id)
);

-- Link apps to GitHub App installations for auto-deploy
ALTER TABLE apps ADD COLUMN github_app_installation_id TEXT
    REFERENCES github_app_installations(id) ON DELETE SET NULL;

-- Indexes
CREATE INDEX IF NOT EXISTS idx_github_apps_team_id ON github_apps(team_id);
CREATE INDEX IF NOT EXISTS idx_github_apps_is_system_wide ON github_apps(is_system_wide);
CREATE INDEX IF NOT EXISTS idx_github_app_installations_github_app_id
    ON github_app_installations(github_app_id);
CREATE INDEX IF NOT EXISTS idx_github_app_installations_account_login
    ON github_app_installations(account_login);
CREATE INDEX IF NOT EXISTS idx_apps_github_app_installation_id ON apps(github_app_installation_id);
