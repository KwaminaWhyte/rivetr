-- Migration 062: Add rollback retention count to apps
ALTER TABLE apps ADD COLUMN rollback_retention_count INTEGER NOT NULL DEFAULT 10;
