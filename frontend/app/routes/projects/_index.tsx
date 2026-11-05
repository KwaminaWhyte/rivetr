import { useState, useMemo, useEffect } from "react";
import { useSearchParams } from "react-router";
import { useQuery, useQueryClient, useMutation } from "@tanstack/react-query";
import { Box, Database, FolderKanban, LayoutGrid, Layers, List, Plus, Search, Settings, Workflow } from "lucide-react";
import { Link, useNavigate } from "react-router";
import { useQueries } from "@tanstack/react-query";
import { PageHeader, EmptyState, CardGridSkeleton, DataTable, ResourceStatus, ViewToggle, useStoredPreference, type DataColumn, type ViewOption } from "@/components/page";
import { ProjectsCanvas } from "@/components/canvas/projects-canvas";
import { useLocalPreference } from "@/lib/persisted-state";
import { api } from "@/lib/api";
import { useTeamContext } from "@/lib/team-context";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import type { ProjectHealth } from "@/components/project-card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { QueryError } from "@/components/query-error";
import type { App, DeploymentStatus, ManagedDatabase, ProjectEnvironment, ProjectWithApps } from "@/types/api";

export function meta() {
  return [
    { title: "Projects - Rivetr" },
    { name: "description", content: "Manage your projects and applications" },
  ];
}

type FilterTab = "all" | ProjectHealth;
type ViewMode = "list" | "grid" | "canvas";
const VIEW_MODES = ["list", "grid", "canvas"] as const;
const VIEW_OPTIONS: ViewOption<ViewMode>[] = [
  { value: "list", label: "List", icon: List },
  { value: "grid", label: "Grid", icon: LayoutGrid },
  { value: "canvas", label: "Canvas", icon: Workflow, badge: "Try it" },
];

const HEALTH_STATUS: Record<ProjectHealth, { status: string; label: string }> = {
  healthy: { status: "running", label: "Healthy" },
  issues: { status: "failed", label: "Needs attention" },
  building: { status: "building", label: "Building" },
};

function countLabel(n: number, word: string) {
  return `${n} ${word}${n === 1 ? "" : "s"}`;
}

function resourceTotal(p: ProjectWithApps) {
  return p.apps.length + (p.databases || []).length + (p.services || []).length;
}

/** Compact project card: name, description, counts, add and settings icons. */
function ProjectTile({
  project,
  envCount,
  health,
}: {
  project: ProjectWithApps;
  envCount: number;
  health: ProjectHealth;
}) {
  const total = resourceTotal(project);
  return (
    <div className="group relative flex flex-col gap-3 rounded-xl border bg-card p-4 shadow-xs transition-all hover:border-primary/30 hover:shadow-md">
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 items-start gap-3">
          <span className="inline-flex size-9 shrink-0 items-center justify-center rounded-lg border bg-muted/50 text-muted-foreground">
            <FolderKanban className="size-4" />
          </span>
          <div className="min-w-0">
            <Link
              to={`/projects/${project.id}`}
              className="block truncate font-medium outline-none after:absolute after:inset-0 hover:underline focus-visible:underline"
            >
              {project.name}
            </Link>
            <p className="truncate text-sm text-muted-foreground">
              {project.description || "No description"}
            </p>
          </div>
        </div>
        <div className="relative z-10 flex shrink-0 items-center gap-1 opacity-70 transition-opacity group-hover:opacity-100">
          <Button variant="ghost" size="icon-sm" asChild>
            <Link to={`/projects/${project.id}/apps/new`} aria-label={`Add a resource to ${project.name}`} title="Add resource">
              <Plus />
            </Link>
          </Button>
          <Button variant="ghost" size="icon-sm" asChild>
            <Link to={`/projects/${project.id}/environments`} aria-label={`${project.name} settings`} title="Environments">
              <Settings />
            </Link>
          </Button>
        </div>
      </div>
      <div className="flex items-center justify-between gap-2 text-xs text-muted-foreground">
        <span>
          {countLabel(Math.max(envCount, 1), "environment")} · {countLabel(total, "resource")}
        </span>
        {total > 0 && <ResourceStatus status={HEALTH_STATUS[health].status} label={HEALTH_STATUS[health].label} size="sm" />}
      </div>
    </div>
  );
}
const SORT_KEYS = ["attention", "name", "newest"] as const;
type SortKey = (typeof SORT_KEYS)[number];

const HEALTH_RANK: Record<ProjectHealth, number> = { issues: 0, building: 1, healthy: 2 };

