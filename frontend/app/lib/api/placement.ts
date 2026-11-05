/**
 * Multi-server placement API: run one app on several servers behind the
 * Rivetr proxy, with a rolling rollout and per-target deployment records.
 */

import { apiRequest } from "./core";

export type ImageDistribution = "auto" | "registry" | "ssh";
export type LbPolicy = "round_robin" | "least_conn";

export interface MultiServerSettings {
  /** Containers replaced per rolling batch */
  max_unavailable: number;
  /** Roll the whole deployment back when at least this share of targets fail */
  failure_threshold_pct: number;
  distribution: ImageDistribution;
  lb_policy: LbPolicy;
  sticky_sessions: boolean;
  /** Publish remote ports on the server's Tailscale address when it has one */
  bind_private: boolean;
}

export interface PlacementView {
  server_id: string;
  server_name: string;
  replicas: number;
  /** Host the proxy uses to reach this server's containers */
  address: string;
  private_bind: boolean;
  is_local: boolean;
}

export interface PlacementResponse {
  enabled: boolean;
  placements: PlacementView[];
  settings: MultiServerSettings;
  server_count: number;
  registry_configured: boolean;
  upstreams: string[];
}

export interface PlacementInput {
  server_id: string;
  replicas: number;
}

export type DeploymentTargetStatus =
  | "pending"
  | "distributing"
  | "starting"
  | "checking"
  | "healthy"
  | "failed"
  | "rolled_back"
  | "replaced"
  | "stopped";

export interface DeploymentTarget {
  id: string;
  deployment_id: string;
  app_id: string;
  server_id: string;
  server_name: string;
  replica_index: number;
  batch: number;
  status: DeploymentTargetStatus;
  container_name?: string | null;
  container_id?: string | null;
  upstream?: string | null;
  error_message?: string | null;
  log: string;
  started_at?: string | null;
  finished_at?: string | null;
  created_at: string;
}

export const DEFAULT_MULTI_SERVER_SETTINGS: MultiServerSettings = {
  max_unavailable: 1,
  failure_threshold_pct: 50,
  distribution: "auto",
  lb_policy: "round_robin",
  sticky_sessions: false,
  bind_private: true,
};

export const placementApi = {
  get: (appId: string): Promise<PlacementResponse> =>
    apiRequest<PlacementResponse>(`/apps/${appId}/placement`),

  /** Replace the placement. An empty list returns the app to single-host deploys. */
  update: (
    appId: string,
    placements: PlacementInput[],
    settings: MultiServerSettings,
  ): Promise<PlacementResponse> =>
    apiRequest<PlacementResponse>(`/apps/${appId}/placement`, {
      method: "PUT",
      body: JSON.stringify({ placements, settings }),
    }),

  /** `{app_id: server_count}` for every visible app with a placement */
  counts: (): Promise<Record<string, number>> =>
    apiRequest<Record<string, number>>(`/apps/placement-counts`),

  targets: (deploymentId: string): Promise<DeploymentTarget[]> =>
    apiRequest<DeploymentTarget[]>(`/deployments/${deploymentId}/targets`),
};
