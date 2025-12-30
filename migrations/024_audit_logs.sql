-- Migration 024: Audit Logs for tracking user actions
-- Provides compliance logging and debugging capabilities

CREATE TABLE IF NOT EXISTS audit_logs (
    id TEXT PRIMARY KEY NOT NULL,
    -- Action performed (e.g., "app.create", "app.delete", "deployment.trigger", "service.start")
    action TEXT NOT NULL,
    -- Resource type (e.g., "app", "database", "service", "project")
    resource_type TEXT NOT NULL,
    -- ID of the affected resource (nullable for some actions)
    resource_id TEXT,
    -- Name of the resource for display purposes
    resource_name TEXT,
    -- ID of the user who performed the action (nullable for system actions)
    user_id TEXT,
    -- Client IP address
    ip_address TEXT,
    -- JSON blob with additional details (request body, changes, etc.)
    details TEXT,
    -- Timestamp
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);
CREATE INDEX IF NOT EXISTS idx_audit_logs_resource_type ON audit_logs(resource_type);
CREATE INDEX IF NOT EXISTS idx_audit_logs_resource_id ON audit_logs(resource_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at);

-- Composite index for common queries (resource type + date range)
CREATE INDEX IF NOT EXISTS idx_audit_logs_type_date ON audit_logs(resource_type, created_at);
