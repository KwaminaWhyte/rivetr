-- Track when a deployment actually went live (status -> 'running').
-- Distinct from finished_at, which is overwritten to the "replaced/stopped at"
-- moment when a newer deployment takes over. built_at is set once and never
-- overwritten, so the UI can show live uptime for a running deployment and a
-- stable build duration for replaced/failed ones.
ALTER TABLE deployments ADD COLUMN built_at TEXT;
