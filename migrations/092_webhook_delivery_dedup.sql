-- Add delivery_id to webhook_events for idempotent deduplication.
-- GitHub sends X-GitHub-Delivery (a UUID) with every webhook request.
-- Storing it with a UNIQUE constraint lets us skip duplicate deliveries
-- (e.g. retries or double-firing GitHub App installations).

ALTER TABLE webhook_events ADD COLUMN delivery_id TEXT;
CREATE UNIQUE INDEX IF NOT EXISTS idx_webhook_events_delivery_id
    ON webhook_events (delivery_id)
    WHERE delivery_id IS NOT NULL;
