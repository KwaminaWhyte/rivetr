-- Migration 093: Add trigger column to deployments table
-- Tracks how a deployment was initiated: 'push', 'manual', 'webhook', 'rollback', 'restart', etc.

ALTER TABLE deployments ADD COLUMN trigger TEXT DEFAULT 'manual';
