-- Migration 027: Preview Deployments for PR branches
-- Tracks PR-based preview environments with auto-cleanup

CREATE TABLE IF NOT EXISTS preview_deployments (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,

    -- PR information
    pr_number INTEGER NOT NULL,
    pr_title TEXT,
    pr_source_branch TEXT NOT NULL,
    pr_target_branch TEXT NOT NULL,
    pr_author TEXT,
    pr_url TEXT,

    -- Git provider info (for posting comments)
    provider_type TEXT NOT NULL,  -- 'github', 'gitlab', 'gitea'
    repo_full_name TEXT NOT NULL, -- 'owner/repo'

    -- Deployment info
    preview_domain TEXT NOT NULL UNIQUE,  -- pr-123.myapp.example.com
    container_id TEXT,
    container_name TEXT,
    image_tag TEXT,
    port INTEGER,
    commit_sha TEXT,
    commit_message TEXT,
    status TEXT NOT NULL DEFAULT 'pending',  -- pending, building, running, failed, closed
    error_message TEXT,

    -- Comment tracking (to update existing comment)
    github_comment_id INTEGER,

    -- Resource limits (lower than production)
    memory_limit TEXT DEFAULT '256m',
    cpu_limit TEXT DEFAULT '0.5',

    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    closed_at TEXT,

    -- Unique constraint: one preview per PR per app
    UNIQUE(app_id, pr_number)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_preview_deployments_app_id ON preview_deployments(app_id);
CREATE INDEX IF NOT EXISTS idx_preview_deployments_status ON preview_deployments(status);
CREATE INDEX IF NOT EXISTS idx_preview_deployments_pr_number ON preview_deployments(pr_number);
CREATE INDEX IF NOT EXISTS idx_preview_deployments_domain ON preview_deployments(preview_domain);

-- Add preview_enabled flag to apps
ALTER TABLE apps ADD COLUMN preview_enabled INTEGER DEFAULT 0;
