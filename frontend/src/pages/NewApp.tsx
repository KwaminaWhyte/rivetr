import { useState } from "react";
import { useNavigate } from "react-router";
import { useMutation, useQuery } from "@tanstack/react-query";
import { toast } from "sonner";
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
import { api } from "@/lib/api";
import type { AppEnvironment, CreateAppRequest, Project } from "@/types/api";
import { CPU_OPTIONS, MEMORY_OPTIONS } from "@/components/ResourceLimitsCard";

const ENVIRONMENT_OPTIONS: { value: AppEnvironment; label: string }[] = [
  { value: "development", label: "Development" },
  { value: "staging", label: "Staging" },
  { value: "production", label: "Production" },
];

export function NewAppPage() {
  const navigate = useNavigate();
  const [formData, setFormData] = useState<CreateAppRequest>({
    name: "",
    git_url: "",
    branch: "main",
    dockerfile: "Dockerfile",
    port: 3000,
    cpu_limit: "1",
    memory_limit: "512m",
    environment: "development",
    project_id: undefined,
  });
  const [error, setError] = useState("");

  // Fetch projects for the selector
  const { data: projects = [] } = useQuery<Project[]>({
    queryKey: ["projects"],
    queryFn: () => api.getProjects(),
  });

  const createMutation = useMutation({
    mutationFn: (data: CreateAppRequest) => api.createApp(data),
    onSuccess: (app) => {
      toast.success(`Application "${app.name}" created`);
      navigate(`/apps/${app.id}`);
    },
    onError: (err: Error) => {
      toast.error(`Failed to create app: ${err.message}`);
      setError(err.message);
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setError("");

    if (!formData.name.trim()) {
      setError("Name is required");
      return;
    }
    if (!formData.git_url.trim()) {
      setError("Git URL is required");
      return;
    }

    createMutation.mutate(formData);
  };

  const handleChange = (field: keyof CreateAppRequest, value: string | number) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
  };

  return (
    <div className="space-y-6">
      <h1 className="text-3xl font-bold">New Application</h1>

      <Card className="max-w-2xl">
        <CardHeader>
          <CardTitle>Application Details</CardTitle>
        </CardHeader>
        <CardContent>
          {error && (
            <div className="mb-4 p-3 rounded-md bg-destructive/10 text-destructive text-sm">
              {error}
            </div>
          )}

          <form onSubmit={handleSubmit} className="space-y-6">
            <div className="space-y-2">
              <Label htmlFor="name">Name *</Label>
              <Input
                id="name"
                placeholder="my-app"
                value={formData.name}
                onChange={(e) => handleChange("name", e.target.value)}
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
                placeholder="https://github.com/user/repo.git"
                value={formData.git_url}
                onChange={(e) => handleChange("git_url", e.target.value)}
                required
              />
              <p className="text-xs text-muted-foreground">
                The Git repository URL to clone
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="environment">Environment</Label>
              <Select
                value={formData.environment || "development"}
                onValueChange={(value) => handleChange("environment", value)}
              >
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
              <Label htmlFor="project">Project (Optional)</Label>
              <Select
                value={formData.project_id || "none"}
                onValueChange={(value) =>
                  handleChange("project_id", value === "none" ? "" : value)
                }
              >
                <SelectTrigger className="w-full">
                  <SelectValue placeholder="Select a project" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">No Project</SelectItem>
                  {projects.map((project) => (
                    <SelectItem key={project.id} value={project.id}>
                      {project.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                Optionally assign this app to a project for organization
              </p>
            </div>

            <div className="grid gap-6 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="branch">Branch</Label>
                <Input
                  id="branch"
                  placeholder="main"
                  value={formData.branch}
                  onChange={(e) => handleChange("branch", e.target.value)}
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="port">Port</Label>
                <Input
                  id="port"
                  type="number"
                  placeholder="3000"
                  value={formData.port}
                  onChange={(e) => handleChange("port", parseInt(e.target.value) || 3000)}
                />
              </div>
            </div>

            <div className="grid gap-6 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="dockerfile">Dockerfile</Label>
                <Input
                  id="dockerfile"
                  placeholder="Dockerfile"
                  value={formData.dockerfile}
                  onChange={(e) => handleChange("dockerfile", e.target.value)}
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="domain">Domain</Label>
                <Input
                  id="domain"
                  placeholder="app.example.com"
                  value={formData.domain || ""}
                  onChange={(e) => handleChange("domain", e.target.value)}
                />
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="healthcheck">Healthcheck Path</Label>
              <Input
                id="healthcheck"
                placeholder="/health"
                value={formData.healthcheck || ""}
                onChange={(e) => handleChange("healthcheck", e.target.value)}
              />
              <p className="text-xs text-muted-foreground">
                Optional endpoint to check if the app is healthy
              </p>
            </div>

            <div className="grid gap-6 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="cpu_limit">CPU Limit</Label>
                <Select
                  value={formData.cpu_limit || "1"}
                  onValueChange={(value) => handleChange("cpu_limit", value)}
                >
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
                <Select
                  value={formData.memory_limit || "512m"}
                  onValueChange={(value) => handleChange("memory_limit", value)}
                >
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
              <Button
                type="submit"
                disabled={createMutation.isPending}
              >
                {createMutation.isPending ? "Creating..." : "Create Application"}
              </Button>
              <Button
                type="button"
                variant="outline"
                onClick={() => navigate("/apps")}
              >
                Cancel
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
