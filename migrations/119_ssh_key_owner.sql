-- Migration 119: record who created an SSH key. Global keys (app_id NULL) have no team
-- owner, so edits are limited to the creator or an instance admin. Existing keys keep
-- created_by NULL and are admin-only to change.
ALTER TABLE ssh_keys ADD COLUMN created_by TEXT;
