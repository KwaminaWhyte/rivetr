-- Migration 072: Add SSL/TLS configuration columns to databases table
ALTER TABLE databases ADD COLUMN ssl_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE databases ADD COLUMN ssl_mode TEXT;
