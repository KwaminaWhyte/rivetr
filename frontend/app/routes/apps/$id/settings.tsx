import { useState } from "react";
import { useOutletContext, useNavigate } from "react-router";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
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
import { ContainerLabelsCard } from "@/components/container-labels-card";
import { DeploymentCommandsCard } from "@/components/deployment-commands-card";
import { DockerRegistryCard } from "@/components/docker-registry-card";
import { DomainManagementCard } from "@/components/domain-management-card";
import { EnvVarsTab } from "@/components/env-vars-tab";
import { NetworkConfigCard } from "@/components/network-config-card";
import { VolumesCard } from "@/components/volumes-card";
import { api } from "@/lib/api";
import type { App, AppEnvironment, UpdateAppRequest } from "@/types/api";

const ENVIRONMENT_OPTIONS: { value: AppEnvironment; label: string }[] = [
  { value: "development", label: "Development" },
  { value: "staging", label: "Staging" },
  { value: "production", label: "Production" },
];

interface OutletContext {
  app: App;
}

export default function AppSettingsTab() {
  const { app } = useOutletContext<OutletContext>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [deletePassword, setDeletePassword] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isSavingNetwork, setIsSavingNetwork] = useState(false);
  const [isSavingDomains, setIsSavingDomains] = useState(false);
  const [isSavingLabels, setIsSavingLabels] = useState(false);

  // Form state for general settings
  const [generalForm, setGeneralForm] = useState({
    name: app.name,
    git_url: app.git_url,
    branch: app.branch,
    port: app.port,
    environment: app.environment || "development",
    healthcheck: app.healthcheck || "",
  });

  // Form state for build settings
  const [buildForm, setBuildForm] = useState({
    dockerfile: app.dockerfile,
    dockerfile_path: app.dockerfile_path || "",
    base_directory: app.base_directory || "",
    build_target: app.build_target || "",
    watch_paths: app.watch_paths || "",
    custom_docker_options: app.custom_docker_options || "",
  });

  // Handle general settings form submission
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

  // Handle build settings form submission
  const handleBuildSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    try {
      const updates: UpdateAppRequest = {
        dockerfile: buildForm.dockerfile,
        dockerfile_path: buildForm.dockerfile_path,
        base_directory: buildForm.base_directory,
        build_target: buildForm.build_target,
        watch_paths: buildForm.watch_paths,
        custom_docker_options: buildForm.custom_docker_options,
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

  // Handle delete action
  const handleDelete = async () => {
    if (!deletePassword.trim()) return;
    setIsSubmitting(true);
    try {
      await api.deleteApp(app.id, deletePassword);
      toast.success("Application deleted");
      navigate("/projects");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Delete failed");
    } finally {
      setIsSubmitting(false);
    }
  };

  // Handler for saving network configuration
  const handleSaveNetworkConfig = async (updates: UpdateAppRequest) => {
    setIsSavingNetwork(true);
    try {
      await api.updateApp(app.id, updates);
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } finally {
      setIsSavingNetwork(false);
    }
  };

  // Handler for saving domain configuration
  const handleSaveDomainConfig = async (updates: UpdateAppRequest) => {
    setIsSavingDomains(true);
    try {
      await api.updateApp(app.id, updates);
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } finally {
      setIsSavingDomains(false);
    }
  };

  // Handler for saving container labels
  const handleSaveContainerLabels = async (updates: UpdateAppRequest) => {
    setIsSavingLabels(true);
    try {
      await api.updateApp(app.id, updates);
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } finally {
      setIsSavingLabels(false);
    }
  };

  return (
    <div className="space-y-6">
      <Tabs defaultValue="general" className="w-full">
        <TabsList className="grid w-full grid-cols-5">
          <TabsTrigger value="general">General</TabsTrigger>
          <TabsTrigger value="build">Build</TabsTrigger>
          <TabsTrigger value="network">Network</TabsTrigger>
          <TabsTrigger value="storage">Storage</TabsTrigger>
          <TabsTrigger value="security">Security</TabsTrigger>
        </TabsList>

        {/* General Tab */}
        <TabsContent value="general" className="space-y-6">
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
        </TabsContent>

        {/* Build Tab */}
        <TabsContent value="build" className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Build Configuration</CardTitle>
              <CardDescription>
                Configure how your application is built with Docker.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <form onSubmit={handleBuildSubmit} className="space-y-6">
                <div className="grid gap-4 md:grid-cols-2">
                  <div className="space-y-2">
                    <Label htmlFor="dockerfile">Dockerfile</Label>
                    <Input
                      id="dockerfile"
                      value={buildForm.dockerfile}
                      onChange={(e) => setBuildForm({ ...buildForm, dockerfile: e.target.value })}
                    />
                    <p className="text-xs text-muted-foreground">
                      Dockerfile name (e.g., Dockerfile)
                    </p>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="dockerfile_path">Dockerfile Path</Label>
                    <Input
                      id="dockerfile_path"
                      placeholder="Dockerfile.prod"
                      value={buildForm.dockerfile_path}
                      onChange={(e) => setBuildForm({ ...buildForm, dockerfile_path: e.target.value })}
                    />
                    <p className="text-xs text-muted-foreground">
                      Custom Dockerfile location (relative to base directory)
                    </p>
                  </div>
                </div>

                <div className="grid gap-4 md:grid-cols-2">
                  <div className="space-y-2">
                    <Label htmlFor="base_directory">Base Directory</Label>
                    <Input
                      id="base_directory"
                      placeholder="backend/"
                      value={buildForm.base_directory}
                      onChange={(e) => setBuildForm({ ...buildForm, base_directory: e.target.value })}
                    />
                    <p className="text-xs text-muted-foreground">
                      Subdirectory to use as build context
                    </p>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="build_target">Build Target</Label>
                    <Input
                      id="build_target"
                      placeholder="production"
                      value={buildForm.build_target}
                      onChange={(e) => setBuildForm({ ...buildForm, build_target: e.target.value })}
                    />
                    <p className="text-xs text-muted-foreground">
                      Multi-stage build target (--target flag)
                    </p>
                  </div>
                </div>

                <div className="grid gap-4 md:grid-cols-2">
                  <div className="space-y-2">
                    <Label htmlFor="watch_paths">Watch Paths</Label>
                    <Input
                      id="watch_paths"
                      placeholder='["src/", "package.json"]'
                      value={buildForm.watch_paths}
                      onChange={(e) => setBuildForm({ ...buildForm, watch_paths: e.target.value })}
                    />
                    <p className="text-xs text-muted-foreground">
                      JSON array of paths to trigger auto-deploy
                    </p>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="custom_docker_options">Custom Docker Options</Label>
                    <Textarea
                      id="custom_docker_options"
                      placeholder="--no-cache --build-arg FOO=bar"
                      rows={2}
                      value={buildForm.custom_docker_options}
                      onChange={(e) => setBuildForm({ ...buildForm, custom_docker_options: e.target.value })}
                    />
                    <p className="text-xs text-muted-foreground">
                      Extra Docker build arguments
                    </p>
                  </div>
                </div>

                <Button type="submit" disabled={isSubmitting}>
                  {isSubmitting ? "Saving..." : "Save Changes"}
                </Button>
              </form>
            </CardContent>
          </Card>

          {/* Docker Registry / Deployment Source */}
          <DockerRegistryCard app={app} />
        </TabsContent>

        {/* Network Tab */}
        <TabsContent value="network" className="space-y-6">
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

          {/* Container Labels */}
          <ContainerLabelsCard
            app={app}
            onSave={handleSaveContainerLabels}
            isSaving={isSavingLabels}
          />
        </TabsContent>

        {/* Storage Tab */}
        <TabsContent value="storage" className="space-y-6">
          {/* Volumes */}
          <VolumesCard appId={app.id} />

          {/* Environment Variables */}
          <EnvVarsTab appId={app.id} />
        </TabsContent>

        {/* Security Tab */}
        <TabsContent value="security" className="space-y-6">
          {/* HTTP Basic Auth */}
          <BasicAuthCard appId={app.id} />

          {/* Deployment Commands */}
          <DeploymentCommandsCard
            app={app}
            onSave={() => queryClient.invalidateQueries({ queryKey: ["app", app.id] })}
          />

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
        </TabsContent>
      </Tabs>

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
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="delete-password">Enter your password to confirm</Label>
              <Input
                id="delete-password"
                type="password"
                placeholder="Password"
                value={deletePassword}
                onChange={(e) => setDeletePassword(e.target.value)}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => {
              setShowDeleteDialog(false);
              setDeletePassword("");
            }}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              disabled={isSubmitting || !deletePassword.trim()}
              onClick={handleDelete}
            >
              {isSubmitting ? "Deleting..." : "Delete"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
