import { useState, useMemo } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Form, useNavigation } from "react-router";
import type { Route } from "./+types/_index";
import { Plus } from "lucide-react";
import { toast } from "sonner";
import { api } from "@/lib/api";
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
import type { App, CreateProjectRequest, DeploymentStatus, ProjectWithApps } from "@/types/api";

export async function loader({ request }: Route.LoaderArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const [projectList, apps] = await Promise.all([
    api.getProjects(token).catch(() => []),
    api.getApps(token).catch(() => []),
  ]);

  // Get full project details with apps
  const projectsWithApps = await Promise.all(
    projectList.map((p) => api.getProject(token, p.id).catch(() => ({ ...p, apps: [] })))
  );

  return { projects: projectsWithApps, apps };
}

export async function action({ request }: Route.ActionArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const formData = await request.formData();
  const name = formData.get("name");
  const description = formData.get("description");

  if (typeof name !== "string" || !name.trim()) {
    return { error: "Project name is required" };
  }

  try {
    const project = await api.createProject(token, {
      name: name.trim(),
      description: typeof description === "string" ? description : undefined,
    });
    return { success: true, project };
  } catch (error) {
    return { error: error instanceof Error ? error.message : "Failed to create project" };
  }
}

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

export default function ProjectsPage({ loaderData, actionData }: Route.ComponentProps) {
  const queryClient = useQueryClient();
  const navigation = useNavigation();
  const [activeTab, setActiveTab] = useState<FilterTab>("all");
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false);

  // Use React Query with SSR initial data for real-time updates
  const { data: projects = [] } = useQuery<ProjectWithApps[]>({
    queryKey: ["projects"],
    queryFn: async () => {
      const projectList = await api.getProjects();
      const projectsWithApps = await Promise.all(
        projectList.map((p) => api.getProject(p.id).catch(() => ({ ...p, apps: [] })))
      );
      return projectsWithApps;
    },
    initialData: loaderData.projects,
  });

  const { data: apps = [] } = useQuery<App[]>({
    queryKey: ["apps"],
    queryFn: () => api.getApps(),
    initialData: loaderData.apps,
  });

  // Build app status map
  const appStatuses = useMemo(() => {
    const statusMap: Record<string, DeploymentStatus> = {};
    for (const app of apps) {
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

  // Count projects by status
  const statusCounts = useMemo(() => {
    const counts = { all: projects.length, healthy: 0, issues: 0, building: 0 };
    for (const project of projects) {
      const health = getProjectHealth(project, appStatuses);
      counts[health]++;
    }
    return counts;
  }, [projects, appStatuses]);

  // Close dialog on successful creation
  const isSubmitting = navigation.state === "submitting";

  // Effect to close dialog and invalidate on success
  if (actionData?.success && isCreateDialogOpen) {
    setIsCreateDialogOpen(false);
    queryClient.invalidateQueries({ queryKey: ["projects"] });
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
            <Form method="post">
              <DialogHeader>
                <DialogTitle>Create New Project</DialogTitle>
                <DialogDescription>
                  Projects help you organize related applications together.
                </DialogDescription>
              </DialogHeader>
              {actionData?.error && (
                <div className="p-3 rounded-md bg-destructive/10 text-destructive text-sm">
                  {actionData.error}
                </div>
              )}
              <div className="space-y-4 py-4">
                <div className="space-y-2">
                  <Label htmlFor="project-name">Name</Label>
                  <Input
                    id="project-name"
                    name="name"
                    placeholder="my-project"
                    required
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="project-description">Description</Label>
                  <Textarea
                    id="project-description"
                    name="description"
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
                <Button type="submit" disabled={isSubmitting}>
                  {isSubmitting ? "Creating..." : "Create Project"}
                </Button>
              </DialogFooter>
            </Form>
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
            />
          ))}
        </div>
      )}
    </div>
  );
}
