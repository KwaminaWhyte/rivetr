/**
 * Services API module.
 * Handles Docker Compose services and service templates.
 */

import { apiRequest, getStoredToken } from "./core";
import type {
  Service,
  ContainerStats,
  CreateServiceRequest,
  UpdateServiceRequest,
  ServiceLogEntry,
  ServiceTemplate,
  TemplateCategory,
  DeployTemplateRequest,
  DeployTemplateResponse,
  ServiceDeployment,
  ServiceDeploymentLog,
  ServiceDeployConfig,
  UpdateServiceDeployConfigRequest,
  ServiceContainer,
  ServiceContainerLogLine,
} from "@/types/api";

/** Options for listing services */
export interface GetServicesOptions {
  /** Filter by team ID (optional) */
  teamId?: string;
}

export const servicesApi = {
  // -------------------------------------------------------------------------
  // Docker Compose Services
  // -------------------------------------------------------------------------

  /** List all services */
  getServices: (options?: GetServicesOptions, token?: string) => {
    const params = new URLSearchParams();
    if (options?.teamId !== undefined) {
      params.set("team_id", options.teamId);
    }
    const queryString = params.toString();
    const url = queryString ? `/services?${queryString}` : "/services";
    return apiRequest<Service[]>(url, {}, token);
  },

  /** Get a single service by ID */
  getService: (id: string, token?: string) =>
    apiRequest<Service>(`/services/${id}`, {}, token),

  /** Create a new service */
  createService: (data: CreateServiceRequest, token?: string) =>
    apiRequest<Service>(
      "/services",
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Update an existing service */
  updateService: (id: string, data: UpdateServiceRequest, token?: string) =>
    apiRequest<Service>(
      `/services/${id}`,
      {
        method: "PUT",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Delete a service */
  deleteService: (id: string, token?: string) =>
    apiRequest<void>(
      `/services/${id}`,
      {
        method: "DELETE",
      },
      token
    ),

  /** Start a service */
  startService: (id: string, token?: string) =>
    apiRequest<Service>(
      `/services/${id}/start`,
      {
        method: "POST",
      },
      token
    ),

  /** Stop a service */
  stopService: (id: string, token?: string) =>
    apiRequest<Service>(
      `/services/${id}/stop`,
      {
        method: "POST",
      },
      token
    ),

  /** Restart a service (stop then start) */
  restartService: (id: string, token?: string) =>
    apiRequest<Service>(
      `/services/${id}/restart`,
      {
        method: "POST",
      },
      token
    ),

  /** Deploy a service: pull or build its images and recreate changed containers */
  deployService: (id: string, token?: string) =>
    apiRequest<ServiceDeployment>(`/services/${id}/deploy`, { method: "POST" }, token),

  /** Deployment history of a service, newest first */
  getServiceDeployments: (id: string, limit = 50, token?: string) =>
    apiRequest<ServiceDeployment[]>(`/services/${id}/deployments?limit=${limit}`, {}, token),

  /** One service deployment */
  getServiceDeployment: (id: string, deploymentId: string, token?: string) =>
    apiRequest<ServiceDeployment>(`/services/${id}/deployments/${deploymentId}`, {}, token),

  /** Stored log lines of a service deployment */
  getServiceDeploymentLogs: (id: string, deploymentId: string, token?: string) =>
    apiRequest<ServiceDeploymentLog[]>(
      `/services/${id}/deployments/${deploymentId}/logs`,
      {},
      token
    ),

  /** Redeploy the images a previous deployment ran, pinned by digest */
  rollbackServiceDeployment: (id: string, deploymentId: string, token?: string) =>
    apiRequest<ServiceDeployment>(
      `/services/${id}/deployments/${deploymentId}/rollback`,
      { method: "POST" },
      token
    ),

  /** Containers of a service with state, health and resource usage */
  getServiceContainers: (id: string, token?: string) =>
    apiRequest<ServiceContainer[]>(`/services/${id}/containers`, {}, token),

  /** Restart one container of a service */
  restartServiceContainer: (id: string, container: string, token?: string) =>
    apiRequest<void>(
      `/services/${id}/containers/${encodeURIComponent(container)}/restart`,
      { method: "POST" },
      token
    ),

  /** Recent log lines of one container */
  getServiceContainerLogs: (id: string, container: string, lines = 200, token?: string) =>
    apiRequest<ServiceContainerLogLine[]>(
      `/services/${id}/containers/${encodeURIComponent(container)}/logs?lines=${lines}`,
      {},
      token
    ),

  /** WebSocket URL for a shell in one container */
  getServiceContainerTerminalWsUrl: (id: string, container: string, token: string): string => {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    return `${protocol}//${window.location.host}/api/services/${id}/containers/${encodeURIComponent(container)}/terminal?token=${encodeURIComponent(token)}`;
  },

  /** Build source, images, registry and hooks of a service */
  getServiceDeployConfig: (id: string, token?: string) =>
    apiRequest<ServiceDeployConfig>(`/services/${id}/deploy-config`, {}, token),

  /** Update the deploy config (omitted fields keep their value) */
  updateServiceDeployConfig: (
    id: string,
    data: UpdateServiceDeployConfigRequest,
    token?: string
  ) =>
    apiRequest<ServiceDeployConfig>(
      `/services/${id}/deploy-config`,
      { method: "PUT", body: JSON.stringify(data) },
      token
    ),

  /** Get aggregated resource stats (CPU, RAM, network) for a service's containers */
  getServiceStats: (id: string, token?: string) =>
    apiRequest<ContainerStats>(`/services/${id}/stats`, {}, token),

  /** Get service logs */
  getServiceLogs: (id: string, lines = 100, token?: string) =>
    apiRequest<ServiceLogEntry[]>(
      `/services/${id}/logs?lines=${lines}`,
      {},
      token
    ),

  /** Get SSE URL for service logs streaming */
  getServiceLogsStreamUrl: (id: string, token?: string): string => {
    const base = `/api/services/${id}/logs/stream`;
    return token ? `${base}?token=${encodeURIComponent(token)}` : base;
  },

  // -------------------------------------------------------------------------
  // Service Templates
  // -------------------------------------------------------------------------

  /** List all templates, optionally filtered by category */
  getTemplates: (category?: TemplateCategory, token?: string) => {
    const params = category ? `?category=${category}` : "";
    return apiRequest<ServiceTemplate[]>(`/templates${params}`, {}, token);
  },

  /** Get a single template by ID */
  getTemplate: (id: string, token?: string) =>
    apiRequest<ServiceTemplate>(`/templates/${id}`, {}, token),

  /** Get all available template categories */
  getTemplateCategories: (token?: string) =>
    apiRequest<string[]>("/templates/categories", {}, token),

  /** Deploy a template as a new service */
  deployTemplate: (id: string, data: DeployTemplateRequest, token?: string) =>
    apiRequest<DeployTemplateResponse>(
      `/templates/${id}/deploy`,
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Check whether a port is available (not in use by any service or database) */
  checkPort: (port: number, token?: string) =>
    apiRequest<{ available: boolean; conflict?: string }>(
      `/services/check-port?port=${port}`,
      {},
      token
    ),

  /** Import a SQL dump file into a running database container inside the service */
  importServiceDb: async (
    serviceId: string,
    file: File,
    containerName: string,
    database: string,
    token?: string
  ): Promise<{ success: boolean; message: string; service_id: string; container: string }> => {
    const authToken = token || getStoredToken();
    const headers: Record<string, string> = {};
    if (authToken) {
      headers["Authorization"] = `Bearer ${authToken}`;
    }
    const formData = new FormData();
    formData.append("file", file);
    formData.append("container_name", containerName);
    formData.append("database", database);
    const response = await fetch(`/api/services/${serviceId}/import-db`, {
      method: "POST",
      headers,
      credentials: "include",
      body: formData,
    });
    if (!response.ok) {
      const error = await response.json().catch(() => ({
        error: `Import failed: ${response.status}`,
      }));
      throw new Error(error.error || `Import failed: ${response.status}`);
    }
    return response.json();
  },

  /** Preview the final resolved compose YAML for a service (dry-run, no DB writes) */
  previewCompose: (id: string, token?: string) =>
    apiRequest<{ compose_yaml: string }>(`/services/${id}/preview-compose`, {}, token),

  /** Get auto-generated magic variables (SERVICE_PASSWORD_*, SERVICE_BASE64_*) for a service */
  getServiceGeneratedVars: (id: string, token?: string) =>
    apiRequest<
      {
        id: string;
        service_id: string;
        key: string;
        value: string;
        created_at: string;
        updated_at: string;
      }[]
    >(`/services/${id}/generated-vars`, {}, token),
};
