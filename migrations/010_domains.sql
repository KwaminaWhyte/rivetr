-- Domain management migration
-- Adds support for multiple domains per app with www redirect and primary domain settings

-- Add domains column to apps table (JSON array of domain objects)
-- Each domain object has: { "domain": "example.com", "primary": true, "redirect_www": true }
-- The existing "domain" field is kept for backwards compatibility (stores the primary domain)
ALTER TABLE apps ADD COLUMN domains TEXT DEFAULT NULL;

-- Add auto_subdomain column to store the auto-generated subdomain
-- Format: <app-name>.<base-domain> (e.g., my-app.rivetr.example.com)
ALTER TABLE apps ADD COLUMN auto_subdomain TEXT DEFAULT NULL;