function getProjectHealth(
  project: ProjectWithApps,
  appStatuses: Record<string, DeploymentStatus>,
  databaseStatuses: Record<string, string>
): ProjectHealth {
  const databases = project.databases || [];
  if (project.apps.length === 0 && databases.length === 0) return "healthy";

  const appStatusValues = project.apps.map((app) => appStatuses[app.id]);
  const dbStatusValues = databases.map((db) => databaseStatuses[db.id] || db.status);

  // Check for building status in apps
  if (appStatusValues.some((s) => s === "building" || s === "cloning" || s === "starting" || s === "checking" || s === "pending")) {
    return "building";
  }
  // Check for starting status in databases
  if (dbStatusValues.some((s) => s === "starting" || s === "pulling" || s === "pending")) {
    return "building";
  }
  // Check for issues in apps
  if (appStatusValues.some((s) => s === "failed" || s === "stopped")) {
    return "issues";
  }
  // Check for issues in databases
  if (dbStatusValues.some((s) => s === "failed" || s === "stopped")) {
    return "issues";
  }
  return "healthy";
}

export default function ProjectsPage() {
  const queryClient = useQueryClient();
  const { currentTeamId } = useTeamContext();
  const [activeTab, setActiveTab] = useState<FilterTab>("all");
  const [search, setSearch] = useState("");
  const [sort, setSort] = useLocalPreference<SortKey>("projects:sort", "attention", SORT_KEYS);
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false);
  const [searchParams, setSearchParams] = useSearchParams();
  const navigate = useNavigate();
  const [storedView, setStoredView] = useStoredPreference<ViewMode>("rivetr_projects_view", "grid", VIEW_MODES);
  const urlView = searchParams.get("view");
  const view: ViewMode = (VIEW_MODES as readonly string[]).includes(urlView ?? "") ? (urlView as ViewMode) : storedView;
  const setView = (v: ViewMode) => {
    setStoredView(v);
    const next = new URLSearchParams(searchParams);
    next.set("view", v);
    setSearchParams(next, { replace: true });
  };

  // "New project" action from the command palette links here with ?new=1
  useEffect(() => {
    if (searchParams.get("new") === "1") {
      setIsCreateDialogOpen(true);
      const next = new URLSearchParams(searchParams);
      next.delete("new");
      setSearchParams(next, { replace: true });
    }
  }, [searchParams, setSearchParams]);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [error, setError] = useState<string | null>(null);

  // Use React Query for data fetching
  const {
    data: projects = [],
    isLoading: projectsLoading,
    error: projectsError,
    refetch: refetchProjects,
  } = useQuery<ProjectWithApps[]>({
    queryKey: ["projects-with-apps", currentTeamId],
    queryFn: async () => {
      const projectList = await api.getProjects(currentTeamId ?? undefined);
      const projectsWithApps = await Promise.all(
        projectList.map((p) => api.getProject(p.id).catch(() => ({ ...p, apps: [], databases: [], services: [] } as ProjectWithApps)))
      );
      return projectsWithApps;
    },
    enabled: currentTeamId !== null,
    retry: 1,
  });

  const envQueries = useQueries({
    queries: projects.map((p) => ({
      queryKey: ["environments", p.id],
      queryFn: () => api.getEnvironments(p.id),
      staleTime: 60000,
    })),
  });
  const envCounts: Record<string, number> = {};
  projects.forEach((p, i) => {
    envCounts[p.id] = (envQueries[i]?.data as ProjectEnvironment[] | undefined)?.length ?? 0;
  });

  const { data: apps = [] } = useQuery<App[]>({
    queryKey: ["apps", currentTeamId],
    queryFn: () => api.getApps({ teamId: currentTeamId ?? undefined }),
    enabled: currentTeamId !== null,
  });

  const { data: databases = [] } = useQuery<ManagedDatabase[]>({
    queryKey: ["databases", currentTeamId],
    queryFn: () => api.getDatabases({ teamId: currentTeamId ?? undefined }),
    enabled: currentTeamId !== null,
  });

  // One request for every app's latest deployment status (was one call per app).
  const { data: appStatuses = {} } = useQuery<Record<string, DeploymentStatus>>({
    queryKey: ["app-statuses", currentTeamId],
    queryFn: async () => {
      const raw = await api.getAppStatuses({ teamId: currentTeamId ?? undefined });
      return raw as Record<string, DeploymentStatus>;
    },
    enabled: currentTeamId !== null,
  });

  // Database statuses from the database objects
  const databaseStatuses = useMemo(() => {
    const statuses: Record<string, string> = {};
    for (const db of databases) {
      statuses[db.id] = db.status;
    }
    return statuses;
  }, [databases]);

  // Filter by tab and search text, then sort (problems first by default)
  const filteredProjects = useMemo(() => {
    const q = search.trim().toLowerCase();
    const matchesSearch = (project: ProjectWithApps) =>
      !q ||
      [
        project.name,
        project.description ?? "",
        ...project.apps.map((a) => a.name),
        ...(project.databases || []).map((d) => d.name),
        ...(project.services || []).map((s) => s.name),
      ].some((text) => text.toLowerCase().includes(q));

    return projects
      .filter(
        (p) =>
          activeTab === "all" || getProjectHealth(p, appStatuses, databaseStatuses) === activeTab
      )
      .filter(matchesSearch)
      .sort((a, b) => {
        if (sort === "name") return a.name.localeCompare(b.name);
        if (sort === "newest") return b.created_at.localeCompare(a.created_at);
        const byHealth =
          HEALTH_RANK[getProjectHealth(a, appStatuses, databaseStatuses)] -
          HEALTH_RANK[getProjectHealth(b, appStatuses, databaseStatuses)];
        return byHealth !== 0 ? byHealth : a.name.localeCompare(b.name);
      });
  }, [projects, activeTab, search, sort, appStatuses, databaseStatuses]);

  // "4 projects · 12 apps · 6 databases" subtitle
  const totals = useMemo(() => {
    let appTotal = 0;
    let dbTotal = 0;
    for (const p of projects) {
      appTotal += p.apps.length;
      dbTotal += (p.databases || []).length;
    }
    return { apps: appTotal, databases: dbTotal };
  }, [projects]);

  // Count projects by status
  const statusCounts = useMemo(() => {
    const counts = { all: projects.length, healthy: 0, issues: 0, building: 0 };
    for (const project of projects) {
      const health = getProjectHealth(project, appStatuses, databaseStatuses);
      counts[health]++;
    }
    return counts;
  }, [projects, appStatuses, databaseStatuses]);

  // Create project mutation
  const createProjectMutation = useMutation({
    mutationFn: async () => {
      if (!name.trim()) {
        throw new Error("Project name is required");
      }
      return api.createProject({
        name: name.trim(),
        description: description.trim() || undefined,
        team_id: currentTeamId ?? undefined,
      });
    },
    onSuccess: () => {
      setIsCreateDialogOpen(false);
      setName("");
      setDescription("");
      setError(null);
      queryClient.invalidateQueries({ queryKey: ["projects-with-apps"] });
    },
    onError: (err: Error) => {
      setError(err.message);
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    createProjectMutation.mutate();
  };

  const listColumns: DataColumn<ProjectWithApps>[] = [
    {
      id: "name",
      header: "Project",
      cell: (p) => (
        <div className="flex min-w-0 items-center gap-3">
          <span className="inline-flex size-8 shrink-0 items-center justify-center rounded-lg border bg-muted/50 text-muted-foreground">
            <FolderKanban className="size-4" />
          </span>
          <div className="min-w-0">
            <div className="truncate font-medium">{p.name}</div>
            <div className="truncate text-xs text-muted-foreground">{p.description || "No description"}</div>
          </div>
        </div>
      ),
    },
    {
      id: "resources",
      header: "Resources",
      showFrom: "md",
      cell: (p) => (
        <span className="inline-flex items-center gap-3 text-xs text-muted-foreground">
          <span className="inline-flex items-center gap-1" title="Apps"><Box className="size-3.5" />{p.apps.length}</span>
          <span className="inline-flex items-center gap-1" title="Databases"><Database className="size-3.5" />{(p.databases || []).length}</span>
          <span className="inline-flex items-center gap-1" title="Services"><Layers className="size-3.5" />{(p.services || []).length}</span>
        </span>
      ),
    },
    {
      id: "envs",
      header: "Environments",
      showFrom: "lg",
      cell: (p) => <span className="text-sm text-muted-foreground">{Math.max(envCounts[p.id] ?? 0, 1)}</span>,
    },
    {
      id: "health",
      header: "Status",
      cell: (p) => {
        if (resourceTotal(p) === 0) return <ResourceStatus status="stopped" label="Empty" size="sm" />;
        const h = getProjectHealth(p, appStatuses, databaseStatuses);
        return <ResourceStatus status={HEALTH_STATUS[h].status} label={HEALTH_STATUS[h].label} size="sm" />;
      },
    },
    {
      id: "created",
      header: "Created",
      showFrom: "lg",
      cell: (p) => <span className="text-sm text-muted-foreground">{new Date(p.created_at).toLocaleDateString()}</span>,
    },
  ];

  return (
    <div className="space-y-5">
      <PageHeader
        title="Projects"
        description={
          projects.length > 0
            ? `${countLabel(projects.length, "project")} · ${countLabel(totals.apps, "app")} · ${countLabel(totals.databases, "database")}`
            : "Group related apps, databases and services"
        }
        actions={
          <Button onClick={() => setIsCreateDialogOpen(true)}>
            <Plus className="h-4 w-4" />
            New Project
          </Button>
        }
      />

      <Dialog open={isCreateDialogOpen} onOpenChange={(open) => {
        setIsCreateDialogOpen(open);
        if (!open) {
          setName("");
          setDescription("");
          setError(null);
        }
      }}>
        <DialogContent>
          <form onSubmit={handleSubmit}>
            <DialogHeader>
              <DialogTitle>Create New Project</DialogTitle>
              <DialogDescription>
                Projects help you organize related applications together.
              </DialogDescription>
            </DialogHeader>
            {error && (
              <div className="p-3 rounded-md bg-destructive/10 text-destructive text-sm">
                {error}
              </div>
            )}
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <Label htmlFor="project-name">Name</Label>
                <Input
                  id="project-name"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="my-project"
                  required
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="project-description">Description</Label>
                <Textarea
                  id="project-description"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  placeholder="A brief description of your project..."
                  rows={3}
                />
              </div>
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => setIsCreateDialogOpen(false)}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={createProjectMutation.isPending}>
                {createProjectMutation.isPending ? "Creating..." : "Create Project"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {projectsError && projects.length === 0 && view !== "canvas" && (
        <QueryError
          what="projects"
          error={projectsError}
          onRetry={() => refetchProjects()}
        />
      )}

      {/* Toolbar: search, status filter, sort, view toggle */}
      <div className="flex flex-wrap items-center gap-2">
        {view !== "canvas" && (
          <>
            <div className="relative min-w-0 flex-1 sm:max-w-xs">
              <Search
                className="pointer-events-none absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground"
                aria-hidden="true"
              />
              <Input
                aria-label="Search projects"
                placeholder="Search projects"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="pl-8"
              />
            </div>
            {apps.length > 0 && (
              <Select value={activeTab} onValueChange={(v) => setActiveTab(v as FilterTab)}>
                <SelectTrigger aria-label="Filter by status" className="w-44">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">All ({statusCounts.all})</SelectItem>
                  <SelectItem value="healthy">Healthy ({statusCounts.healthy})</SelectItem>
                  <SelectItem value="issues">Needs attention ({statusCounts.issues})</SelectItem>
                  <SelectItem value="building">Building ({statusCounts.building})</SelectItem>
                </SelectContent>
              </Select>
            )}
            <Select value={sort} onValueChange={(v) => setSort(v as SortKey)}>
              <SelectTrigger aria-label="Sort projects" className="w-40">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="attention">Sort: attention</SelectItem>
                <SelectItem value="name">Sort: name</SelectItem>
                <SelectItem value="newest">Sort: newest</SelectItem>
              </SelectContent>
            </Select>
          </>
        )}
        <ViewToggle className="ml-auto" value={view} options={VIEW_OPTIONS} onChange={setView} />
      </div>

      {view === "canvas" ? (
        <ProjectsCanvas
          className="h-[calc(100svh-15rem)] min-h-[480px]"
          onListView={() => setView("list")}
        />
      ) : projectsLoading ? (
        <CardGridSkeleton count={6} />
      ) : projectsError && projects.length === 0 ? null : filteredProjects.length === 0 ? (
        projects.length === 0 ? (
          <EmptyState
            bordered
            icon={<FolderKanban />}
            title="No projects yet"
            description="Create your first project to organize your apps, databases and services."
            action={
              <Button onClick={() => setIsCreateDialogOpen(true)}>
                <Plus className="h-4 w-4" />
                Create Project
              </Button>
            }
          />
        ) : (
          <EmptyState
            bordered
            compact
            icon={<Search />}
            title="No matches"
            description="No projects match your search or filter."
          />
        )
      ) : view === "list" ? (
        <DataTable
          columns={listColumns}
          rows={filteredProjects}
          getRowKey={(p) => p.id}
          onRowClick={(p) => navigate(`/projects/${p.id}`)}
        />
      ) : (
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {filteredProjects.map((project) => (
            <ProjectTile
              key={project.id}
              project={project}
              envCount={envCounts[project.id] ?? 0}
              health={getProjectHealth(project, appStatuses, databaseStatuses)}
            />
          ))}
        </div>
      )}
    </div>
  );
}
