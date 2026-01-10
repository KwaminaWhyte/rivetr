/**
 * Services API module.
 * Handles Docker Compose services and service templates.
 */

import { apiRequest } from "./core";
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
};
