export interface Destination {
  id: string;
  name: string;
  network_name: string;
  server_id: string | null;
  team_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateDestinationRequest {
  name: string;
  network_name: string;
  server_id?: string;
  team_id?: string;
}
