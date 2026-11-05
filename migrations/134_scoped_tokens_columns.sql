-- Migration 134 (columns): scoped, restrictable, pausable and rollable API tokens.
-- Existing tokens get every scope so nothing that works today breaks.
ALTER TABLE api_tokens ADD COLUMN scopes TEXT NOT NULL DEFAULT 'read,deploy,write,delete,secrets,admin';
-- Optional restriction to one team (NULL = every team the owner belongs to).
ALTER TABLE api_tokens ADD COLUMN team_id TEXT REFERENCES teams(id) ON DELETE CASCADE;
-- Optional restriction to resources: JSON array of {"type": "project|app|database|service", "id": "..."}.
ALTER TABLE api_tokens ADD COLUMN resources TEXT;
-- Paused tokens are rejected until resumed.
ALTER TABLE api_tokens ADD COLUMN disabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE api_tokens ADD COLUMN last_used_ip TEXT;
-- Last four characters of the current secret, for display.
ALTER TABLE api_tokens ADD COLUMN token_hint TEXT;
-- Rolling keeps the previous secret valid until previous_expires_at.
ALTER TABLE api_tokens ADD COLUMN previous_token_hash TEXT;
ALTER TABLE api_tokens ADD COLUMN previous_expires_at TEXT;
ALTER TABLE api_tokens ADD COLUMN rolled_at TEXT;
CREATE INDEX IF NOT EXISTS idx_api_tokens_previous_hash ON api_tokens(previous_token_hash);
