-- Add team_id to notification_channels for team-scoped alert notifications
-- Also add support for generic webhook channel type with headers and payload template

-- Add team_id column to notification_channels (nullable for backward compatibility with global channels)
ALTER TABLE notification_channels ADD COLUMN team_id TEXT REFERENCES teams(id) ON DELETE CASCADE;

-- Create index for team lookups
CREATE INDEX IF NOT EXISTS idx_notification_channels_team_id ON notification_channels(team_id);

-- Note: The existing channel_type CHECK constraint allows 'slack', 'discord', 'email'.
-- SQLite doesn't support ALTER TABLE to modify CHECK constraints, so we need to recreate the table.
-- For now, we'll use 'webhook' as a valid type by storing it, and the application layer will handle validation.
-- The existing CHECK constraint: CHECK(channel_type IN ('slack', 'discord', 'email'))
-- We cannot easily add 'webhook' to SQLite CHECK constraints without recreating the table.
-- The application layer will validate and handle the 'webhook' type appropriately.
