-- How Rivetr reaches a managed server over SSH: 'public' (the host column) or
-- 'tailscale' (tailscale_host, a Tailscale IP or MagicDNS name). Rivetr does
-- not manage tailnet keys; the Rivetr host must already be on the tailnet.

ALTER TABLE servers ADD COLUMN network_mode TEXT NOT NULL DEFAULT 'public';
ALTER TABLE servers ADD COLUMN tailscale_host TEXT;
