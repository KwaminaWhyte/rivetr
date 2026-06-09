-- Migration 110: expand notification_subscriptions.event_type CHECK constraint
-- The NotificationEventType enum (and the dashboard) gained container_crash and
-- container_restarted, but the original CHECK (migration 016) only allowed the
-- first five, so subscribing to those events failed with a CHECK violation.
-- SQLite can't ALTER a CHECK, so rebuild the table preserving data + relations.
CREATE TABLE notification_subscriptions_new (
    id TEXT PRIMARY KEY NOT NULL,
    channel_id TEXT NOT NULL REFERENCES notification_channels(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL CHECK(event_type IN ('deployment_started', 'deployment_success', 'deployment_failed', 'app_stopped', 'app_started', 'container_crash', 'container_restarted')),
    app_id TEXT REFERENCES apps(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(channel_id, event_type, app_id)
);
INSERT INTO notification_subscriptions_new (id, channel_id, event_type, app_id, created_at)
    SELECT id, channel_id, event_type, app_id, created_at FROM notification_subscriptions;
DROP TABLE notification_subscriptions;
ALTER TABLE notification_subscriptions_new RENAME TO notification_subscriptions;
