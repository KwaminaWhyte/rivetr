-- Migration 137: scheduled restart runs, community template review, freeze
-- windows for every deploy trigger.
--
-- Column additions (guarded by column-exists checks in src/db/mod.rs):
--   deployment_freeze_windows.include_previews  INTEGER NOT NULL DEFAULT 0
--   community_template_submissions.env_schema   TEXT  (JSON env var hints)
--   community_template_submissions.rejection_reason TEXT
--   community_template_submissions.approved_template_id TEXT
--   service_templates.is_community              INTEGER NOT NULL DEFAULT 0
--   service_templates.submission_id             TEXT
--
-- Idempotent: every statement below uses IF NOT EXISTS.

-- One row per execution (or skip) of a scheduled restart.
-- status: running | succeeded | failed | skipped
CREATE TABLE IF NOT EXISTS scheduled_restart_runs (
    id TEXT PRIMARY KEY,
    schedule_id TEXT NOT NULL REFERENCES scheduled_restarts(id) ON DELETE CASCADE,
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    scheduled_for TEXT,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    status TEXT NOT NULL DEFAULT 'running',
    catch_up INTEGER NOT NULL DEFAULT 0,
    deployment_id TEXT,
    error_message TEXT
);

CREATE INDEX IF NOT EXISTS idx_scheduled_restart_runs_schedule
    ON scheduled_restart_runs(schedule_id, started_at);
CREATE INDEX IF NOT EXISTS idx_scheduled_restart_runs_app
    ON scheduled_restart_runs(app_id, started_at);
