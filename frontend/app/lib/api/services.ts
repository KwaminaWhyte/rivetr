/**
 * Services API module.
 * Handles Docker Compose services and service templates.
 */

import { apiRequest, getStoredToken } from "./core";
import type {
  Service,
  CreateServiceRequest,
  UpdateServiceRequest,
  ServiceLogEntry,
  ServiceTemplate,
  TemplateCategory,
  DeployTemplateRequest,
  DeployTemplateResponse,
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

  /** Submit a community template suggestion */
  suggestTemplate: (
    data: {
      name: string;
      description: string;
      docker_image: string;
      category: string;
      website_url?: string;
      notes?: string;
    },
    token?: string
  ) =>
    apiRequest<import("../../types/apps").TemplateSuggestion>(
      "/templates/suggest",
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),

  /** List all template suggestions (admin) */
  listSuggestions: (token?: string) =>
    apiRequest<import("../../types/apps").TemplateSuggestion[]>(
      "/templates/suggestions",
      {},
      token
    ),

  /** Approve a template suggestion and seed it */
  approveSuggestion: (id: string, token?: string) =>
    apiRequest<void>(
      `/templates/suggestions/${id}/approve`,
      { method: "PUT" },
      token
    ),
};
