-- Migration 129: per-user UI preferences (theme).
-- Idempotent: CREATE TABLE IF NOT EXISTS. No foreign key so the built-in admin
-- token user ("system") can store a theme too.
--
-- theme holds the JSON theme object written by the dashboard (mode, accent,
-- neutral, radius, density, font, sidebar style, motion, contrast). The
-- instance-wide default theme lives in instance_settings under 'default_theme'.
CREATE TABLE IF NOT EXISTS user_preferences (
    user_id TEXT PRIMARY KEY,
    theme TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
