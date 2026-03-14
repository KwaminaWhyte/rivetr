-- Add cancelled_at timestamp to deployments for cancellation tracking
ALTER TABLE deployments ADD COLUMN cancelled_at TEXT;
