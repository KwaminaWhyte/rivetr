-- Add public access fields to services table
-- public_access: whether the service port is exposed on the host
-- external_port: the host port to bind (e.g., 6380 for Redis)
-- expose_container_port: the container port to expose (e.g., 6379 for Redis)
ALTER TABLE services ADD COLUMN public_access INTEGER NOT NULL DEFAULT 0;
ALTER TABLE services ADD COLUMN external_port INTEGER NOT NULL DEFAULT 0;
ALTER TABLE services ADD COLUMN expose_container_port INTEGER NOT NULL DEFAULT 0;
