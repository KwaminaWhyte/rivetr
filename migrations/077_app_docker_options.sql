-- Migration 076: Add custom Docker run options per app
ALTER TABLE apps ADD COLUMN privileged INTEGER NOT NULL DEFAULT 0;
ALTER TABLE apps ADD COLUMN cap_add TEXT;
ALTER TABLE apps ADD COLUMN devices TEXT;
ALTER TABLE apps ADD COLUMN shm_size TEXT;
ALTER TABLE apps ADD COLUMN init_process INTEGER NOT NULL DEFAULT 0;
