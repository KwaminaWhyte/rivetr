-- Migration 078: Add build_secrets column for BuildKit secret injection
ALTER TABLE apps ADD COLUMN build_secrets TEXT;
