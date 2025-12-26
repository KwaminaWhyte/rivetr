import { useState, useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Plus } from "lucide-react";
import { toast } from "sonner";
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
import { ProjectCard } from "@/components/ProjectCard";
import { api } from "@/lib/api";
import type { App, CreateProjectRequest, DeploymentStatus, ProjectWithApps } from "@/types/api";

type FilterTab = "all" | "healthy" | "issues" | "building";

function getProjectHealth(project: ProjectWithApps, appStatuses: Record<string, DeploymentStatus>): FilterTab {
  if (project.apps.length === 0) return "healthy";

  const statuses = project.apps.map((app) => appStatuses[app.id]);

  if (statuses.some((s) => s === "building" || s === "cloning" || s === "starting" || s === "checking" || s === "pending")) {
    return "building";
  }
  if (statuses.some((s) => s === "failed" || s === "stopped")) {
    return "issues";
  }
  return "healthy";
}

export function ProjectsPage() {
  const queryClient = useQueryClient();
  const [activeTab, setActiveTab] = useState<FilterTab>("all");
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false);
  const [newProject, setNewProject] = useState<CreateProjectRequest>({
    name: "",
    description: "",
  });

  // Fetch all projects
  const {
    data: projects = [],
    isLoading: projectsLoading,
    error: projectsError,
  } = useQuery<ProjectWithApps[]>({
    queryKey: ["projects"],
    queryFn: async () => {
      // Get all projects, then fetch each with apps
      const projectList = await api.getProjects();
      const projectsWithApps = await Promise.all(
        projectList.map((p) => api.getProject(p.id))
      );
      return projectsWithApps;
    },
  });

  // Fetch all apps to get deployment statuses
  const { data: apps = [] } = useQuery<App[]>({
    queryKey: ["apps"],
    queryFn: () => api.getApps(),
  });

  // Build app status map (for now use a simple heuristic based on domain)
  // In a real app, we would fetch deployment statuses
  const appStatuses = useMemo(() => {
    const statusMap: Record<string, DeploymentStatus> = {};
    for (const app of apps) {
      // Simple heuristic: if has domain, assume running
      statusMap[app.id] = app.domain ? "running" : "stopped";
    }
    return statusMap;
  }, [apps]);

  // Filter projects by tab
  const filteredProjects = useMemo(() => {
    if (activeTab === "all") return projects;

    return projects.filter((project) => {
      const health = getProjectHealth(project, appStatuses);
      return health === activeTab;
    });
  }, [projects, activeTab, appStatuses]);

  // Create project mutation
  const createMutation = useMutation({
    mutationFn: (data: CreateProjectRequest) => api.createProject(data),
    onSuccess: (project) => {
      toast.success(`Project "${project.name}" created`);
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      setIsCreateDialogOpen(false);
      setNewProject({ name: "", description: "" });
    },
    onError: (err: Error) => {
      toast.error(`Failed to create project: ${err.message}`);
    },
  });

  const handleCreateProject = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newProject.name.trim()) {
      toast.error("Project name is required");
      return;
    }
    createMutation.mutate(newProject);
  };

  // Count projects by status
  const statusCounts = useMemo(() => {
    const counts = { all: projects.length, healthy: 0, issues: 0, building: 0 };
    for (const project of projects) {
      const health = getProjectHealth(project, appStatuses);
      counts[health]++;
    }
    return counts;
  }, [projects, appStatuses]);

  if (projectsError) {
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
        <Card>
          <CardContent className="py-8 text-center text-destructive">
            Failed to load projects. Please try again.
          </CardContent>
        </Card>
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

        <Dialog open={isCreateDialogOpen} onOpenChange={setIsCreateDialogOpen}>
          <DialogTrigger asChild>
            <Button>
              <Plus className="mr-2 h-4 w-4" />
              New Project
            </Button>
          </DialogTrigger>
          <DialogContent>
            <form onSubmit={handleCreateProject}>
              <DialogHeader>
                <DialogTitle>Create New Project</DialogTitle>
                <DialogDescription>
                  Projects help you organize related applications together.
                </DialogDescription>
              </DialogHeader>
              <div className="space-y-4 py-4">
                <div className="space-y-2">
                  <Label htmlFor="project-name">Name</Label>
                  <Input
                    id="project-name"
                    placeholder="my-project"
                    value={newProject.name}
                    onChange={(e) =>
                      setNewProject((prev) => ({ ...prev, name: e.target.value }))
                    }
                    required
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="project-description">Description</Label>
                  <Textarea
                    id="project-description"
                    placeholder="A brief description of your project..."
                    value={newProject.description || ""}
                    onChange={(e) =>
                      setNewProject((prev) => ({
                        ...prev,
                        description: e.target.value,
                      }))
                    }
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
                <Button type="submit" disabled={createMutation.isPending}>
                  {createMutation.isPending ? "Creating..." : "Create Project"}
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
      {projectsLoading ? (
        <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
          {[1, 2, 3].map((i) => (
            <Card key={i}>
              <CardContent className="p-6">
                <Skeleton className="h-6 w-1/2 mb-2" />
                <Skeleton className="h-4 w-3/4 mb-4" />
                <Skeleton className="h-16 w-full" />
              </CardContent>
            </Card>
          ))}
        </div>
      ) : filteredProjects.length === 0 ? (
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
            />
          ))}
        </div>
      )}
    </div>
  );
}
