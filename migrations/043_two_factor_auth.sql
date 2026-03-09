-- Migration 043: Add two-factor authentication columns to users table
-- totp_secret: Encrypted TOTP secret for generating/verifying codes
-- totp_enabled: Whether 2FA is enabled for this user (0 = disabled, 1 = enabled)
-- recovery_codes: JSON array of hashed recovery codes

ALTER TABLE users ADD COLUMN totp_secret TEXT;
ALTER TABLE users ADD COLUMN totp_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN recovery_codes TEXT
