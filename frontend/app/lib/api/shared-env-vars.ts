/**
 * Shared Environment Variables API module.
 *
 * Handles team-level and project-level shared env vars,
 * plus the resolved env vars endpoint for apps.
 */

import { apiRequest } from "./core";
import type {
  TeamEnvVar,
  ProjectEnvVar,
  ResolvedEnvVar,
  CreateTeamEnvVarRequest,
  UpdateTeamEnvVarRequest,
  CreateProjectEnvVarRequest,
  UpdateProjectEnvVarRequest,
} from "@/types/api";

export const sharedEnvVarsApi = {
  // ---------------------------------------------------------------------------
  // Team Env Vars
  // ---------------------------------------------------------------------------

  /** List all team-level shared environment variables */
  getTeamEnvVars: (teamId: string, reveal = false) =>
    apiRequest<TeamEnvVar[]>(
      `/teams/${teamId}/env-vars${reveal ? "?reveal=true" : ""}`
    ),

  /** Create a team-level shared environment variable */
  createTeamEnvVar: (teamId: string, data: CreateTeamEnvVarRequest) =>
    apiRequest<TeamEnvVar>(`/teams/${teamId}/env-vars`, {
      method: "POST",
      body: JSON.stringify(data),
    }),

  /** Update a team-level shared environment variable */
  updateTeamEnvVar: (
    teamId: string,
    varId: string,
    data: UpdateTeamEnvVarRequest
  ) =>
    apiRequest<TeamEnvVar>(`/teams/${teamId}/env-vars/${varId}`, {
      method: "PUT",
      body: JSON.stringify(data),
    }),

  /** Delete a team-level shared environment variable */
  deleteTeamEnvVar: (teamId: string, varId: string) =>
    apiRequest<void>(`/teams/${teamId}/env-vars/${varId}`, {
      method: "DELETE",
    }),

  // ---------------------------------------------------------------------------
  // Project Env Vars
  // ---------------------------------------------------------------------------

  /** List all project-level shared environment variables */
  getProjectEnvVars: (projectId: string, reveal = false) =>
    apiRequest<ProjectEnvVar[]>(
      `/projects/${projectId}/env-vars${reveal ? "?reveal=true" : ""}`
    ),

  /** Create a project-level shared environment variable */
  createProjectEnvVar: (projectId: string, data: CreateProjectEnvVarRequest) =>
    apiRequest<ProjectEnvVar>(`/projects/${projectId}/env-vars`, {
      method: "POST",
      body: JSON.stringify(data),
    }),

  /** Update a project-level shared environment variable */
  updateProjectEnvVar: (
    projectId: string,
    varId: string,
    data: UpdateProjectEnvVarRequest
  ) =>
    apiRequest<ProjectEnvVar>(`/projects/${projectId}/env-vars/${varId}`, {
      method: "PUT",
      body: JSON.stringify(data),
    }),

  /** Delete a project-level shared environment variable */
  deleteProjectEnvVar: (projectId: string, varId: string) =>
    apiRequest<void>(`/projects/${projectId}/env-vars/${varId}`, {
      method: "DELETE",
    }),

  // ---------------------------------------------------------------------------
  // Resolved Env Vars
  // ---------------------------------------------------------------------------

  /**
   * Get the effective env vars for an app showing the full inheritance chain.
   * team → project → environment → app (highest priority wins).
   * Secrets are always masked as `***`.
   */
  getResolvedEnvVars: (appId: string) =>
    apiRequest<ResolvedEnvVar[]>(`/apps/${appId}/env-vars/resolved`),
};
