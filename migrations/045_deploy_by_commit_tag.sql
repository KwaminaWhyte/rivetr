-- Add git_tag column to deployments table for deploy-by-commit/tag support
ALTER TABLE deployments ADD COLUMN git_tag TEXT;
