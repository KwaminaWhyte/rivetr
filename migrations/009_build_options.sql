-- Advanced Build Options for Rivetr apps
-- Inspired by Coolify's build configuration options

-- Add dockerfile_path - Custom Dockerfile location (relative to repo root)
-- Default NULL means use the existing 'dockerfile' column
ALTER TABLE apps ADD COLUMN dockerfile_path TEXT DEFAULT NULL;

-- Add base_directory - Build context path (subdirectory in repo)
-- Default NULL means use repository root
ALTER TABLE apps ADD COLUMN base_directory TEXT DEFAULT NULL;

-- Add build_target - Docker multi-stage build target (--target flag)
-- Default NULL means build the final stage
ALTER TABLE apps ADD COLUMN build_target TEXT DEFAULT NULL;

-- Add watch_paths - JSON array of paths to trigger auto-deploy on push
-- Default NULL means watch all paths
-- Example: ["src/", "package.json", "Dockerfile"]
ALTER TABLE apps ADD COLUMN watch_paths TEXT DEFAULT NULL;

-- Add custom_docker_options - Extra docker build/run arguments
-- Default NULL means no additional options
-- Example: "--no-cache --build-arg FOO=bar"
ALTER TABLE apps ADD COLUMN custom_docker_options TEXT DEFAULT NULL;
