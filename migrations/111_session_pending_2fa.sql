-- Migration 111: Add is_pending_2fa flag to sessions
-- Security fix (SEC-C2): the pre-2FA temporary session token must NOT be a fully
-- valid session. Sessions created during the password step of a 2FA login are
-- flagged is_pending_2fa=1 and rejected by the auth middleware; validate_2fa
-- consumes the pending session and issues a real (is_pending_2fa=0) session.
ALTER TABLE sessions ADD COLUMN is_pending_2fa INTEGER NOT NULL DEFAULT 0;
