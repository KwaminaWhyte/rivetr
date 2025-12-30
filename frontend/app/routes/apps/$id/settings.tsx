import { useState, useEffect } from "react";
import { useOutletContext, useNavigate } from "react-router";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Switch } from "@/components/ui/switch";
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
import { Sparkles, FileCode, Package } from "lucide-react";
import { BasicAuthCard } from "@/components/basic-auth-card";
import { ContainerLabelsCard } from "@/components/container-labels-card";
import { GitHubSourceCard } from "@/components/github-source-card";
import { DeploymentCommandsCard } from "@/components/deployment-commands-card";
import { DockerRegistryCard } from "@/components/docker-registry-card";
import { DomainManagementCard } from "@/components/domain-management-card";
import { EnvVarsTab } from "@/components/env-vars-tab";
import { NetworkConfigCard } from "@/components/network-config-card";
import { VolumesCard } from "@/components/volumes-card";
import { api } from "@/lib/api";
import type { App, AppEnvironment, BuildType, NixpacksConfig, UpdateAppRequest } from "@/types/api";

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

  // Build type state
  const [buildType, setBuildType] = useState<BuildType>(app.build_type || "dockerfile");
  const [previewEnabled, setPreviewEnabled] = useState(app.preview_enabled || false);
  const [publishDirectory, setPublishDirectory] = useState(app.publish_directory || "dist");

  // Parse nixpacks config from JSON string
  const parseNixpacksConfig = (json: string | null): NixpacksConfig => {
    if (!json) return {};
    try {
      return JSON.parse(json);
    } catch {
      return {};
    }
  };

  const [nixpacksConfig, setNixpacksConfig] = useState<NixpacksConfig>(
    parseNixpacksConfig(app.nixpacks_config)
  );

  // Update state when app changes
  useEffect(() => {
    setBuildType(app.build_type || "dockerfile");
    setPreviewEnabled(app.preview_enabled || false);
    setPublishDirectory(app.publish_directory || "dist");
    setNixpacksConfig(parseNixpacksConfig(app.nixpacks_config));
  }, [app.build_type, app.preview_enabled, app.publish_directory, app.nixpacks_config]);

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
      // Build Nixpacks config if build type is nixpacks
      let nixpacksConfigToSend: NixpacksConfig | undefined = undefined;
      if (buildType === "nixpacks") {
        nixpacksConfigToSend = {};
        if (nixpacksConfig.install_cmd) nixpacksConfigToSend.install_cmd = nixpacksConfig.install_cmd;
        if (nixpacksConfig.build_cmd) nixpacksConfigToSend.build_cmd = nixpacksConfig.build_cmd;
        if (nixpacksConfig.start_cmd) nixpacksConfigToSend.start_cmd = nixpacksConfig.start_cmd;
        if (nixpacksConfig.packages?.length) nixpacksConfigToSend.packages = nixpacksConfig.packages;
        if (nixpacksConfig.apt_packages?.length) nixpacksConfigToSend.apt_packages = nixpacksConfig.apt_packages;
        if (Object.keys(nixpacksConfigToSend).length === 0) {
          nixpacksConfigToSend = undefined;
        }
      }

      const updates: UpdateAppRequest = {
        dockerfile: buildType === "dockerfile" ? buildForm.dockerfile : undefined,
        dockerfile_path: buildType === "dockerfile" ? buildForm.dockerfile_path : undefined,
        base_directory: buildForm.base_directory,
        build_target: buildType === "dockerfile" ? buildForm.build_target : undefined,
        watch_paths: buildForm.watch_paths,
        custom_docker_options: buildType === "dockerfile" ? buildForm.custom_docker_options : undefined,
        build_type: buildType,
        nixpacks_config: nixpacksConfigToSend,
        publish_directory: buildType === "static" ? publishDirectory : undefined,
        preview_enabled: previewEnabled,
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
          {/* GitHub App Connection (shown if connected) */}
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
        </TabsContent>

        {/* Build Tab */}
        <TabsContent value="build" className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Build Configuration</CardTitle>
              <CardDescription>
                Configure how your application is built.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <form onSubmit={handleBuildSubmit} className="space-y-6">
                {/* Build Type Selection */}
                <div className="space-y-3">
                  <Label>Build Type</Label>
                  <div className="grid grid-cols-3 gap-3">
                    <button
                      type="button"
                      onClick={() => setBuildType("nixpacks")}
                      className={`flex flex-col items-center gap-2 p-4 rounded-lg border-2 transition-colors ${
                        buildType === "nixpacks"
                          ? "border-primary bg-primary/5"
                          : "border-border hover:border-muted-foreground/50"
                      }`}
                    >
                      <Sparkles className="h-6 w-6" />
                      <span className="text-sm font-medium">Nixpacks</span>
                      <span className="text-xs text-muted-foreground text-center">
                        Auto-detect
                      </span>
                    </button>
                    <button
                      type="button"
                      onClick={() => setBuildType("dockerfile")}
                      className={`flex flex-col items-center gap-2 p-4 rounded-lg border-2 transition-colors ${
                        buildType === "dockerfile"
                          ? "border-primary bg-primary/5"
                          : "border-border hover:border-muted-foreground/50"
                      }`}
                    >
                      <FileCode className="h-6 w-6" />
                      <span className="text-sm font-medium">Dockerfile</span>
                      <span className="text-xs text-muted-foreground text-center">
                        Custom build
                      </span>
                    </button>
                    <button
                      type="button"
                      onClick={() => setBuildType("static")}
                      className={`flex flex-col items-center gap-2 p-4 rounded-lg border-2 transition-colors ${
                        buildType === "static"
                          ? "border-primary bg-primary/5"
                          : "border-border hover:border-muted-foreground/50"
                      }`}
                    >
                      <Package className="h-6 w-6" />
                      <span className="text-sm font-medium">Static</span>
                      <span className="text-xs text-muted-foreground text-center">
                        HTML/CSS/JS
                      </span>
                    </button>
                  </div>
                </div>

                {/* Nixpacks options */}
                {buildType === "nixpacks" && (
                  <div className="space-y-4 p-4 bg-muted/50 rounded-lg">
                    <p className="text-sm text-muted-foreground">
                      Nixpacks will automatically detect your project type and build it.
                      You can optionally override the default commands.
                    </p>
                    <div className="grid gap-4 md:grid-cols-3">
                      <div className="space-y-2">
                        <Label htmlFor="install_cmd">Install Command</Label>
                        <Input
                          id="install_cmd"
                          placeholder="npm install"
                          value={nixpacksConfig.install_cmd || ""}
                          onChange={(e) => setNixpacksConfig({ ...nixpacksConfig, install_cmd: e.target.value || undefined })}
                        />
                      </div>
                      <div className="space-y-2">
                        <Label htmlFor="build_cmd">Build Command</Label>
                        <Input
                          id="build_cmd"
                          placeholder="npm run build"
                          value={nixpacksConfig.build_cmd || ""}
                          onChange={(e) => setNixpacksConfig({ ...nixpacksConfig, build_cmd: e.target.value || undefined })}
                        />
                      </div>
                      <div className="space-y-2">
                        <Label htmlFor="start_cmd">Start Command</Label>
                        <Input
                          id="start_cmd"
                          placeholder="npm start"
                          value={nixpacksConfig.start_cmd || ""}
                          onChange={(e) => setNixpacksConfig({ ...nixpacksConfig, start_cmd: e.target.value || undefined })}
                        />
                      </div>
                    </div>
                  </div>
                )}

                {/* Static options */}
                {buildType === "static" && (
                  <div className="space-y-2">
                    <Label htmlFor="publish_directory">Publish Directory</Label>
                    <Input
                      id="publish_directory"
                      placeholder="dist, build, public"
                      value={publishDirectory}
                      onChange={(e) => setPublishDirectory(e.target.value)}
                    />
                    <p className="text-xs text-muted-foreground">
                      Directory containing your built static files
                    </p>
                  </div>
                )}

                {/* Dockerfile options */}
                {buildType === "dockerfile" && (
                  <>
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
                  </>
                )}

                {/* Common settings */}
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
                </div>

                {/* Preview deployments toggle */}
                <div className="flex items-center justify-between p-4 rounded-lg border">
                  <div className="space-y-0.5">
                    <Label htmlFor="preview-enabled" className="text-base">Enable PR Previews</Label>
                    <p className="text-sm text-muted-foreground">
                      Automatically deploy preview environments for pull requests
                    </p>
                  </div>
                  <Switch
                    id="preview-enabled"
                    checked={previewEnabled}
                    onCheckedChange={setPreviewEnabled}
                  />
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
