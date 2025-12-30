-- Migration 026: Build type for application deployments
-- Supports: dockerfile (default), nixpacks, static

-- Build type field (dockerfile, nixpacks, static)
ALTER TABLE apps ADD COLUMN build_type TEXT DEFAULT 'dockerfile';

-- Nixpacks-specific configuration (JSON)
-- Contains: install_cmd, build_cmd, start_cmd, packages, apt_packages
ALTER TABLE apps ADD COLUMN nixpacks_config TEXT DEFAULT NULL;

-- Publish directory for static site builds (e.g., "dist", "build", "out")
ALTER TABLE apps ADD COLUMN publish_directory TEXT DEFAULT NULL;
