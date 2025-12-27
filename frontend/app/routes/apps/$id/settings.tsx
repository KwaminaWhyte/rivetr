import { useState, useEffect } from "react";
import { Form, useNavigation, useOutletContext } from "react-router";
import { useQueryClient } from "@tanstack/react-query";
import type { Route } from "./+types/settings";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { ChevronDown } from "lucide-react";
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
import { BasicAuthCard } from "@/components/basic-auth-card";
import { DeploymentCommandsCard } from "@/components/deployment-commands-card";
import { DomainManagementCard } from "@/components/domain-management-card";
import { EnvVarsTab } from "@/components/env-vars-tab";
import { NetworkConfigCard } from "@/components/network-config-card";
import { api } from "@/lib/api";
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
    const port = formData.get("port");
    const healthcheck = formData.get("healthcheck");
    const environment = formData.get("environment");

    // For required fields, only set if present
    if (typeof name === "string" && name) updates.name = name;
    if (typeof git_url === "string" && git_url) updates.git_url = git_url;
    if (typeof branch === "string" && branch) updates.branch = branch;
    if (typeof dockerfile === "string" && dockerfile) updates.dockerfile = dockerfile;
    if (typeof port === "string" && port) updates.port = parseInt(port);
    if (typeof environment === "string" && environment) updates.environment = environment as AppEnvironment;

    // For optional fields, send empty string to clear, or the value to set
    // Don't include the field at all if not present in form
    if (typeof healthcheck === "string") updates.healthcheck = healthcheck; // Empty string means clear

    // Advanced build options - same pattern: empty string means clear
    const dockerfile_path = formData.get("dockerfile_path");
    const base_directory = formData.get("base_directory");
    const build_target = formData.get("build_target");
    const watch_paths = formData.get("watch_paths");
    const custom_docker_options = formData.get("custom_docker_options");

    if (typeof dockerfile_path === "string") updates.dockerfile_path = dockerfile_path;
    if (typeof base_directory === "string") updates.base_directory = base_directory;
    if (typeof build_target === "string") updates.build_target = build_target;
    if (typeof watch_paths === "string") updates.watch_paths = watch_paths;
    if (typeof custom_docker_options === "string") updates.custom_docker_options = custom_docker_options;

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
  const [buildOptionsOpen, setBuildOptionsOpen] = useState(
    Boolean(app.dockerfile_path || app.base_directory || app.build_target || app.watch_paths || app.custom_docker_options)
  );
  const [isSavingNetwork, setIsSavingNetwork] = useState(false);
  const [isSavingDomains, setIsSavingDomains] = useState(false);

  const isSubmitting = navigation.state === "submitting";

  // Handler for saving network configuration
  const handleSaveNetworkConfig = async (updates: UpdateAppRequest) => {
    setIsSavingNetwork(true);
    try {
      await api.updateApp(app.id, updates, token);
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } finally {
      setIsSavingNetwork(false);
    }
  };

  // Handler for saving domain configuration
  const handleSaveDomainConfig = async (updates: UpdateAppRequest) => {
    setIsSavingDomains(true);
    try {
      await api.updateApp(app.id, updates, token);
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } finally {
      setIsSavingDomains(false);
    }
  };

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
                <Label htmlFor="healthcheck">Healthcheck Path</Label>
                <Input id="healthcheck" name="healthcheck" placeholder="/health" defaultValue={app.healthcheck || ""} />
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="environment">Environment</Label>
              <Select name="environment" defaultValue={app.environment || "development"}>
                <SelectTrigger className="w-full md:w-[200px]">
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

            {/* Advanced Build Options */}
            <Collapsible open={buildOptionsOpen} onOpenChange={setBuildOptionsOpen}>
              <CollapsibleTrigger asChild>
                <Button variant="ghost" className="w-full justify-between p-0 h-auto font-medium">
                  Advanced Build Options
                  <ChevronDown className={`h-4 w-4 transition-transform ${buildOptionsOpen ? "rotate-180" : ""}`} />
                </Button>
              </CollapsibleTrigger>
              <CollapsibleContent className="space-y-4 pt-4">
                <div className="grid gap-4 md:grid-cols-2">
                  <div className="space-y-2">
                    <Label htmlFor="dockerfile_path">Dockerfile Path</Label>
                    <Input
                      id="dockerfile_path"
                      name="dockerfile_path"
                      placeholder="Dockerfile.prod"
                      defaultValue={app.dockerfile_path || ""}
                    />
                    <p className="text-xs text-muted-foreground">
                      Custom Dockerfile location (relative to base directory)
                    </p>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="base_directory">Base Directory</Label>
                    <Input
                      id="base_directory"
                      name="base_directory"
                      placeholder="backend/"
                      defaultValue={app.base_directory || ""}
                    />
                    <p className="text-xs text-muted-foreground">
                      Subdirectory to use as build context
                    </p>
                  </div>
                </div>

                <div className="grid gap-4 md:grid-cols-2">
                  <div className="space-y-2">
                    <Label htmlFor="build_target">Build Target</Label>
                    <Input
                      id="build_target"
                      name="build_target"
                      placeholder="production"
                      defaultValue={app.build_target || ""}
                    />
                    <p className="text-xs text-muted-foreground">
                      Multi-stage build target (--target flag)
                    </p>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="watch_paths">Watch Paths</Label>
                    <Input
                      id="watch_paths"
                      name="watch_paths"
                      placeholder='["src/", "package.json"]'
                      defaultValue={app.watch_paths || ""}
                    />
                    <p className="text-xs text-muted-foreground">
                      JSON array of paths to trigger auto-deploy
                    </p>
                  </div>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="custom_docker_options">Custom Docker Options</Label>
                  <Textarea
                    id="custom_docker_options"
                    name="custom_docker_options"
                    placeholder="--no-cache --build-arg FOO=bar"
                    rows={2}
                    defaultValue={app.custom_docker_options || ""}
                  />
                  <p className="text-xs text-muted-foreground">
                    Extra Docker build arguments (e.g., --no-cache, --add-host)
                  </p>
                </div>
              </CollapsibleContent>
            </Collapsible>

            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? "Saving..." : "Save Changes"}
            </Button>
          </Form>
        </CardContent>
      </Card>

      {/* Environment Variables */}
      <EnvVarsTab appId={app.id} token={token} />

      {/* Domain Management */}
      <DomainManagementCard
        app={app}
        onSave={handleSaveDomainConfig}
        isSaving={isSavingDomains}
      />

      {/* Network Configuration */}
      <NetworkConfigCard
        app={app}
        onSave={handleSaveNetworkConfig}
        isSaving={isSavingNetwork}
      />

      {/* Deployment Commands */}
      <DeploymentCommandsCard
        app={app}
        token={token}
        onSave={() => queryClient.invalidateQueries({ queryKey: ["app", app.id] })}
      />

      {/* HTTP Basic Auth */}
      <BasicAuthCard appId={app.id} token={token} />

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
