-- Migration 030: Add automatic rollback and registry push settings to apps
-- This enables health-based auto-rollback and pushing images to registry for rollback support

-- Add auto_rollback_enabled column (0 = disabled, 1 = enabled)
-- When enabled, failed health checks will automatically rollback to the previous version
ALTER TABLE apps ADD COLUMN auto_rollback_enabled INTEGER NOT NULL DEFAULT 0;

-- Add registry_push_enabled column (0 = disabled, 1 = enabled)
-- When enabled, successfully built images are pushed to the configured registry
-- This allows rollback even after local images are cleaned up
ALTER TABLE apps ADD COLUMN registry_push_enabled INTEGER NOT NULL DEFAULT 0;

-- Add max_rollback_versions column (default: 5)
-- Number of deployment versions to keep for rollback in the registry
ALTER TABLE apps ADD COLUMN max_rollback_versions INTEGER NOT NULL DEFAULT 5;

-- Add rollback_from_deployment_id to track which deployment triggered an auto-rollback
-- This helps prevent rollback loops and provides audit trail
ALTER TABLE deployments ADD COLUMN rollback_from_deployment_id TEXT;

-- Add is_auto_rollback flag to identify deployments that were auto-rollbacks
ALTER TABLE deployments ADD COLUMN is_auto_rollback INTEGER NOT NULL DEFAULT 0;
