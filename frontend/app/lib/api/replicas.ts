/**
 * Container Replicas API module.
 * Provides methods for managing multiple container instances per app with load balancing.
 */

import { apiRequest } from "./core";

export interface AppReplica {
  id: string;
  app_id: string;
  replica_index: number;
  container_id?: string;
  status: "running" | "stopped" | "error" | "starting";
  started_at?: string;
  stopped_at?: string;
  created_at: string;
}

export const replicasApi = {
  /**
   * List all replicas for an app
   */
  list: (appId: string): Promise<AppReplica[]> =>
    apiRequest<AppReplica[]>(`/apps/${appId}/replicas`),

  /**
   * Set the replica count for an app.
   * If the app is running, starts/stops containers to match the desired count.
   */
  setCount: (appId: string, count: number): Promise<AppReplica[]> =>
    apiRequest<AppReplica[]>(`/apps/${appId}/replicas/count`, {
      method: "PUT",
      body: JSON.stringify({ count }),
    }),

  /**
   * Restart a specific replica by index
   */
  restart: (appId: string, index: number): Promise<AppReplica> =>
    apiRequest<AppReplica>(`/apps/${appId}/replicas/${index}/restart`, {
      method: "POST",
      body: JSON.stringify({}),
    }),
};
