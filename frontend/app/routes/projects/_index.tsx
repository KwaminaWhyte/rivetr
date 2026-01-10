import { useState, useMemo } from "react";
import { useQuery, useQueryClient, useMutation } from "@tanstack/react-query";
import { Plus } from "lucide-react";
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
import { ProjectCard } from "@/components/project-card";
import type { App, DeploymentStatus, ManagedDatabase, ProjectWithApps } from "@/types/api";

export function meta() {
  return [
    { title: "Projects - Rivetr" },
    { name: "description", content: "Manage your projects and applications" },
  ];
}

type FilterTab = "all" | "healthy" | "issues" | "building";

function getProjectHealth(
  project: ProjectWithApps,
  appStatuses: Record<string, DeploymentStatus>,
  databaseStatuses: Record<string, string>
): FilterTab {
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
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [error, setError] = useState<string | null>(null);

  // Use React Query for data fetching
  const { data: projects = [], isLoading: projectsLoading } = useQuery<ProjectWithApps[]>({
    queryKey: ["projects-with-apps", currentTeamId],
    queryFn: async () => {
      const projectList = await api.getProjects(currentTeamId ?? undefined);
      const projectsWithApps = await Promise.all(
        projectList.map((p) => api.getProject(p.id).catch(() => ({ ...p, apps: [], databases: [], services: [] } as ProjectWithApps)))
      );
      return projectsWithApps;
    },
    enabled: currentTeamId !== null,
  });

  const { data: apps = [] } = useQuery<App[]>({
    queryKey: ["apps", currentTeamId],
    queryFn: () => api.getApps({ teamId: currentTeamId ?? undefined }),
    enabled: currentTeamId !== null,
  });

  const { data: databases = [] } = useQuery<ManagedDatabase[]>({
    queryKey: ["databases", currentTeamId],
    queryFn: () => api.getDatabases(),
    enabled: currentTeamId !== null,
  });

  // Fetch app statuses
  const { data: appStatuses = {} } = useQuery<Record<string, DeploymentStatus>>({
    queryKey: ["app-statuses", apps.map((a) => a.id)],
    queryFn: async () => {
      const statuses: Record<string, DeploymentStatus> = {};
      await Promise.all(
        apps.map(async (app) => {
          try {
            const status = await api.getAppStatus(app.id);
            if (status.status === "running") {
              statuses[app.id] = "running";
            } else if (status.status === "stopped") {
              statuses[app.id] = "stopped";
            } else {
              const deploymentsData = await api.getDeployments(app.id, { per_page: 1 }).catch(() => null);
              if (deploymentsData && deploymentsData.items.length > 0) {
                statuses[app.id] = deploymentsData.items[0].status as DeploymentStatus;
              } else {
                statuses[app.id] = "stopped";
              }
            }
          } catch {
            statuses[app.id] = "stopped";
          }
        })
      );
      return statuses;
    },
    enabled: apps.length > 0,
  });

  // Database statuses from the database objects
  const databaseStatuses = useMemo(() => {
    const statuses: Record<string, string> = {};
    for (const db of databases) {
      statuses[db.id] = db.status;
    }
    return statuses;
  }, [databases]);

  // Filter projects by tab
  const filteredProjects = useMemo(() => {
    if (activeTab === "all") return projects;
    return projects.filter((project) => {
      const health = getProjectHealth(project, appStatuses, databaseStatuses);
      return health === activeTab;
    });
  }, [projects, activeTab, appStatuses, databaseStatuses]);

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
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold">Projects</h1>
            <p className="text-muted-foreground">
              Manage your applications and service groups
            </p>
          </div>
        </div>
        <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
          {[1, 2, 3].map((i) => (
            <Card key={i}>
              <CardContent className="p-6">
                <Skeleton className="h-6 w-32 mb-4" />
                <Skeleton className="h-4 w-full mb-2" />
                <Skeleton className="h-4 w-2/3" />
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
          <h1 className="text-3xl font-bold">Projects</h1>
          <p className="text-muted-foreground">
            Manage your applications and service groups
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

      {/* Filter Tabs */}
      <Tabs value={activeTab} onValueChange={(v: string) => setActiveTab(v as FilterTab)}>
        <TabsList>
          <TabsTrigger value="all">All ({statusCounts.all})</TabsTrigger>
          <TabsTrigger value="healthy">Healthy ({statusCounts.healthy})</TabsTrigger>
          <TabsTrigger value="issues">Issues ({statusCounts.issues})</TabsTrigger>
          <TabsTrigger value="building">Building ({statusCounts.building})</TabsTrigger>
        </TabsList>
      </Tabs>

      {/* Projects Grid */}
      {filteredProjects.length === 0 ? (
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
                No projects match the selected filter.
              </p>
            )}
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
          {filteredProjects.map((project) => (
            <ProjectCard
              key={project.id}
              project={project}
              appStatuses={appStatuses}
              databaseStatuses={databaseStatuses}
            />
          ))}
        </div>
      )}
    </div>
  );
}
