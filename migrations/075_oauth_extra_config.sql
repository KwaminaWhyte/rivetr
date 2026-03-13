-- Migration 075: Add extra_config column to oauth_providers for provider-specific settings
-- (e.g., Azure AD tenant_id)
ALTER TABLE oauth_providers ADD COLUMN extra_config TEXT;
