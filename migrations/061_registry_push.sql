-- Migration 061: Registry push pipeline (columns already exist from migration 030)
-- registry_url, registry_username, registry_password: added in migration 014
-- registry_push_enabled: added in migration 030
-- This migration is a no-op placeholder.
-- The engine uses these columns to push images after a successful build.
SELECT 1;
