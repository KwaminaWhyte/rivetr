-- Migration 035: Team audit logs for tracking team member actions
-- Provides compliance logging and activity tracking for team resources

CREATE TABLE IF NOT EXISTS team_audit_logs (
    id TEXT PRIMARY KEY NOT NULL,
    -- Team that the action belongs to
    team_id TEXT NOT NULL,
    -- User who performed the action (nullable for system actions)
    user_id TEXT,
    -- Action performed (e.g., "member_invited", "member_joined", "role_changed", "app_created")
    action TEXT NOT NULL,
    -- Resource type (e.g., "member", "app", "project", "database", "service", "invitation")
    resource_type TEXT NOT NULL,
    -- ID of the affected resource
    resource_id TEXT,
    -- JSON blob with additional details (old values, new values, etc.)
    details TEXT,
    -- Timestamp
    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_team_audit_logs_team_id ON team_audit_logs(team_id);
CREATE INDEX IF NOT EXISTS idx_team_audit_logs_user_id ON team_audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_team_audit_logs_action ON team_audit_logs(action);
CREATE INDEX IF NOT EXISTS idx_team_audit_logs_resource_type ON team_audit_logs(resource_type);
CREATE INDEX IF NOT EXISTS idx_team_audit_logs_created_at ON team_audit_logs(created_at);

-- Composite index for common queries (team + date range)
CREATE INDEX IF NOT EXISTS idx_team_audit_logs_team_date ON team_audit_logs(team_id, created_at);
