-- Team ownership for Cloudflare tunnels. NULL keeps the legacy behavior
-- (visible to every signed-in user); existing tunnels stay NULL.
ALTER TABLE cloudflare_tunnels ADD COLUMN team_id TEXT REFERENCES teams(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_cloudflare_tunnels_team_id ON cloudflare_tunnels(team_id);
