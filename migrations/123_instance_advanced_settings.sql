-- Migration 123: seed defaults for the Advanced instance settings.
-- Idempotent: INSERT OR IGNORE never overwrites a value the admin already saved.
--
-- MCP server: an install that already has users keeps the endpoint on (it was
-- always on before this setting existed); a fresh install starts with it off.
INSERT OR IGNORE INTO instance_settings (key, value, updated_at)
SELECT 'mcp_enabled',
       CASE WHEN EXISTS (SELECT 1 FROM users) THEN 'true' ELSE 'false' END,
       datetime('now');

-- Registration: OAuth and SSO logins used to create accounts automatically.
-- Keep that working on installs that already have an enabled login provider,
-- but do not open password sign up for them.
INSERT OR IGNORE INTO instance_settings (key, value, updated_at)
SELECT 'registration_enabled',
       CASE WHEN EXISTS (SELECT 1 FROM users)
             AND (EXISTS (SELECT 1 FROM oauth_providers WHERE enabled = 1)
                  OR EXISTS (SELECT 1 FROM oidc_providers WHERE enabled = 1))
            THEN 'true' ELSE 'false' END,
       datetime('now');

INSERT OR IGNORE INTO instance_settings (key, value, updated_at)
SELECT 'allow_password_registration',
       CASE WHEN EXISTS (SELECT 1 FROM users) THEN 'false' ELSE 'true' END,
       datetime('now');
