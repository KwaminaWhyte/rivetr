-- Migration 108: per-service resource limits
-- cpu_limit (e.g. "0.5") and memory_limit (e.g. "512M") are injected into each
-- compose service's deploy.resources.limits at compose-write time, unless the
-- service already declares its own limits. NULL/empty = no Rivetr-injected cap.
ALTER TABLE services ADD COLUMN cpu_limit TEXT;
ALTER TABLE services ADD COLUMN memory_limit TEXT;
