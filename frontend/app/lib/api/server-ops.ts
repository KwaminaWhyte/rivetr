/**
 * Server operations API: images, Docker cleanup and package patching.
 * The server id "localhost" addresses the runtime Rivetr itself runs on.
 */

import { apiRequest } from "./core";

export const LOCALHOST_SERVER_ID = "localhost";

export interface ServerImage {
  id: string;
  short_id: string;
  repository: string;
  tag: string;
  created_at: string | null;
  size_bytes: number;
  used_by: string[];
  /** Referenced by a Rivetr deployment and kept for rollbacks */
  retained: boolean;
}

export interface ImageInventory {
  images: ServerImage[];
  total_bytes: number;
  reclaimable_bytes: number;
}

export interface MaintenanceSettings {
  server_id: string;
  cleanup_enabled: boolean;
  cleanup_cron: string;
  cleanup_trigger: "schedule" | "threshold";
  cleanup_threshold_pct: number;
  remove_unused_volumes: boolean;
  remove_unused_networks: boolean;
  remove_retained_images: boolean;
  patch_check_enabled: boolean;
  patch_check_cron: string;
  auto_security_updates: boolean;
  maintenance_window: string;
  last_patch_check_at: string | null;
}

export type UpdateMaintenanceSettings = Partial<
  Omit<MaintenanceSettings, "server_id" | "last_patch_check_at">
>;

export interface CleanupRun {
  id: string;
  server_id: string;
  status: "running" | "success" | "failed" | "skipped";
  trigger_type: "manual" | "schedule" | "threshold";
  started_at: string;
  finished_at: string | null;
  duration_ms: number | null;
  freed_bytes: number;
  log: string;
}

export interface PackageUpdate {
  name: string;
  current_version: string | null;
  new_version: string;
  arch: string | null;
  repository: string | null;
  security: boolean;
  restarts_docker: boolean;
  /** Pinned with `apt-mark hold`; the package manager will not upgrade it. */
  held?: boolean;
}

export interface PatchState {
  package_manager: string;
  packages: PackageUpdate[];
  reboot_required: boolean;
  checked_at: string | null;
  error: string | null;
  /** Unix seconds of the host's last boot, from the latest check. */
  booted_at?: number | null;
}

export interface PatchJob {
  id: string;
  server_id: string;
  kind: "check" | "update_package" | "update_all" | "reboot" | "configure_auto_updates";
  package: string | null;
  status: "running" | "success" | "failed";
  started_at: string;
  finished_at: string | null;
  log: string;
}

export interface PatchingResponse {
  settings: MaintenanceSettings;
  state: PatchState | null;
  running_job: PatchJob | null;
  recent_jobs: PatchJob[];
  supports_auto_updates: boolean;
}

export interface ServerSummary {
  server_id: string;
  name: string;
  host: string;
  resource_count: number;
  update_count: number;
  security_update_count: number;
  reboot_required: boolean;
  last_patch_check_at: string | null;
  status: "reachable" | "attention" | "unreachable" | "unknown";
}

export interface JobStarted {
  id: string;
}

export const serverOpsApi = {
  summary: (token?: string) =>
    apiRequest<ServerSummary[]>("/server-ops/summary", {}, token),

  listImages: (serverId: string, token?: string) =>
    apiRequest<ImageInventory>(`/servers/${serverId}/images`, {}, token),

  deleteImage: (serverId: string, imageId: string, token?: string) =>
    apiRequest<void>(
      `/servers/${serverId}/images/${encodeURIComponent(imageId)}`,
      { method: "DELETE" },
      token,
    ),

  getSettings: (serverId: string, token?: string) =>
    apiRequest<MaintenanceSettings>(`/servers/${serverId}/maintenance`, {}, token),

  updateSettings: (serverId: string, data: UpdateMaintenanceSettings, token?: string) =>
    apiRequest<MaintenanceSettings>(
      `/servers/${serverId}/maintenance`,
      { method: "PUT", body: JSON.stringify(data) },
      token,
    ),

  listCleanupRuns: (serverId: string, token?: string) =>
    apiRequest<CleanupRun[]>(`/servers/${serverId}/cleanup/runs`, {}, token),

  runCleanup: (serverId: string, token?: string) =>
    apiRequest<JobStarted>(`/servers/${serverId}/cleanup`, { method: "POST" }, token),

  getPatching: (serverId: string, token?: string) =>
    apiRequest<PatchingResponse>(`/servers/${serverId}/patching`, {}, token),

  checkUpdates: (serverId: string, token?: string) =>
    apiRequest<JobStarted>(`/servers/${serverId}/patching/check`, { method: "POST" }, token),

  /** Update one package, or every package when `pkg` is omitted */
  updatePackages: (serverId: string, pkg?: string, token?: string) =>
    apiRequest<JobStarted>(
      `/servers/${serverId}/patching/update`,
      { method: "POST", body: JSON.stringify({ package: pkg ?? null }) },
      token,
    ),

  reboot: (serverId: string, token?: string) =>
    apiRequest<JobStarted>(`/servers/${serverId}/reboot`, { method: "POST" }, token),
};
