import { useState, useEffect } from "react";
import { useOutletContext, useNavigate } from "react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

export function meta() {
  return [
    { title: "App Settings - Rivetr" },
    { name: "description", content: "Configure application settings, environment variables, and resources" },
  ];
}
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { bulkApi } from "@/lib/api/bulk";
import type { ConfigSnapshot } from "@/types/api";
import { Camera, RotateCcw, Trash2, Plus, Shield, Snowflake } from "lucide-react";
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
import { Sparkles, FileCode, Package, Zap, Cloud, Bell } from "lucide-react";
import { AlertsCard } from "@/components/alerts-card";
import { BasicAuthCard } from "@/components/basic-auth-card";
import { ContainerLabelsCard } from "@/components/container-labels-card";
import { GitHubSourceCard } from "@/components/github-source-card";
import { DeploymentCommandsCard } from "@/components/deployment-commands-card";
import { DockerRegistryCard } from "@/components/docker-registry-card";
import { DomainManagementCard } from "@/components/domain-management-card";
import { EnvVarsTab } from "@/components/env-vars-tab";
import { NetworkConfigCard } from "@/components/network-config-card";
import { VolumesCard } from "@/components/volumes-card";
import { RollbackSettingsCard } from "@/components/rollback-settings-card";
import { AppSharingCard } from "@/components/app-sharing-card";
import { api } from "@/lib/api";
import { replicasApi, type AppReplica } from "@/lib/api/replicas";
import { autoscalingApi } from "@/lib/api/autoscaling";
import { buildServersApi, type BuildServer } from "@/lib/api/build-servers";
import type {
  App,
  AppEnvironment,
  BuildType,
  NixpacksConfig,
  UpdateAppRequest,
  DeploymentFreezeWindow,
  CreateFreezeWindowRequest,
  AutoscalingRule,
  CreateAutoscalingRuleRequest,
} from "@/types/api";
import { Badge } from "@/components/ui/badge";

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

  // Snapshot state
  const [showSnapshotDialog, setShowSnapshotDialog] = useState(false);
  const [snapshotName, setSnapshotName] = useState("");
  const [snapshotDescription, setSnapshotDescription] = useState("");
  const [isSavingSnapshot, setIsSavingSnapshot] = useState(false);

  // Approval & maintenance mode state
  const [requireApproval, setRequireApproval] = useState(app.require_approval ?? false);
  const [maintenanceMode, setMaintenanceMode] = useState(app.maintenance_mode ?? false);
  const [maintenanceMessage, setMaintenanceMessage] = useState(
    app.maintenance_message ?? "Service temporarily unavailable"
  );
  const [isSavingDeployControl, setIsSavingDeployControl] = useState(false);

  // Freeze windows state
  const [showFreezeWindowDialog, setShowFreezeWindowDialog] = useState(false);
  const [freezeWindowForm, setFreezeWindowForm] = useState<CreateFreezeWindowRequest>({
    name: "",
    start_time: "22:00",
    end_time: "06:00",
    days_of_week: "0,1,2,3,4,5,6",
    app_id: app.id,
  });
  const [isSavingFreezeWindow, setIsSavingFreezeWindow] = useState(false);

  // Sync approval/maintenance from app data
  useEffect(() => {
    setRequireApproval(app.require_approval ?? false);
    setMaintenanceMode(app.maintenance_mode ?? false);
    setMaintenanceMessage(app.maintenance_message ?? "Service temporarily unavailable");
  }, [app.require_approval, app.maintenance_mode, app.maintenance_message]);

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
  const [buildServerId, setBuildServerId] = useState<string>(app.build_server_id || "");

  // Fetch available build servers
  const { data: buildServers = [] } = useQuery<BuildServer[]>({
    queryKey: ["build-servers"],
    queryFn: () => buildServersApi.list(),
  });

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
    setBuildServerId(app.build_server_id || "");
  }, [app.build_type, app.preview_enabled, app.publish_directory, app.nixpacks_config, app.build_server_id]);

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
        publish_directory: buildType === "staticsite" ? publishDirectory : undefined,
        preview_enabled: previewEnabled,
        // Empty string clears the build server assignment on the backend
        build_server_id: buildServerId || "",
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

  // Snapshots
  const { data: snapshots = [], refetch: refetchSnapshots } = useQuery<ConfigSnapshot[]>({
    queryKey: ["snapshots", app.id],
    queryFn: () => bulkApi.listSnapshots(app.id),
  });

  const handleCreateSnapshot = async () => {
    if (!snapshotName.trim()) return;
    setIsSavingSnapshot(true);
    try {
      await bulkApi.createSnapshot(app.id, {
        name: snapshotName.trim(),
        description: snapshotDescription.trim() || undefined,
      });
      toast.success("Snapshot saved");
      setShowSnapshotDialog(false);
      setSnapshotName("");
      setSnapshotDescription("");
      refetchSnapshots();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to save snapshot");
    } finally {
      setIsSavingSnapshot(false);
    }
  };

  const handleRestoreSnapshot = async (snapshotId: string, name: string) => {
    try {
      await bulkApi.restoreSnapshot(app.id, snapshotId);
      toast.success(`Restored from snapshot "${name}"`);
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to restore snapshot");
    }
  };

  const handleDeleteSnapshot = async (snapshotId: string) => {
    try {
      await bulkApi.deleteSnapshot(app.id, snapshotId);
      toast.success("Snapshot deleted");
      refetchSnapshots();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to delete snapshot");
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

  // Handler for saving deployment control settings
  const handleSaveDeployControl = async () => {
    setIsSavingDeployControl(true);
    try {
      await api.updateApp(app.id, {
        require_approval: requireApproval,
        maintenance_mode: maintenanceMode,
        maintenance_message: maintenanceMessage,
      });
      toast.success("Deployment control settings saved");
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to save settings");
    } finally {
      setIsSavingDeployControl(false);
    }
  };

  // Freeze windows query
  const { data: freezeWindows = [], refetch: refetchFreezeWindows } = useQuery<
    DeploymentFreezeWindow[]
  >({
    queryKey: ["freeze-windows", app.id],
    queryFn: () => api.getFreezeWindows({ appId: app.id }),
  });

  // Create freeze window handler
  const handleCreateFreezeWindow = async () => {
    if (!freezeWindowForm.name.trim()) return;
    setIsSavingFreezeWindow(true);
    try {
      await api.createFreezeWindow(freezeWindowForm);
      toast.success("Freeze window created");
      setShowFreezeWindowDialog(false);
      setFreezeWindowForm({
        name: "",
        start_time: "22:00",
        end_time: "06:00",
        days_of_week: "0,1,2,3,4,5,6",
        app_id: app.id,
      });
      refetchFreezeWindows();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to create freeze window");
    } finally {
      setIsSavingFreezeWindow(false);
    }
  };

  // Rollback retention state
  const [rollbackRetentionCount, setRollbackRetentionCount] = useState(
    app.rollback_retention_count ?? 10
  );
  const [isSavingRetention, setIsSavingRetention] = useState(false);

  const handleSaveRetention = async () => {
    setIsSavingRetention(true);
    try {
      await api.updateApp(app.id, { rollback_retention_count: rollbackRetentionCount });
      toast.success("Rollback retention saved");
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to save");
    } finally {
      setIsSavingRetention(false);
    }
  };

  useEffect(() => {
    setRollbackRetentionCount(app.rollback_retention_count ?? 10);
  }, [app.rollback_retention_count]);

  // Autoscaling state
  const { data: autoscalingRules = [], refetch: refetchAutoscaling } = useQuery<AutoscalingRule[]>({
    queryKey: ["autoscaling", app.id],
    queryFn: () => autoscalingApi.list(app.id),
  });
  const [showAutoscalingDialog, setShowAutoscalingDialog] = useState(false);
  const [editingRule, setEditingRule] = useState<AutoscalingRule | null>(null);
  const [autoscalingForm, setAutoscalingForm] = useState<CreateAutoscalingRuleRequest>({
    metric: "cpu",
    scale_up_threshold: 80,
    scale_down_threshold: 20,
    min_replicas: 1,
    max_replicas: 10,
    cooldown_seconds: 300,
    enabled: true,
  });
  const [isSavingAutoscaling, setIsSavingAutoscaling] = useState(false);

  const handleOpenAutoscalingDialog = (rule?: AutoscalingRule) => {
    if (rule) {
      setEditingRule(rule);
      setAutoscalingForm({
        metric: rule.metric,
        scale_up_threshold: rule.scale_up_threshold,
        scale_down_threshold: rule.scale_down_threshold,
        min_replicas: rule.min_replicas,
        max_replicas: rule.max_replicas,
        cooldown_seconds: rule.cooldown_seconds,
        enabled: rule.enabled === 1,
      });
    } else {
      setEditingRule(null);
      setAutoscalingForm({
        metric: "cpu",
        scale_up_threshold: 80,
        scale_down_threshold: 20,
        min_replicas: 1,
        max_replicas: 10,
        cooldown_seconds: 300,
        enabled: true,
      });
    }
    setShowAutoscalingDialog(true);
  };

  const handleSaveAutoscalingRule = async () => {
    setIsSavingAutoscaling(true);
    try {
      if (editingRule) {
        await autoscalingApi.update(app.id, editingRule.id, autoscalingForm);
        toast.success("Autoscaling rule updated");
      } else {
        await autoscalingApi.create(app.id, autoscalingForm);
        toast.success("Autoscaling rule created");
      }
      setShowAutoscalingDialog(false);
      refetchAutoscaling();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to save rule");
    } finally {
      setIsSavingAutoscaling(false);
    }
  };

  const handleDeleteAutoscalingRule = async (ruleId: string) => {
    try {
      await autoscalingApi.delete(app.id, ruleId);
      toast.success("Rule deleted");
      refetchAutoscaling();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to delete rule");
    }
  };

  // Replicas state
  const [replicaCount, setReplicaCount] = useState(app.replica_count ?? 1);
  const [isSavingReplicas, setIsSavingReplicas] = useState(false);
  const [restartingReplica, setRestartingReplica] = useState<number | null>(null);

  const { data: replicas = [], refetch: refetchReplicas } = useQuery<AppReplica[]>({
    queryKey: ["replicas", app.id],
    queryFn: () => replicasApi.list(app.id),
  });

  const handleSetReplicaCount = async () => {
    setIsSavingReplicas(true);
    try {
      await replicasApi.setCount(app.id, replicaCount);
      toast.success(`Replica count updated to ${replicaCount}`);
      refetchReplicas();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to update replica count");
    } finally {
      setIsSavingReplicas(false);
    }
  };

  const handleRestartReplica = async (index: number) => {
    setRestartingReplica(index);
    try {
      await replicasApi.restart(app.id, index);
      toast.success(`Replica ${index} restarted`);
      refetchReplicas();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to restart replica");
    } finally {
      setRestartingReplica(null);
    }
  };

  // Delete freeze window handler
  const handleDeleteFreezeWindow = async (id: string) => {
    try {
      await api.deleteFreezeWindow(app.id, id);
      toast.success("Freeze window deleted");
      refetchFreezeWindows();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to delete freeze window");
    }
  };

  return (
    <div className="space-y-6">
      <Tabs defaultValue="general" className="w-full">
        <TabsList className="grid w-full grid-cols-9">
          <TabsTrigger value="general">General</TabsTrigger>
          <TabsTrigger value="build">Build</TabsTrigger>
          <TabsTrigger value="network">Network</TabsTrigger>
          <TabsTrigger value="storage">Storage</TabsTrigger>
          <TabsTrigger value="alerts">
            <Bell className="h-4 w-4 mr-1" />
            Alerts
          </TabsTrigger>
          <TabsTrigger value="security">Security</TabsTrigger>
          <TabsTrigger value="deployment">
            <Shield className="h-4 w-4 mr-1" />
            Deploy
          </TabsTrigger>
          <TabsTrigger value="replicas">Replicas</TabsTrigger>
          <TabsTrigger value="snapshots">Snapshots</TabsTrigger>
          <TabsTrigger value="sharing">Sharing</TabsTrigger>
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
                  <div className="grid grid-cols-3 md:grid-cols-5 gap-3">
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
                      onClick={() => setBuildType("railpack")}
                      className={`flex flex-col items-center gap-2 p-4 rounded-lg border-2 transition-colors ${
                        buildType === "railpack"
                          ? "border-primary bg-primary/5"
                          : "border-border hover:border-muted-foreground/50"
                      }`}
                    >
                      <Zap className="h-6 w-6" />
                      <span className="text-sm font-medium">Railpack</span>
                      <span className="text-xs text-muted-foreground text-center">
                        Fast builds
                      </span>
                    </button>
                    <button
                      type="button"
                      onClick={() => setBuildType("cnb")}
                      className={`flex flex-col items-center gap-2 p-4 rounded-lg border-2 transition-colors ${
                        buildType === "cnb"
                          ? "border-primary bg-primary/5"
                          : "border-border hover:border-muted-foreground/50"
                      }`}
                    >
                      <Cloud className="h-6 w-6" />
                      <span className="text-sm font-medium">Buildpacks</span>
                      <span className="text-xs text-muted-foreground text-center">
                        Paketo/Heroku
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
                      onClick={() => setBuildType("staticsite")}
                      className={`flex flex-col items-center gap-2 p-4 rounded-lg border-2 transition-colors ${
                        buildType === "staticsite"
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

                {/* Railpack options */}
                {buildType === "railpack" && (
                  <div className="space-y-4 p-4 bg-muted/50 rounded-lg">
                    <p className="text-sm text-muted-foreground">
                      Railpack (Railway's Nixpacks successor) offers faster builds with better caching.
                      38% faster for Node.js, 77% faster for Python compared to Nixpacks.
                    </p>
                    <p className="text-xs text-muted-foreground">
                      <strong>Note:</strong> Requires Railpack CLI and BuildKit. Linux/macOS only (Windows not supported).
                    </p>
                  </div>
                )}

                {/* Cloud Native Buildpacks options */}
                {buildType === "cnb" && (
                  <div className="space-y-4 p-4 bg-muted/50 rounded-lg">
                    <p className="text-sm text-muted-foreground">
                      Cloud Native Buildpacks (Paketo/Heroku) create production-ready, security-focused container images
                      without requiring a Dockerfile. Auto-selects the best builder for your project.
                    </p>
                    <p className="text-xs text-muted-foreground">
                      <strong>Note:</strong> Requires Pack CLI installed. Supports Java, Node.js, Python, Go, Ruby, .NET, and more.
                    </p>
                  </div>
                )}

                {/* Static options */}
                {buildType === "staticsite" && (
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

                {/* Build Server selection */}
                <div className="space-y-2">
                  <Label htmlFor="build-server">Build Server</Label>
                  <Select
                    value={buildServerId || "__local__"}
                    onValueChange={(v) => setBuildServerId(v === "__local__" ? "" : v)}
                  >
                    <SelectTrigger id="build-server">
                      <SelectValue placeholder="Local (default)" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="__local__">Local (default)</SelectItem>
                      {buildServers.map((bs) => (
                        <SelectItem key={bs.id} value={bs.id}>
                          {bs.name} ({bs.host}:{bs.port})
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <p className="text-xs text-muted-foreground">
                    Offload Docker builds to a dedicated remote build server. Leave blank to build locally.
                  </p>
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

        {/* Alerts Tab */}
        <TabsContent value="alerts" className="space-y-6">
          <AlertsCard appId={app.id} />
        </TabsContent>

        {/* Security Tab */}
        <TabsContent value="security" className="space-y-6">
          {/* Rollback Settings */}
          <RollbackSettingsCard app={app} />

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

        {/* Deployment Control Tab */}
        <TabsContent value="deployment" className="space-y-6">
          {/* Approval & Maintenance */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Shield className="h-5 w-5" />
                Deployment Control
              </CardTitle>
              <CardDescription>
                Control how deployments are triggered and when the app is accessible.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              {/* Require Approval toggle */}
              <div className="flex items-center justify-between rounded-lg border p-4">
                <div className="space-y-0.5">
                  <Label htmlFor="require-approval" className="text-base">
                    Require Approval
                  </Label>
                  <p className="text-sm text-muted-foreground">
                    Non-admin users must have their deployments approved by an admin before they run.
                  </p>
                </div>
                <Switch
                  id="require-approval"
                  checked={requireApproval}
                  onCheckedChange={setRequireApproval}
                />
              </div>

              {/* Maintenance Mode toggle */}
              <div className="space-y-3">
                <div className="flex items-center justify-between rounded-lg border p-4">
                  <div className="space-y-0.5">
                    <Label htmlFor="maintenance-mode" className="text-base">
                      Maintenance Mode
                    </Label>
                    <p className="text-sm text-muted-foreground">
                      Show a maintenance page instead of the live application.
                    </p>
                  </div>
                  <Switch
                    id="maintenance-mode"
                    checked={maintenanceMode}
                    onCheckedChange={setMaintenanceMode}
                  />
                </div>
                {maintenanceMode && (
                  <div className="space-y-2 ml-4">
                    <Label htmlFor="maintenance-message">Maintenance Message</Label>
                    <Input
                      id="maintenance-message"
                      value={maintenanceMessage}
                      onChange={(e) => setMaintenanceMessage(e.target.value)}
                      placeholder="Service temporarily unavailable"
                    />
                    <p className="text-xs text-muted-foreground">
                      Message shown to visitors during maintenance
                    </p>
                  </div>
                )}
              </div>

              <Button
                onClick={handleSaveDeployControl}
                disabled={isSavingDeployControl}
              >
                {isSavingDeployControl ? "Saving..." : "Save Changes"}
              </Button>
            </CardContent>
          </Card>

          {/* Freeze Windows */}
          <Card>
            <CardHeader className="flex flex-row items-center justify-between">
              <div>
                <CardTitle className="flex items-center gap-2">
                  <Snowflake className="h-5 w-5" />
                  Deployment Freeze Windows
                </CardTitle>
                <CardDescription>
                  Block deployments during specific time windows (e.g., business hours, weekends).
                </CardDescription>
              </div>
              <Button
                size="sm"
                className="gap-2"
                onClick={() => setShowFreezeWindowDialog(true)}
              >
                <Plus className="h-4 w-4" />
                Add Window
              </Button>
            </CardHeader>
            <CardContent>
              {freezeWindows.length === 0 ? (
                <div className="py-8 text-center text-muted-foreground">
                  No freeze windows configured. Add one to block deployments during specific times.
                </div>
              ) : (
                <div className="space-y-3">
                  {freezeWindows.map((fw) => (
                    <div
                      key={fw.id}
                      className="flex items-center justify-between rounded-md border p-3"
                    >
                      <div className="space-y-0.5">
                        <div className="flex items-center gap-2">
                          <p className="font-medium text-sm">{fw.name}</p>
                          {fw.is_active ? (
                            <Badge variant="secondary" className="text-xs">Active</Badge>
                          ) : (
                            <Badge variant="outline" className="text-xs text-muted-foreground">Inactive</Badge>
                          )}
                        </div>
                        <p className="text-xs text-muted-foreground">
                          {fw.start_time} – {fw.end_time} UTC
                          {" "}·{" "}
                          Days: {fw.days_of_week}
                        </p>
                      </div>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="gap-1.5 text-destructive hover:text-destructive"
                        onClick={() => handleDeleteFreezeWindow(fw.id)}
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                      </Button>
                    </div>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>

          {/* Rollback Retention */}
          <Card>
            <CardHeader>
              <CardTitle>Rollback Retention</CardTitle>
              <CardDescription>
                Number of previous successful deployments to keep available for rollback.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-end gap-4">
                <div className="space-y-2 flex-1 max-w-xs">
                  <Label htmlFor="rollback-retention">Retention Count</Label>
                  <Input
                    id="rollback-retention"
                    type="number"
                    min={1}
                    max={50}
                    value={rollbackRetentionCount}
                    onChange={(e) =>
                      setRollbackRetentionCount(
                        Math.max(1, Math.min(50, parseInt(e.target.value) || 10))
                      )
                    }
                  />
                  <p className="text-xs text-muted-foreground">
                    Number of deployments to retain for rollback (1–50, default 10).
                    Older successful deployments will be automatically deleted.
                  </p>
                </div>
                <Button onClick={handleSaveRetention} disabled={isSavingRetention}>
                  {isSavingRetention ? "Saving..." : "Save"}
                </Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        {/* Replicas Tab */}
        <TabsContent value="replicas" className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Container Replicas</CardTitle>
              <CardDescription>
                Run multiple container instances to distribute load. Changes apply on the next deployment.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              {/* Replica count input */}
              <div className="flex items-end gap-4">
                <div className="space-y-2 flex-1 max-w-xs">
                  <Label htmlFor="replica-count">Replica Count</Label>
                  <Input
                    id="replica-count"
                    type="number"
                    min={1}
                    max={10}
                    value={replicaCount}
                    onChange={(e) => setReplicaCount(Math.max(1, Math.min(10, parseInt(e.target.value) || 1)))}
                  />
                  <p className="text-xs text-muted-foreground">
                    Number of container instances to run (1–10)
                  </p>
                </div>
                <Button onClick={handleSetReplicaCount} disabled={isSavingReplicas}>
                  {isSavingReplicas ? "Updating..." : "Update"}
                </Button>
              </div>

              {/* Replica status table */}
              {replicas.length > 0 ? (
                <div className="rounded-md border">
                  <table className="w-full text-sm">
                    <thead>
                      <tr className="border-b bg-muted/50">
                        <th className="px-4 py-2 text-left font-medium">Index</th>
                        <th className="px-4 py-2 text-left font-medium">Container ID</th>
                        <th className="px-4 py-2 text-left font-medium">Status</th>
                        <th className="px-4 py-2 text-left font-medium">Started At</th>
                        <th className="px-4 py-2 text-right font-medium">Actions</th>
                      </tr>
                    </thead>
                    <tbody>
                      {replicas.map((replica) => (
                        <tr key={replica.id} className="border-b last:border-0">
                          <td className="px-4 py-2 font-mono">{replica.replica_index}</td>
                          <td className="px-4 py-2 font-mono text-xs text-muted-foreground">
                            {replica.container_id ? replica.container_id.slice(0, 12) : "—"}
                          </td>
                          <td className="px-4 py-2">
                            <Badge
                              variant={
                                replica.status === "running"
                                  ? "secondary"
                                  : replica.status === "error"
                                  ? "destructive"
                                  : "outline"
                              }
                              className="text-xs"
                            >
                              {replica.status}
                            </Badge>
                          </td>
                          <td className="px-4 py-2 text-xs text-muted-foreground">
                            {replica.started_at
                              ? new Date(replica.started_at).toLocaleString()
                              : "—"}
                          </td>
                          <td className="px-4 py-2 text-right">
                            <Button
                              variant="ghost"
                              size="sm"
                              className="gap-1.5"
                              disabled={restartingReplica === replica.replica_index}
                              onClick={() => handleRestartReplica(replica.replica_index)}
                            >
                              <RotateCcw className="h-3.5 w-3.5" />
                              {restartingReplica === replica.replica_index ? "Restarting..." : "Restart"}
                            </Button>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <div className="py-8 text-center text-muted-foreground">
                  No replica data yet. Deploy your app to start tracking replicas.
                </div>
              )}
            </CardContent>
          </Card>

          {/* Autoscaling */}
          <Card>
            <CardHeader className="flex flex-row items-center justify-between">
              <div>
                <CardTitle>Auto-scaling</CardTitle>
                <CardDescription>
                  Automatically scale replicas based on CPU or memory usage.
                </CardDescription>
              </div>
              <Button size="sm" className="gap-2" onClick={() => handleOpenAutoscalingDialog()}>
                <Plus className="h-4 w-4" />
                Add Rule
              </Button>
            </CardHeader>
            <CardContent>
              {autoscalingRules.length === 0 ? (
                <div className="py-8 text-center text-muted-foreground">
                  No autoscaling rules configured. Add a rule to enable automatic scaling.
                </div>
              ) : (
                <div className="space-y-3">
                  {autoscalingRules.map((rule) => (
                    <div
                      key={rule.id}
                      className="flex items-center justify-between rounded-md border p-3"
                    >
                      <div className="space-y-1">
                        <div className="flex items-center gap-2">
                          <p className="font-medium text-sm capitalize">{rule.metric}</p>
                          <Badge variant={rule.enabled === 1 ? "secondary" : "outline"} className="text-xs">
                            {rule.enabled === 1 ? "Enabled" : "Disabled"}
                          </Badge>
                        </div>
                        <p className="text-xs text-muted-foreground">
                          Scale up at {rule.scale_up_threshold}% · Scale down at {rule.scale_down_threshold}%
                          {" "}· {rule.min_replicas}–{rule.max_replicas} replicas · {rule.cooldown_seconds}s cooldown
                        </p>
                      </div>
                      <div className="flex gap-2">
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => handleOpenAutoscalingDialog(rule)}
                        >
                          Edit
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          className="text-destructive hover:text-destructive"
                          onClick={() => handleDeleteAutoscalingRule(rule.id)}
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </Button>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        {/* Snapshots Tab */}
        <TabsContent value="snapshots" className="space-y-6">
          <Card>
            <CardHeader className="flex flex-row items-center justify-between">
              <div>
                <CardTitle>Config Snapshots</CardTitle>
                <CardDescription>
                  Save named snapshots of your app configuration for quick restore.
                </CardDescription>
              </div>
              <Button onClick={() => setShowSnapshotDialog(true)} className="gap-2">
                <Camera className="h-4 w-4" />
                Take Snapshot
              </Button>
            </CardHeader>
            <CardContent>
              {snapshots.length === 0 ? (
                <div className="py-8 text-center text-muted-foreground">
                  No snapshots yet. Take a snapshot to save the current configuration.
                </div>
              ) : (
                <div className="space-y-3">
                  {snapshots.map((snap) => (
                    <div
                      key={snap.id}
                      className="flex items-center justify-between rounded-md border p-3"
                    >
                      <div>
                        <p className="font-medium text-sm">{snap.name}</p>
                        {snap.description && (
                          <p className="text-xs text-muted-foreground">{snap.description}</p>
                        )}
                        <p className="text-xs text-muted-foreground mt-1">
                          {new Date(snap.created_at).toLocaleString()}
                        </p>
                      </div>
                      <div className="flex gap-2">
                        <Button
                          variant="outline"
                          size="sm"
                          className="gap-1.5"
                          onClick={() => handleRestoreSnapshot(snap.id, snap.name)}
                        >
                          <RotateCcw className="h-3.5 w-3.5" />
                          Restore
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          className="gap-1.5 text-destructive hover:text-destructive"
                          onClick={() => handleDeleteSnapshot(snap.id)}
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </Button>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        {/* Sharing Tab */}
        <TabsContent value="sharing" className="space-y-6">
          <AppSharingCard app={app} />
        </TabsContent>
      </Tabs>

      {/* Freeze Window Create Dialog */}
      <Dialog
        open={showFreezeWindowDialog}
        onOpenChange={(open) => {
          setShowFreezeWindowDialog(open);
          if (!open) {
            setFreezeWindowForm({
              name: "",
              start_time: "22:00",
              end_time: "06:00",
              days_of_week: "0,1,2,3,4,5,6",
              app_id: app.id,
            });
          }
        }}
      >
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Add Freeze Window</DialogTitle>
            <DialogDescription>
              Define a time window during which deployments will be blocked. Times are in UTC.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="space-y-2">
              <Label htmlFor="fw-name">Name</Label>
              <Input
                id="fw-name"
                placeholder="e.g. Business Hours"
                value={freezeWindowForm.name}
                onChange={(e) =>
                  setFreezeWindowForm({ ...freezeWindowForm, name: e.target.value })
                }
              />
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="fw-start">Start Time (UTC)</Label>
                <Input
                  id="fw-start"
                  type="time"
                  value={freezeWindowForm.start_time}
                  onChange={(e) =>
                    setFreezeWindowForm({
                      ...freezeWindowForm,
                      start_time: e.target.value,
                    })
                  }
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="fw-end">End Time (UTC)</Label>
                <Input
                  id="fw-end"
                  type="time"
                  value={freezeWindowForm.end_time}
                  onChange={(e) =>
                    setFreezeWindowForm({
                      ...freezeWindowForm,
                      end_time: e.target.value,
                    })
                  }
                />
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="fw-days">Days of Week</Label>
              <Input
                id="fw-days"
                placeholder="0,1,2,3,4,5,6"
                value={freezeWindowForm.days_of_week}
                onChange={(e) =>
                  setFreezeWindowForm({
                    ...freezeWindowForm,
                    days_of_week: e.target.value,
                  })
                }
              />
              <p className="text-xs text-muted-foreground">
                Comma-separated: 0=Sunday, 1=Monday, ... 6=Saturday. Leave blank for all days.
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setShowFreezeWindowDialog(false)}
              disabled={isSavingFreezeWindow}
            >
              Cancel
            </Button>
            <Button
              onClick={handleCreateFreezeWindow}
              disabled={isSavingFreezeWindow || !freezeWindowForm.name.trim()}
              className="gap-2"
            >
              <Snowflake className="h-4 w-4" />
              {isSavingFreezeWindow ? "Creating..." : "Create Window"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Snapshot Dialog */}
      <Dialog open={showSnapshotDialog} onOpenChange={(open) => {
        setShowSnapshotDialog(open);
        if (!open) { setSnapshotName(""); setSnapshotDescription(""); }
      }}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Take Config Snapshot</DialogTitle>
            <DialogDescription>
              Save a named snapshot of the current app configuration and (masked) env vars.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="space-y-2">
              <Label htmlFor="snap-name">Snapshot Name</Label>
              <Input
                id="snap-name"
                placeholder="e.g. pre-upgrade-backup"
                value={snapshotName}
                onChange={(e) => setSnapshotName(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="snap-desc">Description (optional)</Label>
              <Textarea
                id="snap-desc"
                placeholder="What changed / why this snapshot..."
                value={snapshotDescription}
                onChange={(e) => setSnapshotDescription(e.target.value)}
                rows={2}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowSnapshotDialog(false)} disabled={isSavingSnapshot}>
              Cancel
            </Button>
            <Button onClick={handleCreateSnapshot} disabled={isSavingSnapshot || !snapshotName.trim()} className="gap-2">
              <Camera className="h-4 w-4" />
              {isSavingSnapshot ? "Saving..." : "Save Snapshot"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Autoscaling Rule Dialog */}
      <Dialog open={showAutoscalingDialog} onOpenChange={setShowAutoscalingDialog}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>{editingRule ? "Edit" : "Add"} Autoscaling Rule</DialogTitle>
            <DialogDescription>
              Configure when to scale replicas up or down based on a metric threshold.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="space-y-2">
              <Label>Metric</Label>
              <Select
                value={autoscalingForm.metric}
                onValueChange={(v) =>
                  setAutoscalingForm({ ...autoscalingForm, metric: v as "cpu" | "memory" | "request_rate" })
                }
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="cpu">CPU %</SelectItem>
                  <SelectItem value="memory">Memory %</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="scale-up">Scale Up Threshold (%)</Label>
                <Input
                  id="scale-up"
                  type="number"
                  min={0}
                  max={100}
                  value={autoscalingForm.scale_up_threshold}
                  onChange={(e) =>
                    setAutoscalingForm({
                      ...autoscalingForm,
                      scale_up_threshold: parseFloat(e.target.value) || 80,
                    })
                  }
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="scale-down">Scale Down Threshold (%)</Label>
                <Input
                  id="scale-down"
                  type="number"
                  min={0}
                  max={100}
                  value={autoscalingForm.scale_down_threshold}
                  onChange={(e) =>
                    setAutoscalingForm({
                      ...autoscalingForm,
                      scale_down_threshold: parseFloat(e.target.value) || 20,
                    })
                  }
                />
              </div>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="min-replicas">Min Replicas</Label>
                <Input
                  id="min-replicas"
                  type="number"
                  min={1}
                  max={100}
                  value={autoscalingForm.min_replicas}
                  onChange={(e) =>
                    setAutoscalingForm({
                      ...autoscalingForm,
                      min_replicas: parseInt(e.target.value) || 1,
                    })
                  }
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="max-replicas">Max Replicas</Label>
                <Input
                  id="max-replicas"
                  type="number"
                  min={1}
                  max={100}
                  value={autoscalingForm.max_replicas}
                  onChange={(e) =>
                    setAutoscalingForm({
                      ...autoscalingForm,
                      max_replicas: parseInt(e.target.value) || 10,
                    })
                  }
                />
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="cooldown">Cooldown (seconds)</Label>
              <Input
                id="cooldown"
                type="number"
                min={30}
                value={autoscalingForm.cooldown_seconds}
                onChange={(e) =>
                  setAutoscalingForm({
                    ...autoscalingForm,
                    cooldown_seconds: parseInt(e.target.value) || 300,
                  })
                }
              />
              <p className="text-xs text-muted-foreground">
                Minimum time between scaling actions
              </p>
            </div>
            <div className="flex items-center justify-between rounded-lg border p-3">
              <Label htmlFor="as-enabled">Enabled</Label>
              <Switch
                id="as-enabled"
                checked={autoscalingForm.enabled}
                onCheckedChange={(v) => setAutoscalingForm({ ...autoscalingForm, enabled: v })}
              />
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setShowAutoscalingDialog(false)}
              disabled={isSavingAutoscaling}
            >
              Cancel
            </Button>
            <Button onClick={handleSaveAutoscalingRule} disabled={isSavingAutoscaling}>
              {isSavingAutoscaling ? "Saving..." : editingRule ? "Update" : "Create"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

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
