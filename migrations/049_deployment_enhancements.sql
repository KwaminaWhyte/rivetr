-- Migration 049: Deployment Enhancements
-- Approval workflow, maintenance mode, freeze windows, and scheduled deployments

-- Add approval workflow columns to deployments
ALTER TABLE deployments ADD COLUMN approval_status TEXT
  CHECK(approval_status IN ('pending', 'approved', 'rejected')) DEFAULT NULL;
ALTER TABLE deployments ADD COLUMN approved_by TEXT REFERENCES users(id);
ALTER TABLE deployments ADD COLUMN approved_at TEXT;
ALTER TABLE deployments ADD COLUMN rejection_reason TEXT;
ALTER TABLE deployments ADD COLUMN scheduled_at TEXT;

-- Add approval and maintenance settings to apps
ALTER TABLE apps ADD COLUMN require_approval INTEGER NOT NULL DEFAULT 0;
ALTER TABLE apps ADD COLUMN maintenance_mode INTEGER NOT NULL DEFAULT 0;
ALTER TABLE apps ADD COLUMN maintenance_message TEXT DEFAULT 'Service temporarily unavailable';

-- Deployment freeze windows table
CREATE TABLE IF NOT EXISTS deployment_freeze_windows (
  id TEXT PRIMARY KEY,
  app_id TEXT REFERENCES apps(id) ON DELETE CASCADE,
  team_id TEXT REFERENCES teams(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  start_time TEXT NOT NULL,
  end_time TEXT NOT NULL,
  days_of_week TEXT NOT NULL DEFAULT '0,1,2,3,4,5,6',
  is_active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
