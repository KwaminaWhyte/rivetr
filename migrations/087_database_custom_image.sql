-- Migration 086: Add custom_image and init_commands columns to databases table
-- custom_image: override the default Docker image for this database type (e.g. timescaledb/timescaledb-ha:pg16-latest)
-- init_commands: JSON array of SQL strings to run after first start (e.g. ["CREATE EXTENSION postgis;"])
ALTER TABLE databases ADD COLUMN custom_image TEXT;
ALTER TABLE databases ADD COLUMN init_commands TEXT;
