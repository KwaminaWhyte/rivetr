-- Migration 038: Add 'webhook' to notification_channels channel_type CHECK constraint
-- SQLite doesn't support ALTER CHECK, so we recreate the table
-- Note: PRAGMA foreign_keys=OFF must be set before running this migration

-- Step 1: Create new table with updated constraint
CREATE TABLE IF NOT EXISTS notification_channels_new (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    channel_type TEXT NOT NULL CHECK(channel_type IN ('slack', 'discord', 'email', 'webhook')),
    config TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    team_id TEXT REFERENCES teams(id) ON DELETE CASCADE
);

-- Step 2: Copy existing data
INSERT OR IGNORE INTO notification_channels_new SELECT * FROM notification_channels;

-- Step 3: Drop old table
DROP TABLE IF EXISTS notification_channels;

-- Step 4: Rename new table
ALTER TABLE notification_channels_new RENAME TO notification_channels;

-- Step 5: Recreate indexes
CREATE INDEX IF NOT EXISTS idx_notification_channels_type ON notification_channels(channel_type);
CREATE INDEX IF NOT EXISTS idx_notification_channels_enabled ON notification_channels(enabled);
CREATE INDEX IF NOT EXISTS idx_notification_channels_team_id ON notification_channels(team_id);
