CREATE TABLE IF NOT EXISTS webhook_events (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,      -- github, gitlab, gitea, bitbucket, dockerhub
    event_type TEXT NOT NULL,    -- push, pull_request, etc.
    repository TEXT,
    branch TEXT,
    commit_sha TEXT,
    payload_size INTEGER,        -- bytes
    apps_triggered INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'received' CHECK(status IN ('received', 'processed', 'ignored', 'error')),
    error_message TEXT,
    received_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_webhook_events_received_at ON webhook_events(received_at);
CREATE INDEX IF NOT EXISTS idx_webhook_events_provider ON webhook_events(provider);
