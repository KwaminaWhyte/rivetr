-- Add Docker Registry support for deploying pre-built images
-- Allows deploying from Docker Hub, GHCR, or private registries

-- Docker image name (e.g., "nginx", "ghcr.io/user/app")
-- When set, deployment will pull this image instead of building from git
ALTER TABLE apps ADD COLUMN docker_image TEXT DEFAULT NULL;

-- Image tag (default: "latest")
ALTER TABLE apps ADD COLUMN docker_image_tag TEXT DEFAULT 'latest';

-- Custom registry URL (null = Docker Hub)
ALTER TABLE apps ADD COLUMN registry_url TEXT DEFAULT NULL;

-- Registry authentication credentials
ALTER TABLE apps ADD COLUMN registry_username TEXT DEFAULT NULL;

-- Registry password (stored encrypted - TODO: encrypt in application layer)
ALTER TABLE apps ADD COLUMN registry_password TEXT DEFAULT NULL;
