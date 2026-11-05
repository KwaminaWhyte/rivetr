/**
 * Small query hooks used by the shell (sidebar header, breadcrumbs, top bar).
 * They reuse the query keys and fetchers of the pages, so the shell never adds
 * a second request for data a page already loaded.
 */
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { serversApi, type Server } from "@/lib/api/servers";
import type { ResourceMatch } from "@/lib/resource-nav";
import type {
  App,
  ManagedDatabase,
  Project,
  ProjectEnvironment,
  ProjectWithApps,
  Service,
} from "@/types/api";

export interface ResourceInfo {
  name?: string;
  status?: string;
  projectId?: string | null;
  environmentId?: string | null;
}

export function useResourceInfo(match: ResourceMatch | null): ResourceInfo {
  const kind = match?.kind;
  const id = match?.id ?? "";

  const app = useQuery<App>({
    queryKey: ["app", id],
    queryFn: () => api.getApp(id),
    enabled: kind === "app",
  });
  const database = useQuery<ManagedDatabase>({
    queryKey: ["database", id],
    queryFn: () => api.getDatabase(id, true),
    enabled: kind === "database",
  });
  const service = useQuery<Service>({
    queryKey: ["service", id],
    queryFn: () => api.getService(id),
    enabled: kind === "service",
  });
  const servers = useServers(kind === "server");

  switch (kind) {
    case "app":
      return {
        name: app.data?.name,
        projectId: app.data?.project_id,
        environmentId: app.data?.environment_id,
      };
    case "database":
      return {
        name: database.data?.name,
        status: database.data?.status,
        projectId: database.data?.project_id,
      };
    case "service":
      return {
        name: service.data?.name,
        status: service.data?.status,
        projectId: service.data?.project_id,
      };
    case "server": {
      const server = servers.data?.find((s) => s.id === id);
      return { name: server?.name, status: server?.status };
    }
    default:
      return {};
  }
}

export function useProjectDetail(projectId?: string | null) {
  return useQuery<ProjectWithApps>({
    queryKey: ["project", projectId],
    queryFn: () => api.getProject(projectId!),
    enabled: !!projectId,
  });
}

export function useProjectEnvironments(projectId?: string | null) {
  return useQuery<ProjectEnvironment[]>({
    queryKey: ["environments", projectId],
    queryFn: () => api.getEnvironments(projectId!),
    enabled: !!projectId,
  });
}

/** Same key and fetcher as the command palette. */
export function useProjectList(teamId: string | null, enabled = true) {
  return useQuery<Project[]>({
    queryKey: ["projects", teamId],
    queryFn: () => api.getProjects(teamId ?? undefined),
    enabled,
    staleTime: 30000,
  });
}

/** Same key and fetcher as the servers page. */
export function useServers(enabled = true) {
  return useQuery<Server[]>({
    queryKey: ["servers"],
    queryFn: () => serversApi.list(),
    enabled,
    staleTime: 30000,
  });
}
