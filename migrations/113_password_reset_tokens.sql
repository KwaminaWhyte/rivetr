-- Short-lived tokens for the forgot-password flow. Only the SHA-256 hash of the
-- token is stored (the raw token is emailed to the user and never persisted).
-- Tokens are single-use (used_at stamped on consumption) and expire after a
-- short window enforced in the handler via expires_at.
CREATE TABLE IF NOT EXISTS password_reset_tokens (
    id TEXT PRIMARY KEY,
    token_hash TEXT NOT NULL UNIQUE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL,
    used_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_password_reset_tokens_user
    ON password_reset_tokens(user_id);
