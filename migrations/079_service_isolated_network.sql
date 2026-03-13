-- Migration 079: Add isolated_network column to services table
-- When enabled (default), each service gets a dedicated Docker network
-- named rivetr-svc-{id_prefix} so it is isolated from other services.
ALTER TABLE services ADD COLUMN isolated_network INTEGER NOT NULL DEFAULT 1;
