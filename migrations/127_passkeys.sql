-- Migration 127: passkeys (WebAuthn).
-- Idempotent: CREATE ... IF NOT EXISTS and INSERT OR IGNORE.

CREATE TABLE IF NOT EXISTS user_passkeys (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    -- base64url (no padding) credential id, unique across all users
    credential_id TEXT NOT NULL UNIQUE,
    -- WebAuthn user handle (UUID) shared by all of a user's passkeys
    user_handle TEXT NOT NULL,
    -- Serialized webauthn-rs Passkey (public key, counter, flags). Not secret.
    passkey TEXT NOT NULL,
    sign_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_used_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_user_passkeys_user ON user_passkeys (user_id);

-- Short-lived registration / login ceremony state (server side, single use).
CREATE TABLE IF NOT EXISTS webauthn_challenges (
    id TEXT PRIMARY KEY,
    user_id TEXT,
    -- 'register' | 'login'
    purpose TEXT NOT NULL,
    state TEXT NOT NULL,
    -- Unix seconds
    expires_at INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_webauthn_challenges_expires ON webauthn_challenges (expires_at);

INSERT OR IGNORE INTO instance_settings (key, value, updated_at)
VALUES ('passkeys_enabled', 'true', datetime('now'));
