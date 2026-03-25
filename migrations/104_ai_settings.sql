-- Migration 104: AI provider settings in instance_settings table
-- Stores the AI provider config (api_key, provider, model) set from the dashboard.

INSERT OR IGNORE INTO instance_settings (key, value) VALUES ('ai_provider', NULL);
INSERT OR IGNORE INTO instance_settings (key, value) VALUES ('ai_api_key', NULL);
INSERT OR IGNORE INTO instance_settings (key, value) VALUES ('ai_model', NULL);
INSERT OR IGNORE INTO instance_settings (key, value) VALUES ('ai_max_tokens', NULL);
