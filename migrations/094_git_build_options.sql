-- Add git clone options, build options, and custom container name to apps table
ALTER TABLE apps ADD COLUMN git_submodules INTEGER NOT NULL DEFAULT 0;
ALTER TABLE apps ADD COLUMN git_lfs INTEGER NOT NULL DEFAULT 0;
ALTER TABLE apps ADD COLUMN shallow_clone INTEGER NOT NULL DEFAULT 1;
ALTER TABLE apps ADD COLUMN disable_build_cache INTEGER NOT NULL DEFAULT 0;
ALTER TABLE apps ADD COLUMN include_source_commit INTEGER NOT NULL DEFAULT 0;
ALTER TABLE apps ADD COLUMN custom_container_name TEXT DEFAULT NULL;
