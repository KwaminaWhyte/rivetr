-- Migration 071: Instance settings table
-- Stores key-value settings for the Rivetr instance itself (domain, name, etc.)

CREATE TABLE IF NOT EXISTS instance_settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Seed default rows so GET always returns something
INSERT OR IGNORE INTO instance_settings (key, value) VALUES ('instance_domain', NULL);
INSERT OR IGNORE INTO instance_settings (key, value) VALUES ('instance_name', NULL);
