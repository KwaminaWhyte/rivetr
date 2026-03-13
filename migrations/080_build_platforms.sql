-- Migration 080: Add build_platforms column to apps table
-- Stores the target platform(s) for Docker builds, e.g. "linux/amd64" or "linux/arm64".
-- NULL means use the default (linux/amd64).
ALTER TABLE apps ADD COLUMN build_platforms TEXT;
