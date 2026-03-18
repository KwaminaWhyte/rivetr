-- Migration 100: CA certificates table
-- Stores custom CA certificates (PEM format) for servers using private CAs.

CREATE TABLE IF NOT EXISTS ca_certificates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    certificate TEXT NOT NULL,
    team_id TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
