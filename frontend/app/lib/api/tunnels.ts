/**
 * Cloudflare Tunnels API module.
 * Handles cloudflared tunnel lifecycle and route management.
 */

import { apiRequest } from "./core";

export interface CloudflareTunnelRoute {
  id: string;
  tunnel_id: string;
  hostname: string;
  service_url: string;
  app_id?: string;
  created_at: string;
}

export interface CloudflareTunnel {
  id: string;
  name: string;
  /** Always "***" — token is never returned in full. */
  tunnel_token: string;
  container_id?: string;
  status: "stopped" | "starting" | "running" | "error";
  routes: CloudflareTunnelRoute[];
  created_at: string;
  updated_at: string;
}

export interface CreateTunnelRequest {
  name: string;
  tunnel_token: string;
}

export interface CreateTunnelRouteRequest {
  hostname: string;
  service_url: string;
  app_id?: string;
}

export const tunnelsApi = {
  /** List all Cloudflare tunnels with their routes. */
  list: (token?: string) =>
    apiRequest<CloudflareTunnel[]>("/tunnels", {}, token),

  /** Create a tunnel and start the cloudflared container. */
  create: (data: CreateTunnelRequest, token?: string) =>
    apiRequest<CloudflareTunnel>("/tunnels", {
      method: "POST",
      body: JSON.stringify(data),
    }, token),

  /** Delete a tunnel (stops container and removes record). */
  delete: (id: string, token?: string) =>
    apiRequest<void>(`/tunnels/${id}`, { method: "DELETE" }, token),

  /** Start the cloudflared container for a tunnel. */
  start: (id: string, token?: string) =>
    apiRequest<void>(`/tunnels/${id}/start`, { method: "POST" }, token),

  /** Stop the cloudflared container for a tunnel. */
  stop: (id: string, token?: string) =>
    apiRequest<void>(`/tunnels/${id}/stop`, { method: "POST" }, token),

  /** List routes for a tunnel. */
  listRoutes: (tunnelId: string, token?: string) =>
    apiRequest<CloudflareTunnelRoute[]>(`/tunnels/${tunnelId}/routes`, {}, token),

  /** Add a route to a tunnel. */
  createRoute: (tunnelId: string, data: CreateTunnelRouteRequest, token?: string) =>
    apiRequest<CloudflareTunnelRoute>(`/tunnels/${tunnelId}/routes`, {
      method: "POST",
      body: JSON.stringify(data),
    }, token),

  /** Remove a route from a tunnel. */
  deleteRoute: (tunnelId: string, routeId: string, token?: string) =>
    apiRequest<void>(`/tunnels/${tunnelId}/routes/${routeId}`, { method: "DELETE" }, token),
};
