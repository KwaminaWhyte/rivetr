/**
 * Environments API module.
 * Handles project environment CRUD and environment-scoped env vars.
 */

import { apiRequest } from "./core";
import type {
  ProjectEnvironment,
  CreateEnvironmentRequest,
  UpdateEnvironmentRequest,
  EnvironmentEnvVar,
  CreateEnvironmentEnvVarRequest,
  UpdateEnvironmentEnvVarRequest,
  CloneEnvironmentRequest,
  CloneEnvironmentResponse,
} from "@/types/api";

export const environmentsApi = {
  /** List all environments for a project */
  getEnvironments: (projectId: string) =>
    apiRequest<ProjectEnvironment[]>(`/projects/${projectId}/environments`),

  /** Create a new environment for a project */
  createEnvironment: (projectId: string, data: CreateEnvironmentRequest) =>
    apiRequest<ProjectEnvironment>(`/projects/${projectId}/environments`, {
      method: "POST",
      body: JSON.stringify(data),
    }),

  /** Update an environment */
  updateEnvironment: (id: string, data: UpdateEnvironmentRequest) =>
    apiRequest<ProjectEnvironment>(`/environments/${id}`, {
      method: "PUT",
      body: JSON.stringify(data),
    }),

  /** Delete an environment */
  deleteEnvironment: (id: string) =>
    apiRequest<void>(`/environments/${id}`, {
      method: "DELETE",
    }),

  /** Clone an environment — duplicates all apps, env vars, volumes, databases, and services */
  cloneEnvironment: (
    projectId: string,
    envId: string,
    data: CloneEnvironmentRequest
  ) =>
    apiRequest<CloneEnvironmentResponse>(
      `/projects/${projectId}/environments/${envId}/clone`,
      {
        method: "POST",
        body: JSON.stringify(data),
      }
    ),

  /** List env vars for an environment */
  getEnvironmentEnvVars: (envId: string, reveal = false) =>
    apiRequest<EnvironmentEnvVar[]>(
      `/environments/${envId}/env-vars${reveal ? "?reveal=true" : ""}`
    ),

  /** Create an env var for an environment */
  createEnvironmentEnvVar: (
    envId: string,
    data: CreateEnvironmentEnvVarRequest
  ) =>
    apiRequest<EnvironmentEnvVar>(`/environments/${envId}/env-vars`, {
      method: "POST",
      body: JSON.stringify(data),
    }),

  /** Update an env var */
  updateEnvironmentEnvVar: (
    envId: string,
    varId: string,
    data: UpdateEnvironmentEnvVarRequest
  ) =>
    apiRequest<EnvironmentEnvVar>(
      `/environments/${envId}/env-vars/${varId}`,
      {
        method: "PUT",
        body: JSON.stringify(data),
      }
    ),

  /** Delete an env var */
  deleteEnvironmentEnvVar: (envId: string, varId: string) =>
    apiRequest<void>(`/environments/${envId}/env-vars/${varId}`, {
      method: "DELETE",
    }),
};
