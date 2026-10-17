-- Opt-in per-app proxy access logging. Existing apps keep logging (default 1);
-- turning it off skips building log metadata and writing to proxy_logs for
-- that app's requests.
ALTER TABLE apps ADD COLUMN access_logging INTEGER NOT NULL DEFAULT 1;
