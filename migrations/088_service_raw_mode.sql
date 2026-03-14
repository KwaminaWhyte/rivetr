-- Migration 088: Add raw compose mode toggle per service
ALTER TABLE services ADD COLUMN raw_compose_mode INTEGER NOT NULL DEFAULT 0;
