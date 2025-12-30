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
  /** List all projects */
  getProjects: () => apiRequest<Project[]>("/projects"),

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
