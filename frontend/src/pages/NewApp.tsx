import { useState } from "react";
import { useNavigate } from "react-router";
import { useMutation } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { api } from "@/lib/api";
import type { CreateAppRequest } from "@/types/api";

export function NewAppPage() {
  const navigate = useNavigate();
  const [formData, setFormData] = useState<CreateAppRequest>({
    name: "",
    git_url: "",
    branch: "main",
    dockerfile: "Dockerfile",
    port: 3000,
  });
  const [error, setError] = useState("");

  const createMutation = useMutation({
    mutationFn: (data: CreateAppRequest) => api.createApp(data),
    onSuccess: (app) => {
      navigate(`/apps/${app.id}`);
    },
    onError: (err: Error) => {
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
