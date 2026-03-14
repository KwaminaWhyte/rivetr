CREATE TABLE IF NOT EXISTS community_template_submissions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    category TEXT NOT NULL,
    icon TEXT,
    compose_content TEXT NOT NULL,
    submitted_by TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending',
    admin_notes TEXT,
    reviewed_by TEXT REFERENCES users(id),
    reviewed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_cts_status ON community_template_submissions(status);
CREATE INDEX IF NOT EXISTS idx_cts_submitted_by ON community_template_submissions(submitted_by);
