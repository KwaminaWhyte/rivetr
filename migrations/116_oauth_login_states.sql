-- Migration 116: dedicated store for OAuth login CSRF states.
-- They were stored in `sessions` under a fake user id ('oauth_state'), which violates the
-- sessions.user_id foreign key, so social login failed with a 500 on every install.
CREATE TABLE IF NOT EXISTS oauth_login_states (
    state TEXT PRIMARY KEY NOT NULL,
    expires_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_oauth_login_states_expires ON oauth_login_states(expires_at);
