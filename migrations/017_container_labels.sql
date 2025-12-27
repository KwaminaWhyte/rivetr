-- Container labels for custom Docker/Podman labels (Coolify-inspired)
-- Stores as JSON object {"key": "value", ...}
ALTER TABLE apps ADD COLUMN container_labels TEXT DEFAULT NULL;
