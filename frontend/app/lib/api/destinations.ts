/**
 * Destinations API module.
 * Handles Docker destination (named network) management.
 */

import { apiRequest } from "./core";
import type { Destination, CreateDestinationRequest } from "@/types/destinations";

export const destinationsApi = {
  list: (teamId?: string, token?: string) => {
    const params = teamId ? `?team_id=${encodeURIComponent(teamId)}` : "";
    return apiRequest<Destination[]>(`/destinations${params}`, {}, token);
  },
  getOne: (id: string, token?: string) =>
    apiRequest<Destination>(`/destinations/${id}`, {}, token),
  create: (data: CreateDestinationRequest, token?: string) =>
    apiRequest<Destination>(
      "/destinations",
      { method: "POST", body: JSON.stringify(data) },
      token
    ),
  delete: (id: string, token?: string) =>
    apiRequest<void>(`/destinations/${id}`, { method: "DELETE" }, token),
};
