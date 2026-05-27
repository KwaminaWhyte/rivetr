-- Migration 107: Add per-app stop_grace_period
-- Seconds to wait for graceful shutdown before SIGKILL when stopping a container.
-- NULL uses the runtime default (Docker's 10s).
ALTER TABLE apps ADD COLUMN stop_grace_period INTEGER;
