import { useState, useEffect } from "react";
import { Form, useNavigation, useOutletContext } from "react-router";
import { useQueryClient } from "@tanstack/react-query";
import type { Route } from "./+types/settings";
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
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { EnvVarsTab } from "@/components/env-vars-tab";
import { CPU_OPTIONS, MEMORY_OPTIONS } from "@/components/resource-limits-card";
import type { App, AppEnvironment, UpdateAppRequest } from "@/types/api";

const ENVIRONMENT_OPTIONS: { value: AppEnvironment; label: string }[] = [
  { value: "development", label: "Development" },
  { value: "staging", label: "Staging" },
  { value: "production", label: "Production" },
];

interface OutletContext {
  app: App;
  token: string;
}

export async function action({ request, params }: Route.ActionArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  if (intent === "update") {
    const updates: UpdateAppRequest = {};
    const name = formData.get("name");
    const git_url = formData.get("git_url");
    const branch = formData.get("branch");
    const dockerfile = formData.get("dockerfile");
    const domain = formData.get("domain");
    const port = formData.get("port");
    const healthcheck = formData.get("healthcheck");
    const environment = formData.get("environment");
    const cpu_limit = formData.get("cpu_limit");
    const memory_limit = formData.get("memory_limit");

    if (typeof name === "string") updates.name = name;
    if (typeof git_url === "string") updates.git_url = git_url;
    if (typeof branch === "string") updates.branch = branch;
    if (typeof dockerfile === "string") updates.dockerfile = dockerfile;
    if (typeof domain === "string") updates.domain = domain || undefined;
    if (typeof port === "string") updates.port = parseInt(port) || undefined;
    if (typeof healthcheck === "string") updates.healthcheck = healthcheck || undefined;
    if (typeof environment === "string") updates.environment = environment as AppEnvironment;
    if (typeof cpu_limit === "string") updates.cpu_limit = cpu_limit;
    if (typeof memory_limit === "string") updates.memory_limit = memory_limit;

    try {
      await api.updateApp(token, params.id!, updates);
      return { success: true };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Update failed" };
    }
  }

  return { error: "Unknown action" };
}

export default function AppSettingsTab({ actionData }: Route.ComponentProps) {
  const { app, token } = useOutletContext<OutletContext>();
  const navigation = useNavigation();
  const queryClient = useQueryClient();
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);

  const isSubmitting = navigation.state === "submitting";

  useEffect(() => {
    if (actionData?.success) {
      toast.success("Settings saved");
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    }
    if (actionData?.error) {
      toast.error(actionData.error);
    }
  }, [actionData, app.id, queryClient]);

  return (
    <div className="space-y-6">
      {/* General Settings */}
      <Card>
        <CardHeader>
          <CardTitle>Application Settings</CardTitle>
          <CardDescription>
            Update your application configuration. Changes will take effect on the next deployment.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="space-y-6">
            <input type="hidden" name="intent" value="update" />

            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="name">Name</Label>
                <Input id="name" name="name" defaultValue={app.name} />
              </div>
              <div className="space-y-2">
                <Label htmlFor="git_url">Git URL</Label>
                <Input id="git_url" name="git_url" defaultValue={app.git_url} />
              </div>
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="branch">Branch</Label>
                <Input id="branch" name="branch" defaultValue={app.branch} />
              </div>
              <div className="space-y-2">
                <Label htmlFor="port">Port</Label>
                <Input id="port" name="port" type="number" defaultValue={app.port} />
              </div>
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="dockerfile">Dockerfile</Label>
                <Input id="dockerfile" name="dockerfile" defaultValue={app.dockerfile} />
              </div>
              <div className="space-y-2">
                <Label htmlFor="domain">Domain</Label>
                <Input id="domain" name="domain" placeholder="app.example.com" defaultValue={app.domain || ""} />
              </div>
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="healthcheck">Healthcheck Path</Label>
                <Input id="healthcheck" name="healthcheck" placeholder="/health" defaultValue={app.healthcheck || ""} />
              </div>
              <div className="space-y-2">
                <Label htmlFor="environment">Environment</Label>
                <Select name="environment" defaultValue={app.environment || "development"}>
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
              </div>
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="cpu_limit">CPU Limit</Label>
                <Select name="cpu_limit" defaultValue={app.cpu_limit || "1"}>
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
              </div>
              <div className="space-y-2">
                <Label htmlFor="memory_limit">Memory Limit</Label>
                <Select name="memory_limit" defaultValue={app.memory_limit || "512m"}>
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
              </div>
            </div>

            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? "Saving..." : "Save Changes"}
            </Button>
          </Form>
        </CardContent>
      </Card>

      {/* Environment Variables */}
      <EnvVarsTab appId={app.id} token={token} />

      {/* Danger Zone */}
      <Card className="border-destructive/50">
        <CardHeader>
          <CardTitle className="text-destructive">Danger Zone</CardTitle>
          <CardDescription>
            Irreversible actions that will affect your application.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Button variant="destructive" onClick={() => setShowDeleteDialog(true)}>
            Delete Application
          </Button>
        </CardContent>
      </Card>

      {/* Delete Confirmation Dialog */}
      <Dialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Application</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete "{app.name}"? This action cannot
              be undone. All deployments and logs will be permanently deleted.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowDeleteDialog(false)}>
              Cancel
            </Button>
            <Form method="post" action={`/apps/${app.id}`}>
              <input type="hidden" name="intent" value="delete" />
              <Button type="submit" variant="destructive" disabled={isSubmitting}>
                {isSubmitting ? "Deleting..." : "Delete"}
              </Button>
            </Form>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
