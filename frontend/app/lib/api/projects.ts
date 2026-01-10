/**
 * Projects API module.
 * Handles project CRUD operations.
 */

import { apiRequest } from "./core";
import type {
  Project,
  ProjectWithApps,
  CreateProjectRequest,
  UpdateProjectRequest,
} from "@/types/api";

export const projectsApi = {
  /**
   * List all projects, optionally filtered by team.
   * @param teamId - If provided, filter projects by team. Empty string gets unassigned projects.
   */
  getProjects: (teamId?: string) => {
    const params = new URLSearchParams();
    if (teamId !== undefined) {
      params.set("team_id", teamId);
    }
    const query = params.toString();
    return apiRequest<Project[]>(`/projects${query ? `?${query}` : ""}`);
  },

  /** Get a single project with its apps, databases, and services */
  getProject: (id: string) => apiRequest<ProjectWithApps>(`/projects/${id}`),

  /** Create a new project */
  createProject: (data: CreateProjectRequest, token?: string) =>
    apiRequest<Project>(
      "/projects",
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Update an existing project */
  updateProject: (id: string, data: UpdateProjectRequest, token?: string) =>
    apiRequest<Project>(
      `/projects/${id}`,
      {
        method: "PUT",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Delete a project */
  deleteProject: (id: string, token?: string) =>
    apiRequest<void>(
      `/projects/${id}`,
      {
        method: "DELETE",
      },
      token
    ),
};
