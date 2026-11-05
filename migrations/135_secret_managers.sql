-- Migration 135: external secret managers (HashiCorp Vault, OpenBao, Infisical,
-- Doppler, AWS Secrets Manager) resolved at deploy time.
--
-- secret_connections: team-scoped connections. `config` is non-secret JSON
-- (address, mount, region, ...); `credentials` is a JSON object of tokens and
-- keys, encrypted with the instance encryption key like env var values.
-- Env var values reference a connection by name:
--   secret://<connection>/<path>#<key>
-- secret_imports: link every key at a path into an app, service or project's
-- env, with an optional key prefix.
-- secret_resolution_state: per-owner failure policy ('fail' or 'last_known')
-- and the last successfully resolved values (encrypted JSON), used only by the
-- 'last_known' policy when a provider is unreachable during a deploy.

CREATE TABLE IF NOT EXISTS secret_connections (
    id TEXT PRIMARY KEY NOT NULL,
    team_id TEXT REFERENCES teams(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    provider TEXT NOT NULL,
    config TEXT NOT NULL DEFAULT '{}',
    credentials TEXT NOT NULL DEFAULT '',
    last_tested_at TEXT,
    last_test_ok INTEGER,
    last_test_message TEXT,
    created_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_secret_connections_team_name
    ON secret_connections(COALESCE(team_id, ''), name);

CREATE TABLE IF NOT EXISTS secret_imports (
    id TEXT PRIMARY KEY NOT NULL,
    owner_type TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    connection_id TEXT NOT NULL REFERENCES secret_connections(id) ON DELETE CASCADE,
    path TEXT NOT NULL DEFAULT '',
    prefix TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(owner_type, owner_id, connection_id, path, prefix)
);

CREATE INDEX IF NOT EXISTS idx_secret_imports_owner
    ON secret_imports(owner_type, owner_id);

CREATE TABLE IF NOT EXISTS secret_resolution_state (
    owner_type TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    failure_policy TEXT NOT NULL DEFAULT 'fail',
    last_values TEXT,
    last_resolved_at TEXT,
    PRIMARY KEY (owner_type, owner_id)
);
