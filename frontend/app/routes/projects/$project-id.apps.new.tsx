import { useState } from "react";
import { Form, Link, redirect, useNavigation } from "react-router";
import { useQuery } from "@tanstack/react-query";
import type { Route } from "./+types/$project-id.apps.new";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Eye, EyeOff, GitBranch, Package } from "lucide-react";
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
  const deployment_source = formData.get("deployment_source") || "git";
  const port = parseInt(formData.get("port") as string) || 3000;
  const domain = formData.get("domain") || undefined;
  const healthcheck = formData.get("healthcheck") || undefined;
  const cpu_limit = formData.get("cpu_limit") || "1";
  const memory_limit = formData.get("memory_limit") || "512m";
  const environment = (formData.get("environment") || "development") as AppEnvironment;

  if (typeof name !== "string" || !name.trim()) {
    return { error: "Name is required" };
  }

  try {
    if (deployment_source === "registry") {
      // Registry-based deployment
      const docker_image = formData.get("docker_image");
      const docker_image_tag = formData.get("docker_image_tag") || "latest";
      const registry_url = formData.get("registry_url") || undefined;
      const registry_username = formData.get("registry_username") || undefined;
      const registry_password = formData.get("registry_password") || undefined;

      if (typeof docker_image !== "string" || !docker_image.trim()) {
        return { error: "Docker image is required" };
      }

      const app = await api.createApp(token, {
        name: name.trim(),
        docker_image: docker_image.trim(),
        docker_image_tag: docker_image_tag as string,
        registry_url: registry_url as string | undefined,
        registry_username: registry_username as string | undefined,
        registry_password: registry_password as string | undefined,
        port,
        domain: domain as string | undefined,
        healthcheck: healthcheck as string | undefined,
        cpu_limit: cpu_limit as string,
        memory_limit: memory_limit as string,
        environment,
        project_id: params.projectId,
      });
      return redirect(`/apps/${app.id}`);
    } else {
      // Git-based deployment
      const git_url = formData.get("git_url");
      const branch = formData.get("branch") || "main";
      const dockerfile = formData.get("dockerfile") || "Dockerfile";

      if (typeof git_url !== "string" || !git_url.trim()) {
        return { error: "Git URL is required" };
      }

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
    }
  } catch (error) {
    return { error: error instanceof Error ? error.message : "Failed to create app" };
  }
}

export default function NewAppPage({ loaderData, actionData, params }: Route.ComponentProps) {
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";
  const [deploymentSource, setDeploymentSource] = useState<"git" | "registry">("git");
  const [showPassword, setShowPassword] = useState(false);

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
          <CardDescription>
            Create a new application by building from Git or deploying a pre-built Docker image.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {actionData?.error && (
            <div className="mb-4 p-3 rounded-md bg-destructive/10 text-destructive text-sm">
              {actionData.error}
            </div>
          )}

          <Form method="post" className="space-y-6">
            <input type="hidden" name="deployment_source" value={deploymentSource} />

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

            {/* Deployment Source Tabs */}
            <div className="space-y-4">
              <Label>Deployment Source</Label>
              <Tabs value={deploymentSource} onValueChange={(v) => setDeploymentSource(v as "git" | "registry")}>
                <TabsList className="grid w-full grid-cols-2">
                  <TabsTrigger value="git" className="flex items-center gap-2">
                    <GitBranch className="h-4 w-4" />
                    Build from Git
                  </TabsTrigger>
                  <TabsTrigger value="registry" className="flex items-center gap-2">
                    <Package className="h-4 w-4" />
                    Docker Registry
                  </TabsTrigger>
                </TabsList>

                {/* Git Source Tab */}
                <TabsContent value="git" className="space-y-4 pt-4">
                  <div className="space-y-2">
                    <Label htmlFor="git_url">Git Repository URL *</Label>
                    <Input
                      id="git_url"
                      name="git_url"
                      placeholder="https://github.com/user/repo.git"
                      required={deploymentSource === "git"}
                    />
                    <p className="text-xs text-muted-foreground">
                      The Git repository URL to clone
                    </p>
                  </div>

                  <div className="grid gap-4 md:grid-cols-2">
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
                      <Label htmlFor="dockerfile">Dockerfile</Label>
                      <Input
                        id="dockerfile"
                        name="dockerfile"
                        placeholder="Dockerfile"
                        defaultValue="Dockerfile"
                      />
                    </div>
                  </div>
                </TabsContent>

                {/* Registry Source Tab */}
                <TabsContent value="registry" className="space-y-4 pt-4">
                  <div className="space-y-2">
                    <Label htmlFor="docker_image">Docker Image *</Label>
                    <Input
                      id="docker_image"
                      name="docker_image"
                      placeholder="nginx, ghcr.io/user/app, registry.example.com/image"
                      required={deploymentSource === "registry"}
                    />
                    <p className="text-xs text-muted-foreground">
                      Image name with optional registry prefix
                    </p>
                  </div>

                  <div className="grid gap-4 md:grid-cols-2">
                    <div className="space-y-2">
                      <Label htmlFor="docker_image_tag">Image Tag</Label>
                      <Input
                        id="docker_image_tag"
                        name="docker_image_tag"
                        placeholder="latest"
                        defaultValue="latest"
                      />
                      <p className="text-xs text-muted-foreground">
                        Tag or version (e.g., latest, v1.0.0)
                      </p>
                    </div>

                    <div className="space-y-2">
                      <Label htmlFor="registry_url">Registry URL</Label>
                      <Input
                        id="registry_url"
                        name="registry_url"
                        placeholder="Leave empty for Docker Hub"
                      />
                      <p className="text-xs text-muted-foreground">
                        Custom registry URL (optional)
                      </p>
                    </div>
                  </div>

                  <div className="space-y-2">
                    <p className="text-sm font-medium">Registry Authentication (optional)</p>
                    <div className="grid gap-4 md:grid-cols-2">
                      <div className="space-y-2">
                        <Label htmlFor="registry_username">Username</Label>
                        <Input
                          id="registry_username"
                          name="registry_username"
                          placeholder="Username or token name"
                          autoComplete="off"
                        />
                      </div>

                      <div className="space-y-2">
                        <Label htmlFor="registry_password">Password / Access Token</Label>
                        <div className="relative">
                          <Input
                            id="registry_password"
                            name="registry_password"
                            type={showPassword ? "text" : "password"}
                            placeholder="Password or access token"
                            autoComplete="new-password"
                          />
                          <Button
                            type="button"
                            variant="ghost"
                            size="sm"
                            className="absolute right-0 top-0 h-full px-3 py-2 hover:bg-transparent"
                            onClick={() => setShowPassword(!showPassword)}
                          >
                            {showPassword ? (
                              <EyeOff className="h-4 w-4 text-muted-foreground" />
                            ) : (
                              <Eye className="h-4 w-4 text-muted-foreground" />
                            )}
                          </Button>
                        </div>
                      </div>
                    </div>
                    <p className="text-xs text-muted-foreground">
                      Required for private registries. For GitHub Container Registry, use a personal access token.
                    </p>
                  </div>
                </TabsContent>
              </Tabs>
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
                <Label htmlFor="port">Port</Label>
                <Input
                  id="port"
                  name="port"
                  type="number"
                  placeholder="3000"
                  defaultValue="3000"
                />
                <p className="text-xs text-muted-foreground">
                  Container port to expose
                </p>
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
