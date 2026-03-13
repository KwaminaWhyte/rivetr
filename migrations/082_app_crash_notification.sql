-- Migration 082: Add last_crash_notified_at column to apps for crash notification rate-limiting
ALTER TABLE apps ADD COLUMN last_crash_notified_at TEXT;
