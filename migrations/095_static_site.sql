-- Migration 094: Add is_static_site flag to apps
-- When true, the app is served as a static site from the publish directory
-- (no running process needed — files are served by a static file server)
ALTER TABLE apps ADD COLUMN is_static_site INTEGER NOT NULL DEFAULT 0;
