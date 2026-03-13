ALTER TABLE apps ADD COLUMN restart_policy TEXT NOT NULL DEFAULT 'unless-stopped';
