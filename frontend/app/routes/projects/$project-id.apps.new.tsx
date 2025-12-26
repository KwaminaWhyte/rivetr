import { Form, Link, redirect, useNavigation } from "react-router";
import { useQuery } from "@tanstack/react-query";
import type { Route } from "./+types/$project-id.apps.new";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { CPU_OPTIONS, MEMORY_OPTIONS } from "@/components/resource-limits-card";
import { api } from "@/lib/api";
import type { AppEnvironment, Project } from "@/types/api";

const ENVIRONMENT_OPTIONS: { value: AppEnvironment; label: string }[] = [
  { value: "development", label: "Development" },
  { value: "staging", label: "Staging" },
  { value: "production", label: "Production" },
];

export async function loader({ request, params }: Route.LoaderArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const project = await api.getProject(token, params.projectId!);
  const projects = await api.getProjects(token).catch(() => []);
  return { project, projects };
}

export async function action({ request, params }: Route.ActionArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const formData = await request.formData();

  const name = formData.get("name");
  const git_url = formData.get("git_url");
  const branch = formData.get("branch") || "main";
  const dockerfile = formData.get("dockerfile") || "Dockerfile";
  const port = parseInt(formData.get("port") as string) || 3000;
  const domain = formData.get("domain") || undefined;
  const healthcheck = formData.get("healthcheck") || undefined;
  const cpu_limit = formData.get("cpu_limit") || "1";
  const memory_limit = formData.get("memory_limit") || "512m";
  const environment = (formData.get("environment") || "development") as AppEnvironment;

  if (typeof name !== "string" || !name.trim()) {
    return { error: "Name is required" };
  }
  if (typeof git_url !== "string" || !git_url.trim()) {
    return { error: "Git URL is required" };
  }

  try {
    const app = await api.createApp(token, {
      name: name.trim(),
      git_url: git_url.trim(),
      branch: branch as string,
      dockerfile: dockerfile as string,
      port,
      domain: domain as string | undefined,
      healthcheck: healthcheck as string | undefined,
      cpu_limit: cpu_limit as string,
      memory_limit: memory_limit as string,
      environment,
      project_id: params.projectId,
    });
    return redirect(`/apps/${app.id}`);
  } catch (error) {
    return { error: error instanceof Error ? error.message : "Failed to create app" };
  }
}

export default function NewAppPage({ loaderData, actionData, params }: Route.ComponentProps) {
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";

  // Use React Query with SSR initial data
  const { data: projects = [] } = useQuery<Project[]>({
    queryKey: ["projects"],
    queryFn: () => api.getProjects(),
    initialData: loaderData.projects,
  });

  const projectId = params.projectId;

  return (
    <div className="space-y-6">
      <h1 className="text-3xl font-bold">New Application</h1>

      <Card className="max-w-2xl">
        <CardHeader>
          <CardTitle>Application Details</CardTitle>
        </CardHeader>
        <CardContent>
          {actionData?.error && (
            <div className="mb-4 p-3 rounded-md bg-destructive/10 text-destructive text-sm">
              {actionData.error}
            </div>
          )}

          <Form method="post" className="space-y-6">
            <div className="space-y-2">
              <Label htmlFor="name">Name *</Label>
              <Input
                id="name"
                name="name"
                placeholder="my-app"
                required
              />
              <p className="text-xs text-muted-foreground">
                A unique name for your application
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="git_url">Git Repository URL *</Label>
              <Input
                id="git_url"
                name="git_url"
                placeholder="https://github.com/user/repo.git"
                required
              />
              <p className="text-xs text-muted-foreground">
                The Git repository URL to clone
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="environment">Environment</Label>
              <Select name="environment" defaultValue="development">
                <SelectTrigger className="w-full">
                  <SelectValue placeholder="Select environment" />
                </SelectTrigger>
                <SelectContent>
                  {ENVIRONMENT_OPTIONS.map((option) => (
                    <SelectItem key={option.value} value={option.value}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                The deployment environment for this application
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="project">Project</Label>
              <Select name="project_id" defaultValue={projectId} disabled>
                <SelectTrigger className="w-full">
                  <SelectValue placeholder="Select a project" />
                </SelectTrigger>
                <SelectContent>
                  {projects.map((project) => (
                    <SelectItem key={project.id} value={project.id}>
                      {project.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                This app will be added to the selected project
              </p>
            </div>

            <div className="grid gap-6 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="branch">Branch</Label>
                <Input
                  id="branch"
                  name="branch"
                  placeholder="main"
                  defaultValue="main"
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="port">Port</Label>
                <Input
                  id="port"
                  name="port"
                  type="number"
                  placeholder="3000"
                  defaultValue="3000"
                />
              </div>
            </div>

            <div className="grid gap-6 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="dockerfile">Dockerfile</Label>
                <Input
                  id="dockerfile"
                  name="dockerfile"
                  placeholder="Dockerfile"
                  defaultValue="Dockerfile"
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="domain">Domain</Label>
                <Input
                  id="domain"
                  name="domain"
                  placeholder="app.example.com"
                />
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="healthcheck">Healthcheck Path</Label>
              <Input
                id="healthcheck"
                name="healthcheck"
                placeholder="/health"
              />
              <p className="text-xs text-muted-foreground">
                Optional endpoint to check if the app is healthy
              </p>
            </div>

            <div className="grid gap-6 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="cpu_limit">CPU Limit</Label>
                <Select name="cpu_limit" defaultValue="1">
                  <SelectTrigger className="w-full">
                    <SelectValue placeholder="Select CPU limit" />
                  </SelectTrigger>
                  <SelectContent>
                    {CPU_OPTIONS.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {option.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  Maximum CPU cores for this container
                </p>
              </div>

              <div className="space-y-2">
                <Label htmlFor="memory_limit">Memory Limit</Label>
                <Select name="memory_limit" defaultValue="512m">
                  <SelectTrigger className="w-full">
                    <SelectValue placeholder="Select memory limit" />
                  </SelectTrigger>
                  <SelectContent>
                    {MEMORY_OPTIONS.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {option.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  Maximum memory for this container
                </p>
              </div>
            </div>

            <div className="flex gap-4">
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? "Creating..." : "Create Application"}
              </Button>
              <Button type="button" variant="outline" asChild>
                <Link to={`/projects/${projectId}`}>Cancel</Link>
              </Button>
            </div>
          </Form>
        </CardContent>
      </Card>
    </div>
  );
}
