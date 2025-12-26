-- Add image_tag column to deployments for rollback support
-- The image_tag stores the Docker/Podman image tag used for this deployment
-- enabling rollback to previous versions without rebuilding

ALTER TABLE deployments ADD COLUMN image_tag TEXT;

-- Index to quickly find successful deployments for an app (for rollback lookup)
CREATE INDEX IF NOT EXISTS idx_deployments_app_status ON deployments(app_id, status);
