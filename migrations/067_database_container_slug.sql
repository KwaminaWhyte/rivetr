-- Migration 067: Add container_slug to databases for globally unique container names
--
-- Previously, the container name was derived as `rivetr-db-{name}`, which could
-- collide if two databases in different teams had the same name.
-- Now each database gets a stable, unique slug based on its ID prefix.

ALTER TABLE databases ADD COLUMN container_slug TEXT;

-- Back-fill existing rows with the old name-based value so nothing breaks.
UPDATE databases SET container_slug = 'rivetr-db-' || name WHERE container_slug IS NULL;
