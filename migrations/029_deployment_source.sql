-- Add deployment_source field to apps table
-- Tracks how the app is deployed: 'git', 'upload', or 'registry'

ALTER TABLE apps ADD COLUMN deployment_source TEXT DEFAULT 'git';
