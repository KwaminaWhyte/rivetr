import { useState, useMemo, useEffect } from "react";
import { useSearchParams } from "react-router";
import { useQuery, useQueryClient, useMutation } from "@tanstack/react-query";
import { Plus, Search } from "lucide-react";
import { api } from "@/lib/api";
import { useTeamContext } from "@/lib/team-context";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { ProjectCard, type ProjectHealth } from "@/components/project-card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { QueryError } from "@/components/query-error";
import type { App, DeploymentStatus, ManagedDatabase, ProjectWithApps } from "@/types/api";

export function meta() {
  return [
    { title: "Projects - Rivetr" },
    { name: "description", content: "Manage your projects and applications" },
  ];
}

type FilterTab = "all" | ProjectHealth;
type SortKey = "attention" | "name" | "newest";

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
  const [sort, setSort] = useState<SortKey>("attention");
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false);
  const [searchParams, setSearchParams] = useSearchParams();

  // "New project" action from the command palette links here with ?new=1
  useEffect(() => {
    if (searchParams.get("new") === "1") {
      setIsCreateDialogOpen(true);
      setSearchParams({}, { replace: true });
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

  // Show loading state
  if (projectsLoading) {
    return (
      <div className="space-y-6">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Projects</h1>
          <p className="text-sm text-muted-foreground">
            Group related apps, databases and services
          </p>
        </div>
        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {[1, 2, 3].map((i) => (
            <Card key={i}>
              <CardContent className="space-y-3 p-4">
                <div className="flex items-center justify-between">
                  <Skeleton className="h-5 w-32" />
                  <Skeleton className="h-6 w-20 rounded-full" />
                </div>
                <Skeleton className="h-9 w-full" />
                <Skeleton className="h-9 w-full" />
                <Skeleton className="h-9 w-full" />
              </CardContent>
            </Card>
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Projects</h1>
          <p className="text-sm text-muted-foreground">
            {projects.length > 0
              ? `${projects.length} project${projects.length !== 1 ? "s" : ""} · ${totals.apps} app${totals.apps !== 1 ? "s" : ""} · ${totals.databases} database${totals.databases !== 1 ? "s" : ""}`
              : "Group related apps, databases and services"}
          </p>
        </div>

        <Dialog open={isCreateDialogOpen} onOpenChange={(open) => {
          setIsCreateDialogOpen(open);
          if (!open) {
            setName("");
            setDescription("");
            setError(null);
          }
        }}>
          <DialogTrigger asChild>
            <Button>
              <Plus className="mr-2 h-4 w-4" />
              New Project
            </Button>
          </DialogTrigger>
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
      </div>

      {projectsError && projects.length === 0 && (
        <QueryError
          what="projects"
          error={projectsError}
          onRetry={() => refetchProjects()}
        />
      )}

      {/* Toolbar: status filter (hidden until there are apps to filter, U4), search, sort */}
      {projects.length > 0 && (
        <div className="flex flex-wrap items-center justify-between gap-3">
          {apps.length > 0 ? (
            <Tabs value={activeTab} onValueChange={(v: string) => setActiveTab(v as FilterTab)}>
              <TabsList>
                <TabsTrigger value="all">All ({statusCounts.all})</TabsTrigger>
                <TabsTrigger value="healthy">Healthy ({statusCounts.healthy})</TabsTrigger>
                <TabsTrigger value="issues">Issues ({statusCounts.issues})</TabsTrigger>
                <TabsTrigger value="building">Building ({statusCounts.building})</TabsTrigger>
              </TabsList>
            </Tabs>
          ) : (
            <div />
          )}
          <div className="flex items-center gap-2">
            <div className="relative flex-1 sm:flex-none">
              <Search
                className="pointer-events-none absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground"
                aria-hidden="true"
              />
              <Input
                aria-label="Search projects"
                placeholder="Search projects"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="pl-8 sm:w-60"
              />
            </div>
            <Select value={sort} onValueChange={(v) => setSort(v as SortKey)}>
              <SelectTrigger aria-label="Sort projects" className="w-40">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="attention">Needs attention</SelectItem>
                <SelectItem value="name">Name</SelectItem>
                <SelectItem value="newest">Newest</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>
      )}

      {/* Projects Grid */}
      {projectsError && projects.length === 0 ? null : filteredProjects.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center">
            {projects.length === 0 ? (
              <>
                <p className="text-muted-foreground mb-4">
                  No projects yet. Create your first project to organize your apps.
                </p>
                <Button onClick={() => setIsCreateDialogOpen(true)}>
                  <Plus className="mr-2 h-4 w-4" />
                  Create Project
                </Button>
              </>
            ) : (
              <p className="text-muted-foreground">
                No projects match your search or filter.
              </p>
            )}
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {filteredProjects.map((project) => (
            <ProjectCard
              key={project.id}
              project={project}
              health={getProjectHealth(project, appStatuses, databaseStatuses)}
              appStatuses={appStatuses}
              databaseStatuses={databaseStatuses}
            />
          ))}
        </div>
      )}
    </div>
  );
}
