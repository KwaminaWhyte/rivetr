-- Migration 084: White Label Configuration
-- Allows instance admins to customize branding (app name, logo, CSS, etc.)

CREATE TABLE IF NOT EXISTS white_label (
    id INTEGER PRIMARY KEY DEFAULT 1,
    app_name TEXT NOT NULL DEFAULT 'Rivetr',
    app_description TEXT,
    logo_url TEXT,
    favicon_url TEXT,
    custom_css TEXT,
    footer_text TEXT,
    support_url TEXT,
    docs_url TEXT,
    login_page_message TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO white_label (id) VALUES (1);
