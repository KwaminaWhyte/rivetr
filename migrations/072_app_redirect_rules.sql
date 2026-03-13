-- Migration 072: URL redirect rules per app
-- Supports regex-based HTTP redirects enforced at the proxy level

CREATE TABLE IF NOT EXISTS app_redirect_rules (
    id TEXT PRIMARY KEY,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    source_pattern TEXT NOT NULL,   -- regex pattern to match against request path
    destination TEXT NOT NULL,      -- redirect destination (can use $1, $2 for capture groups)
    is_permanent INTEGER NOT NULL DEFAULT 0, -- 1 = 301 Moved Permanently, 0 = 302 Found
    is_enabled INTEGER NOT NULL DEFAULT 1,
    sort_order INTEGER NOT NULL DEFAULT 0,   -- lower = higher priority
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_app_redirect_rules_app_id
    ON app_redirect_rules(app_id, is_enabled, sort_order);
