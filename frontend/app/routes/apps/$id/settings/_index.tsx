import { useState } from "react";
import { useOutletContext } from "react-router";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
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
import { GitHubSourceCard } from "@/components/github-source-card";
import { api } from "@/lib/api";
import type { App, AppEnvironment, UpdateAppRequest } from "@/types/api";

const ENVIRONMENT_OPTIONS: { value: AppEnvironment; label: string }[] = [
  { value: "development", label: "Development" },
  { value: "staging", label: "Staging" },
  { value: "production", label: "Production" },
];

export default function AppSettingsGeneral() {
  const { app } = useOutletContext<{ app: App }>();
  const queryClient = useQueryClient();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [generalForm, setGeneralForm] = useState({
    name: app.name,
    git_url: app.git_url,
    branch: app.branch,
    port: app.port,
    environment: app.environment || "development",
    healthcheck: app.healthcheck || "",
  });

  const handleGeneralSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    try {
      const updates: UpdateAppRequest = {
        name: generalForm.name,
        git_url: generalForm.git_url,
        branch: generalForm.branch,
        port: generalForm.port,
        environment: generalForm.environment as AppEnvironment,
        healthcheck: generalForm.healthcheck,
      };
      await api.updateApp(app.id, updates);
      toast.success("Settings saved");
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Update failed");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="space-y-6">
      <GitHubSourceCard app={app} />
      <Card>
        <CardHeader>
          <CardTitle>General Settings</CardTitle>
          <CardDescription>
            Basic application configuration. Changes will take effect on the next deployment.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleGeneralSubmit} className="space-y-6">
            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="name">Name</Label>
                <Input
                  id="name"
                  value={generalForm.name}
                  onChange={(e) => setGeneralForm({ ...generalForm, name: e.target.value })}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="git_url">Git URL</Label>
                <Input
                  id="git_url"
                  value={generalForm.git_url}
                  onChange={(e) => setGeneralForm({ ...generalForm, git_url: e.target.value })}
                />
              </div>
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="branch">Branch</Label>
                <Input
                  id="branch"
                  value={generalForm.branch}
                  onChange={(e) => setGeneralForm({ ...generalForm, branch: e.target.value })}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="port">Port</Label>
                <Input
                  id="port"
                  type="number"
                  value={generalForm.port}
                  onChange={(e) => setGeneralForm({ ...generalForm, port: parseInt(e.target.value) || 0 })}
                />
              </div>
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="environment">Environment</Label>
                <Select
                  value={generalForm.environment}
                  onValueChange={(value) => setGeneralForm({ ...generalForm, environment: value as AppEnvironment })}
                >
                  <SelectTrigger>
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
              </div>
              <div className="space-y-2">
                <Label htmlFor="healthcheck">Healthcheck Path</Label>
                <Input
                  id="healthcheck"
                  placeholder="/health"
                  value={generalForm.healthcheck}
                  onChange={(e) => setGeneralForm({ ...generalForm, healthcheck: e.target.value })}
                />
                <p className="text-xs text-muted-foreground">
                  Endpoint to check if the app is running
                </p>
              </div>
            </div>

            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? "Saving..." : "Save Changes"}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
