import { useMemo } from "react";
import { useQueries, useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { getPrimaryDomain } from "@/lib/utils";
import type {
  App,
  DependencyGraph,
  ManagedDatabase,
  ProjectEnvironment,
  ProjectWithApps,
  Service,
} from "@/types/api";

export type CanvasKind = "app" | "database" | "service";

export interface CanvasResource {
  id: string;
  kind: CanvasKind;
  name: string;
  href: string;
  status: string;
  /** "Dockerfile · main", "PostgreSQL 16", "Docker Compose" */
  source: string;
  url: string | null;
  serverId: string | null;
  projectId: string | null;
  /** Environment id for apps; null puts it in the "Shared" box. */
  environmentId: string | null;
}

export interface CanvasEnvironment {
  id: string;
  name: string;
  isDefault: boolean;
  resources: CanvasResource[];
}

export interface CanvasProject {
  id: string;
  name: string;
  description: string | null;
  /** False for the synthetic "Unassigned" group. */
  real: boolean;
  environments: CanvasEnvironment[];
  resourceCount: number;
}

const BUILD_LABEL: Record<string, string> = {
  dockerfile: "Dockerfile",
  nixpacks: "Nixpacks",
  railpack: "Railpack",
  cnb: "Buildpacks",
  staticsite: "Static site",
};

const DB_LABEL: Record<string, string> = {
  postgres: "PostgreSQL",
  mysql: "MySQL",
  mariadb: "MariaDB",
  mongodb: "MongoDB",
  redis: "Redis",
  dragonfly: "Dragonfly",
  keydb: "KeyDB",
  clickhouse: "ClickHouse",
  libsql: "libSQL",
};

function appSource(app: App): string {
  if (app.docker_image) {
    return `Image · ${app.docker_image}${app.docker_image_tag ? `:${app.docker_image_tag}` : ""}`;
  }
  const build = BUILD_LABEL[app.build_type] ?? "App";
  if (app.deployment_source === "upload") return `${build} · upload`;
  return app.branch ? `${build} · ${app.branch}` : build;
}

export function toAppResource(app: App, statuses: Record<string, string>): CanvasResource {
  const domain = getPrimaryDomain(app);
  return {
    id: app.id,
    kind: "app",
    name: app.name,
    href: `/apps/${app.id}`,
    status: statuses[app.id] ?? "stopped",
    source: appSource(app),
    url: domain ? `https://${domain}` : null,
    serverId: app.server_id ?? null,
    projectId: app.project_id,
    environmentId: app.environment_id,
  };
}

export function toDatabaseResource(db: ManagedDatabase): CanvasResource {
  return {
    id: db.id,
    kind: "database",
    name: db.name,
    href: `/databases/${db.id}`,
    status: db.status,
    source: `${DB_LABEL[db.db_type] ?? db.db_type} ${db.version}`.trim(),
    url: null,
    serverId: null,
    projectId: db.project_id,
    environmentId: null,
  };
}

export function toServiceResource(svc: Service): CanvasResource {
  return {
    id: svc.id,
    kind: "service",
    name: svc.name,
    href: `/services/${svc.id}`,
    status: svc.status,
    source: "Docker Compose",
    url: svc.domain ? `https://${svc.domain}` : null,
    serverId: null,
    projectId: svc.project_id,
    environmentId: null,
  };
}

export const SHARED_ENV_ID = "__shared__";

function groupIntoEnvironments(
  resources: CanvasResource[],
  envs: ProjectEnvironment[]
): CanvasEnvironment[] {
  const out: CanvasEnvironment[] = envs.map((e) => ({
    id: e.id,
    name: e.name,
    isDefault: e.is_default,
    resources: [],
  }));
  const byId = new Map(out.map((e) => [e.id, e]));
  const fallback = out.find((e) => e.isDefault) ?? out[0];
  const shared: CanvasEnvironment = { id: SHARED_ENV_ID, name: "Shared", isDefault: false, resources: [] };

  for (const r of resources) {
    if (r.kind === "app") {
      const env = (r.environmentId && byId.get(r.environmentId)) || fallback;
      (env ?? shared).resources.push(r);
    } else {
      shared.resources.push(r);
    }
  }
  if (shared.resources.length > 0 || out.length === 0) out.push(shared);
  return out;
}

/**
 * Everything the canvas needs, built from the same queries (and query keys)
 * the projects page and command palette already use.
 */
export function useCanvasData(teamId: string | null) {
  const teamParam = teamId ?? undefined;

  const projectsQuery = useQuery<ProjectWithApps[]>({
    queryKey: ["projects-with-apps", teamId],
    queryFn: async () => {
      const list = await api.getProjects(teamParam);
      return Promise.all(
        list.map((p) =>
          api
            .getProject(p.id)
            .catch(() => ({ ...p, apps: [], databases: [], services: [] }) as ProjectWithApps)
        )
      );
    },
    refetchInterval: 15000,
  });
  const appsQuery = useQuery<App[]>({
    queryKey: ["apps", teamId],
    queryFn: () => api.getApps({ teamId: teamParam }),
  });
  const databasesQuery = useQuery<ManagedDatabase[]>({
    queryKey: ["databases", teamId],
    queryFn: () => api.getDatabases({ teamId: teamParam }),
    refetchInterval: 15000,
  });
  const servicesQuery = useQuery<Service[]>({
    queryKey: ["services", teamId],
    queryFn: () => api.getServices(teamParam ? { teamId: teamParam } : undefined),
    refetchInterval: 15000,
  });
  const statusesQuery = useQuery<Record<string, string>>({
    queryKey: ["app-statuses", teamId],
    queryFn: () => api.getAppStatuses({ teamId: teamParam }),
    refetchInterval: 10000,
  });

  const projectList = projectsQuery.data ?? [];
  const envQueries = useQueries({
    queries: projectList.map((p) => ({
      queryKey: ["environments", p.id],
      queryFn: () => api.getEnvironments(p.id),
      staleTime: 60000,
    })),
  });
  const graphQueries = useQueries({
    queries: projectList.map((p) => ({
      queryKey: ["dependency-graph", p.id],
      queryFn: () => api.getDependencyGraph(p.id),
      staleTime: 30000,
    })),
  });

  const envData = envQueries.map((q) => q.data);
  const graphData = graphQueries.map((q) => q.data);
  const statuses = statusesQuery.data;

  const projects = useMemo<CanvasProject[]>(() => {
    const st = statuses ?? {};
    const out: CanvasProject[] = projectList.map((p, i) => {
      const resources = [
        ...p.apps.map((a) => toAppResource(a, st)),
        ...(p.databases ?? []).map(toDatabaseResource),
        ...(p.services ?? []).map(toServiceResource),
      ];
      return {
        id: p.id,
        name: p.name,
        description: p.description,
        real: true,
        environments: groupIntoEnvironments(resources, envData[i] ?? []),
        resourceCount: resources.length,
      };
    });

    const unassigned = [
      ...(appsQuery.data ?? []).filter((a) => !a.project_id).map((a) => toAppResource(a, st)),
      ...(databasesQuery.data ?? []).filter((d) => !d.project_id).map(toDatabaseResource),
      ...(servicesQuery.data ?? []).filter((s) => !s.project_id).map(toServiceResource),
    ];
    if (unassigned.length > 0) {
      out.push({
        id: "__unassigned__",
        name: "Unassigned",
        description: "Resources not in a project yet",
        real: false,
        environments: [
          { id: "__unassigned_env__", name: "No project", isDefault: false, resources: unassigned },
        ],
        resourceCount: unassigned.length,
      });
    }
    return out;
    // envData / graphData are new arrays each render; key on their contents.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectList, statuses, appsQuery.data, databasesQuery.data, servicesQuery.data, JSON.stringify(envData.map((e) => e?.map((x) => x.id)))]);

  const edges = useMemo(() => {
    const list: { from: string; to: string; label: string }[] = [];
    for (const g of graphData as (DependencyGraph | undefined)[]) {
      if (g) list.push(...g.edges);
    }
    return list;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [JSON.stringify(graphData.map((g) => g?.edges ?? []))]);

  return {
    projects,
    edges,
    isLoading: projectsQuery.isLoading,
    error: projectsQuery.error,
    refetch: projectsQuery.refetch,
  };
}
