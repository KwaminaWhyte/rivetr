-- Add pre/post deployment commands to apps table
-- Commands are stored as JSON arrays and executed during the deployment pipeline

-- Add pre_deploy_commands column (JSON array of shell commands)
-- Executed after build completes, before container starts
ALTER TABLE apps ADD COLUMN pre_deploy_commands TEXT DEFAULT NULL;

-- Add post_deploy_commands column (JSON array of shell commands)
-- Executed after container is healthy and running
ALTER TABLE apps ADD COLUMN post_deploy_commands TEXT DEFAULT NULL;
