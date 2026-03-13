-- Migration 086: Cloudflare Tunnel integration
-- Stores cloudflared tunnel configurations and the routes associated with each tunnel.

CREATE TABLE IF NOT EXISTS cloudflare_tunnels (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    tunnel_token TEXT NOT NULL,
    container_id TEXT,
    status TEXT NOT NULL DEFAULT 'stopped',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS cloudflare_tunnel_routes (
    id TEXT PRIMARY KEY,
    tunnel_id TEXT NOT NULL REFERENCES cloudflare_tunnels(id) ON DELETE CASCADE,
    hostname TEXT NOT NULL,
    service_url TEXT NOT NULL,
    app_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
