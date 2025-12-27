-- Notification channels (Slack, Discord, Email configurations)
CREATE TABLE IF NOT EXISTS notification_channels (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    channel_type TEXT NOT NULL CHECK(channel_type IN ('slack', 'discord', 'email')),
    config TEXT NOT NULL, -- JSON config (webhook_url for Slack/Discord, SMTP settings for Email)
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Notification subscriptions (which events trigger notifications on which channels)
CREATE TABLE IF NOT EXISTS notification_subscriptions (
    id TEXT PRIMARY KEY NOT NULL,
    channel_id TEXT NOT NULL REFERENCES notification_channels(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL CHECK(event_type IN ('deployment_started', 'deployment_success', 'deployment_failed', 'app_stopped', 'app_started')),
    app_id TEXT REFERENCES apps(id) ON DELETE CASCADE, -- NULL means all apps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(channel_id, event_type, app_id)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_notification_channels_type ON notification_channels(channel_type);
CREATE INDEX IF NOT EXISTS idx_notification_channels_enabled ON notification_channels(enabled);
CREATE INDEX IF NOT EXISTS idx_notification_subscriptions_channel_id ON notification_subscriptions(channel_id);
CREATE INDEX IF NOT EXISTS idx_notification_subscriptions_event_type ON notification_subscriptions(event_type);
CREATE INDEX IF NOT EXISTS idx_notification_subscriptions_app_id ON notification_subscriptions(app_id);
