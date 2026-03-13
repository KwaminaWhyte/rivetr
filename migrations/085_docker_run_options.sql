-- Migration 083: Add extended Docker run options per app
ALTER TABLE apps ADD COLUMN docker_cap_drop TEXT;
ALTER TABLE apps ADD COLUMN docker_gpus TEXT;
ALTER TABLE apps ADD COLUMN docker_ulimits TEXT;
ALTER TABLE apps ADD COLUMN docker_security_opt TEXT;
