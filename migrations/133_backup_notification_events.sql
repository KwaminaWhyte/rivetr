-- Migration 133 (part 1): backup notification events.
-- Adds backup_succeeded, backup_failed and backup_missed to the event_type CHECK
-- and lets a subscription target one backup resource (a database, volume or the
-- instance) through resource_type/resource_id. resource_id has no foreign key
-- because it can point at different tables. SQLite cannot ALTER a CHECK, so the
-- table is rebuilt; the Rust side only runs this when backup_missed is missing.
CREATE TABLE notification_subscriptions_new (
    id TEXT PRIMARY KEY NOT NULL,
    channel_id TEXT NOT NULL REFERENCES notification_channels(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL CHECK(event_type IN ('deployment_started', 'deployment_success', 'deployment_failed', 'app_stopped', 'app_started', 'container_crash', 'container_restarted', 'backup_succeeded', 'backup_failed', 'backup_missed')),
    app_id TEXT REFERENCES apps(id) ON DELETE CASCADE,
    resource_type TEXT CHECK(resource_type IS NULL OR resource_type IN ('database', 'volume', 'instance')),
    resource_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(channel_id, event_type, app_id, resource_type, resource_id)
);
INSERT INTO notification_subscriptions_new (id, channel_id, event_type, app_id, created_at)
    SELECT id, channel_id, event_type, app_id, created_at FROM notification_subscriptions;
DROP TABLE notification_subscriptions;
ALTER TABLE notification_subscriptions_new RENAME TO notification_subscriptions;
CREATE INDEX IF NOT EXISTS idx_notification_subscriptions_channel_id ON notification_subscriptions(channel_id);
CREATE INDEX IF NOT EXISTS idx_notification_subscriptions_event_type ON notification_subscriptions(event_type);
CREATE INDEX IF NOT EXISTS idx_notification_subscriptions_app_id ON notification_subscriptions(app_id);
