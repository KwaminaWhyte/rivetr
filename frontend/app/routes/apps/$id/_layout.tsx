import { useState, useMemo, useEffect } from "react";
import {
  Link,
  Outlet,
  useLocation,
  useParams,
  useNavigate,
} from "react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";
import { EnvironmentBadge } from "@/components/environment-badge";
import { api } from "@/lib/api";
import { getPrimaryDomain } from "@/lib/utils";
import { bulkApi } from "@/lib/api/bulk";
import { useBreadcrumb } from "@/lib/breadcrumb-context";
import type { App, AppStatus, Deployment, DeploymentStatus, DeploymentListResponse, Project, GitCommit, GitTag, DeploymentFreezeWindow } from "@/types/api";
import {
  Play,
  Square,
  Circle,
  RotateCw,
  ChevronDown,
  Rocket,
  ExternalLink,
  Upload,
  GitCommitHorizontal,
  Tag,
  Copy,
  WrenchIcon,
  Link2,
} from "lucide-react";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ZipUploadZone } from "@/components/zip-upload-zone";
import type { BuildDetectionResult } from "@/types/api";

export function meta() {
  return [
    { title: "Application - Rivetr" },
    { name: "description", content: "Application overview and management" },
  ];
}

// Running status badge component
function RunningStatusBadge({ status }: { status?: AppStatus }) {
  if (!status) return null;

  if (status.running) {
    return (
      <Badge className="bg-green-500 text-white gap-1">
        <Circle className="h-2 w-2 fill-current" />
        Running
      </Badge>
    );
  }

  if (status.status === "stopped") {
    return (
      <Badge variant="secondary" className="gap-1">
        <Circle className="h-2 w-2" />
        Stopped
      </Badge>
    );
  }

  if (status.status === "not_deployed") {
    return (
      <Badge variant="outline" className="gap-1 text-muted-foreground">
        Not Deployed
      </Badge>
    );
  }

  return null;
}

const ACTIVE_STATUSES: DeploymentStatus[] = [
  "pending",
  "cloning",
  "building",
  "starting",
  "checking",
];

function isActiveDeployment(status: DeploymentStatus): boolean {
  return ACTIVE_STATUSES.includes(status);
}

const TAB_DEFS = [
  { id: "general", label: "General", path: "" },
  { id: "env-vars", label: "Env Vars", path: "/env-vars" },
  { id: "network", label: "Network", path: "/network" },
  { id: "settings", label: "Settings", path: "/settings" },
  { id: "deployments", label: "Deployments", path: "/deployments" },
  { id: "previews", label: "Previews", path: "/previews" },
  { id: "jobs", label: "Jobs", path: "/jobs" },
  { id: "logs", label: "Logs", path: "/logs" },
  { id: "log-drains", label: "Log Drains", path: "/log-drains" },
  { id: "monitoring", label: "Monitoring", path: "/monitoring" },
  { id: "terminal", label: "Terminal", path: "/terminal" },
];

export default function AppDetailLayout() {
  const { id } = useParams();
  const location = useLocation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [isSubmitting, setIsSubmitting] = useState(false);
  // Persist restart state in localStorage so a page refresh doesn't lose the indicator.
  // We store the timestamp; if it's less than 90 seconds old, the restart is still in progress.
  const RESTART_KEY = id ? `rivetr_restarting_${id}` : null;
  const [isRestarting, setIsRestarting] = useState(() => {
    if (!RESTART_KEY) return false;
    const ts = localStorage.getItem(RESTART_KEY);
    if (!ts) return false;
    return Date.now() - Number(ts) < 90_000;
  });
  const { setItems } = useBreadcrumb();

  // Upload deploy state
  const [showUploadDialog, setShowUploadDialog] = useState(false);
  const [isUploading, setIsUploading] = useState(false);
  const [uploadFile, setUploadFile] = useState<File | null>(null);
  const [detectionResult, setDetectionResult] = useState<BuildDetectionResult | null>(null);

  // Clone app state
  const [showCloneDialog, setShowCloneDialog] = useState(false);
  const [cloneName, setCloneName] = useState("");
  const [isCloning, setIsCloning] = useState(false);

  // Maintenance mode state
  const [isMaintenanceLoading, setIsMaintenanceLoading] = useState(false);

  // Deploy by commit/tag state
  const [showDeployOptionsDialog, setShowDeployOptionsDialog] = useState(false);
  const [deployTarget, setDeployTarget] = useState<"latest" | "commit" | "tag">("latest");
  const [selectedCommitSha, setSelectedCommitSha] = useState<string>("");
  const [selectedTagName, setSelectedTagName] = useState<string>("");

  // Use React Query for app data
  const {
    data: app,
    isLoading: appLoading,
    error: appError,
  } = useQuery<App>({
    queryKey: ["app", id],
    queryFn: () => api.getApp(id!),
    enabled: !!id,
  });

  // Fetch project for breadcrumb
  const { data: project } = useQuery<Project>({
    queryKey: ["project", app?.project_id],
    queryFn: () => api.getProject(app!.project_id!),
    enabled: !!app?.project_id,
  });

  // Set breadcrumbs when app and project are loaded
  useEffect(() => {
    if (app) {
      const breadcrumbs = [];
      if (project) {
        breadcrumbs.push({ label: project.name, href: `/projects/${project.id}` });
      } else {
        breadcrumbs.push({ label: "Projects", href: "/projects" });
      }
      breadcrumbs.push({ label: "Apps" });
      breadcrumbs.push({ label: app.name });
      setItems(breadcrumbs);
    }
  }, [app, project, setItems]);

  const { data: deploymentsData } = useQuery<DeploymentListResponse>({
    queryKey: ["deployments", id],
    queryFn: () => api.getDeployments(id!, { per_page: 20 }),
    enabled: !!id,
    refetchInterval: (query) => {
      const data = query.state.data;
      if (!data || data.items.length === 0) return 5000;
      const hasActive = data.items.some((d: Deployment) =>
        isActiveDeployment(d.status)
      );
      return hasActive ? 2000 : 30000;
    },
    refetchIntervalInBackground: false,
  });

  const deployments = deploymentsData?.items ?? [];

  // Count pending-approval deployments for this app
  const pendingApprovalCount = deployments.filter(
    (d: Deployment) => d.approval_status === "pending"
  ).length;

  // Fetch commits for deploy-by-commit (only when dialog is open)
  const { data: commits = [], isLoading: commitsLoading } = useQuery<GitCommit[]>({
    queryKey: ["commits", id],
    queryFn: () => api.getCommits(id!, 20),
    enabled: !!id && showDeployOptionsDialog && deployTarget === "commit",
  });

  // Fetch tags for deploy-by-tag (only when dialog is open)
  const { data: tags = [], isLoading: tagsLoading } = useQuery<GitTag[]>({
    queryKey: ["tags", id],
    queryFn: () => api.getTags(id!, 20),
    enabled: !!id && showDeployOptionsDialog && deployTarget === "tag",
  });

  // Query for app status (running/stopped)
  const { data: appStatus, refetch: refetchStatus } = useQuery<AppStatus>({
    queryKey: ["appStatus", id],
    queryFn: () => api.getAppStatus(id!),
    enabled: !!id,
    refetchInterval: 10000, // Poll every 10 seconds
  });

  const hasActiveDeployment = useMemo(() => {
    return deployments.some((d) => isActiveDeployment(d.status));
  }, [deployments]);

  // Handle deploy action with optional commit/tag targeting
  const handleDeploy = async (options?: { commit_sha?: string; git_tag?: string }) => {
    if (!id) return;
    setIsSubmitting(true);
    try {
      const deployment = await api.triggerDeploy(id, options);
      toast.success("Deployment started");
      queryClient.invalidateQueries({ queryKey: ["deployments", id] });
      // Navigate to deployment detail page to watch live logs
      navigate(`/apps/${id}/deployments/${deployment.id}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Deployment failed");
    } finally {
      setIsSubmitting(false);
    }
  };

  // Handle deploy from the options dialog
  const handleDeployWithOptions = async () => {
    if (deployTarget === "commit" && selectedCommitSha) {
      await handleDeploy({ commit_sha: selectedCommitSha });
    } else if (deployTarget === "tag" && selectedTagName) {
      await handleDeploy({ git_tag: selectedTagName });
    } else {
      await handleDeploy();
    }
    setShowDeployOptionsDialog(false);
    setDeployTarget("latest");
    setSelectedCommitSha("");
    setSelectedTagName("");
  };

  // Handle file selection for upload deploy
  const handleFileSelect = async (file: File) => {
    setUploadFile(file);
    setDetectionResult(null);

    // Auto-detect build type
    try {
      const result = await api.detectBuildType(file);
      setDetectionResult(result);
    } catch (error) {
      // Detection is optional, don't show error
      console.warn("Build detection failed:", error);
    }
  };

  // Handle upload deploy
  const handleUploadDeploy = async () => {
    if (!id || !uploadFile) return;
    setIsUploading(true);
    try {
      const result = await api.uploadDeploy(id, uploadFile);
      toast.success("Upload deployment started");
      setShowUploadDialog(false);
      setUploadFile(null);
      setDetectionResult(null);
      queryClient.invalidateQueries({ queryKey: ["deployments", id] });
      queryClient.invalidateQueries({ queryKey: ["app", id] });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Upload deployment failed");
    } finally {
      setIsUploading(false);
    }
  };

  // Handle start action
  const handleStart = async () => {
    if (!id) return;
    setIsSubmitting(true);
    try {
      await api.startApp(id);
      toast.success("Application started");
      refetchStatus();
      queryClient.invalidateQueries({ queryKey: ["app", id] });
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to start app"
      );
    } finally {
      setIsSubmitting(false);
    }
  };

  // Handle stop action
  const handleStop = async () => {
    if (!id) return;
    setIsSubmitting(true);
    try {
      await api.stopApp(id);
      toast.success("Application stopped");
      refetchStatus();
      queryClient.invalidateQueries({ queryKey: ["app", id] });
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to stop app"
      );
    } finally {
      setIsSubmitting(false);
    }
  };

  // Handle restart action
  const handleRestart = async () => {
    if (!id || !RESTART_KEY) return;
    localStorage.setItem(RESTART_KEY, String(Date.now()));
    setIsRestarting(true);
    try {
      await api.restartApp(id);
      toast.success("Application restarted successfully");
      refetchStatus();
      queryClient.invalidateQueries({ queryKey: ["app", id] });
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to restart app"
      );
    } finally {
      if (RESTART_KEY) localStorage.removeItem(RESTART_KEY);
      setIsRestarting(false);
    }
  };

  // Handle clone app
  const handleClone = async () => {
    if (!id) return;
    setIsCloning(true);
    try {
      const result = await bulkApi.cloneApp(id, cloneName ? { name: cloneName } : undefined);
      toast.success(`App cloned as "${result.app.name}"`);
      setShowCloneDialog(false);
      setCloneName("");
      navigate(`/apps/${result.app.id}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to clone app");
    } finally {
      setIsCloning(false);
    }
  };

  // Handle maintenance mode toggle
  const handleToggleMaintenance = async () => {
    if (!id || !app) return;
    const currentMode = (app as App & { maintenance_mode?: boolean }).maintenance_mode ?? false;
    setIsMaintenanceLoading(true);
    try {
      await bulkApi.setMaintenanceMode(id, { enabled: !currentMode });
      toast.success(currentMode ? "Maintenance mode disabled" : "Maintenance mode enabled");
      queryClient.invalidateQueries({ queryKey: ["app", id] });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to toggle maintenance mode");
    } finally {
      setIsMaintenanceLoading(false);
    }
  };

  // Determine active tab from path
  const basePath = `/apps/${id}`;
  const currentPath = location.pathname;
  const activeTab =
    TAB_DEFS.find((tab) => {
      if (tab.path === "") {
        return currentPath === basePath || currentPath === basePath + "/";
      }
      return currentPath.startsWith(basePath + tab.path);
    })?.id || "general";

  if (appLoading) {
    return (
      <div className="space-y-6">
        <div className="animate-pulse">
          <div className="h-8 bg-muted rounded w-1/3 mb-2"></div>
          <div className="h-4 bg-muted rounded w-1/2"></div>
        </div>
      </div>
    );
  }

  if (appError || !app) {
    return (
      <div className="space-y-6">
        <h1 className="text-3xl font-bold">Application Not Found</h1>
        <p className="text-muted-foreground">
          The application you're looking for doesn't exist or has been deleted.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center gap-3">
            <h1 className="text-3xl font-bold">{app.name}</h1>
            <EnvironmentBadge environment={app.environment} />
            <RunningStatusBadge status={appStatus} />
            {isRestarting && (
              <span className="flex items-center gap-1.5 text-sm font-normal text-amber-600">
                <span className="relative flex h-2 w-2">
                  <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-amber-400 opacity-75"></span>
                  <span className="relative inline-flex h-2 w-2 rounded-full bg-amber-500"></span>
                </span>
                Restarting
              </span>
            )}
            {!isRestarting && hasActiveDeployment && (
              <span className="flex items-center gap-1.5 text-sm font-normal text-blue-600">
                <span className="relative flex h-2 w-2">
                  <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-blue-400 opacity-75"></span>
                  <span className="relative inline-flex h-2 w-2 rounded-full bg-blue-500"></span>
                </span>
                Deploying
              </span>
            )}
          </div>
          <p className="text-muted-foreground">{app.git_url}</p>
        </div>
        <div className="flex gap-2">
          {/* Start/Stop/Restart buttons */}
          {appStatus?.status === "running" ? (
            <>
              <Button
                variant="outline"
                disabled={isSubmitting || isRestarting || hasActiveDeployment}
                className="gap-2"
                onClick={handleRestart}
              >
                <RotateCw className={`h-4 w-4 ${isRestarting ? "animate-spin" : ""}`} />
                {isRestarting ? "Restarting..." : "Restart"}
              </Button>
              <Button
                variant="outline"
                disabled={isSubmitting || hasActiveDeployment}
                className="gap-2"
                onClick={handleStop}
              >
                <Square className="h-4 w-4" />
                Stop
              </Button>
            </>
          ) : appStatus?.status === "stopped" ? (
            <Button
              variant="outline"
              disabled={isSubmitting || hasActiveDeployment}
              className="gap-2"
              onClick={handleStart}
            >
              <Play className="h-4 w-4" />
              Start
            </Button>
          ) : null}
          {/* Deploy button with dropdown */}
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                disabled={isSubmitting || hasActiveDeployment}
                className="gap-2"
              >
                <Rocket className="h-4 w-4" />
                {isSubmitting ? "Deploying..." : "Deploy"}
                <ChevronDown className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-60">
              <DropdownMenuItem onClick={() => handleDeploy()}>
                <Rocket className="h-4 w-4 mr-2" />
                Redeploy (latest commit)
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem onClick={() => setShowDeployOptionsDialog(true)}>
                <GitCommitHorizontal className="h-4 w-4 mr-2" />
                Deploy specific commit/tag
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem onClick={() => setShowUploadDialog(true)}>
                <Upload className="h-4 w-4 mr-2" />
                Deploy from ZIP file
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
          {/* Links dropdown — shows all app URLs */}
          {(() => {
            const parsedDomains: Array<{ domain: string; primary: boolean; redirect_www: boolean }> = (() => {
              try { return app.domains ? JSON.parse(app.domains) : []; } catch { return []; }
            })();
            const allLinks: string[] = [];
            for (const d of parsedDomains) {
              allLinks.push(d.domain);
            }
            if (app.auto_subdomain && !allLinks.includes(app.auto_subdomain)) {
              allLinks.push(app.auto_subdomain);
            }
            if (app.domain && !allLinks.includes(app.domain)) {
              allLinks.push(app.domain);
            }
            return (
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="outline" className="gap-2">
                    <Link2 className="h-4 w-4" />
                    Links
                    <ChevronDown className="h-4 w-4" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-72">
                  {allLinks.length === 0 ? (
                    <DropdownMenuItem disabled>
                      <span className="text-muted-foreground">No domains configured</span>
                    </DropdownMenuItem>
                  ) : (
                    allLinks.map((link) => (
                      <DropdownMenuItem key={link} asChild>
                        <a
                          href={`https://${link}`}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="flex items-center gap-2"
                        >
                          <ExternalLink className="h-4 w-4 shrink-0" />
                          <span className="truncate font-mono text-sm">{link}</span>
                        </a>
                      </DropdownMenuItem>
                    ))
                  )}
                </DropdownMenuContent>
              </DropdownMenu>
            );
          })()}

          {/* Open App button - prefer domains JSON > domain > auto_subdomain > host_port */}
          {appStatus?.running && (() => {
            const primaryDomain = getPrimaryDomain(app);
            if (primaryDomain) {
              return (
                <Button variant="outline" asChild className="gap-2">
                  <a href={`https://${primaryDomain}`} target="_blank" rel="noopener noreferrer">
                    <ExternalLink className="h-4 w-4" />
                    Open App
                  </a>
                </Button>
              );
            }
            if (appStatus.host_port) {
              const host = typeof window !== 'undefined' ? window.location.hostname : 'localhost';
              return (
                <Button variant="outline" asChild className="gap-2">
                  <a href={`http://${host}:${appStatus.host_port}`} target="_blank" rel="noopener noreferrer" title="No domain configured — accessing via host port">
                    <ExternalLink className="h-4 w-4" />
                    Open App
                    <span className="text-xs text-muted-foreground ml-1">
                      (port {appStatus.host_port})
                    </span>
                  </a>
                </Button>
              );
            }
            return null;
          })()}
          {/* More actions dropdown: Clone + Maintenance */}
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="outline" size="icon">
                <ChevronDown className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem
                onClick={() => {
                  setCloneName(`${app.name}-copy`);
                  setShowCloneDialog(true);
                }}
              >
                <Copy className="h-4 w-4 mr-2" />
                Clone App
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                onClick={handleToggleMaintenance}
                disabled={isMaintenanceLoading}
              >
                <WrenchIcon className="h-4 w-4 mr-2" />
                {(app as App & { maintenance_mode?: boolean }).maintenance_mode
                  ? "Disable Maintenance"
                  : "Enable Maintenance"}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
          {/* Maintenance mode badge */}
          {(app as App & { maintenance_mode?: boolean }).maintenance_mode && (
            <Badge variant="outline" className="gap-1 border-yellow-500 text-yellow-600">
              <WrenchIcon className="h-3 w-3" />
              Maintenance
            </Badge>
          )}
        </div>
      </div>

      {/* Tabs Navigation */}
      <Tabs value={activeTab} className="w-full">
        <TabsList className="w-full justify-start">
          {TAB_DEFS.map((tab) => (
            <TabsTrigger key={tab.id} value={tab.id} asChild>
              <Link to={`${basePath}${tab.path}`} className="gap-1.5">
                {tab.label}
                {tab.id === "deployments" && pendingApprovalCount > 0 && (
                  <Badge
                    variant="destructive"
                    className="h-4 min-w-4 px-1 text-[10px] rounded-full"
                  >
                    {pendingApprovalCount}
                  </Badge>
                )}
              </Link>
            </TabsTrigger>
          ))}
        </TabsList>
      </Tabs>

      {/* Tab Content via Outlet */}
      <Outlet context={{ app, deployments, deploymentsData }} />

      {/* Upload Deploy Dialog */}
      <Dialog
        open={showUploadDialog}
        onOpenChange={(open) => {
          setShowUploadDialog(open);
          if (!open) {
            setUploadFile(null);
            setDetectionResult(null);
          }
        }}
      >
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>Deploy from ZIP File</DialogTitle>
            <DialogDescription>
              Upload a ZIP file containing your project files. The build type will
              be auto-detected.
            </DialogDescription>
          </DialogHeader>

          <ZipUploadZone
            onFileSelect={handleFileSelect}
            isUploading={isUploading}
            detectionResult={detectionResult}
            disabled={isUploading}
          />

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setShowUploadDialog(false);
                setUploadFile(null);
                setDetectionResult(null);
              }}
              disabled={isUploading}
            >
              Cancel
            </Button>
            <Button
              onClick={handleUploadDeploy}
              disabled={!uploadFile || isUploading}
            >
              {isUploading ? "Deploying..." : "Deploy"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Deploy by Commit/Tag Dialog */}
      <Dialog
        open={showDeployOptionsDialog}
        onOpenChange={(open) => {
          setShowDeployOptionsDialog(open);
          if (!open) {
            setDeployTarget("latest");
            setSelectedCommitSha("");
            setSelectedTagName("");
          }
        }}
      >
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>Deploy Specific Version</DialogTitle>
            <DialogDescription>
              Choose to deploy the latest code, a specific commit, or a tagged release.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4">
            {/* Deploy target selection */}
            <div className="space-y-2">
              <Label className="text-sm font-medium">Deploy target</Label>
              <div className="flex gap-2">
                <Button
                  variant={deployTarget === "latest" ? "default" : "outline"}
                  size="sm"
                  onClick={() => setDeployTarget("latest")}
                  className="gap-1.5"
                >
                  <Rocket className="h-3.5 w-3.5" />
                  Latest
                </Button>
                <Button
                  variant={deployTarget === "commit" ? "default" : "outline"}
                  size="sm"
                  onClick={() => setDeployTarget("commit")}
                  className="gap-1.5"
                >
                  <GitCommitHorizontal className="h-3.5 w-3.5" />
                  Specific Commit
                </Button>
                <Button
                  variant={deployTarget === "tag" ? "default" : "outline"}
                  size="sm"
                  onClick={() => setDeployTarget("tag")}
                  className="gap-1.5"
                >
                  <Tag className="h-3.5 w-3.5" />
                  Specific Tag
                </Button>
              </div>
            </div>

            {/* Commit selector */}
            {deployTarget === "commit" && (
              <div className="space-y-2">
                <Label className="text-sm font-medium">Select commit</Label>
                {commitsLoading ? (
                  <div className="flex items-center gap-2 text-sm text-muted-foreground py-2">
                    <RotateCw className="h-4 w-4 animate-spin" />
                    Loading commits...
                  </div>
                ) : commits.length === 0 ? (
                  <p className="text-sm text-muted-foreground py-2">
                    No commits found. Make sure the app has a GitHub App connection.
                  </p>
                ) : (
                  <Select value={selectedCommitSha} onValueChange={setSelectedCommitSha}>
                    <SelectTrigger className="w-full">
                      <SelectValue placeholder="Choose a commit..." />
                    </SelectTrigger>
                    <SelectContent>
                      {commits.map((commit) => (
                        <SelectItem key={commit.sha} value={commit.sha}>
                          <span className="flex items-center gap-2">
                            <code className="text-xs font-mono bg-muted px-1 py-0.5 rounded">
                              {commit.sha.slice(0, 7)}
                            </code>
                            <span className="truncate max-w-[280px]">
                              {commit.message.split("\n")[0]}
                            </span>
                          </span>
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              </div>
            )}

            {/* Tag selector */}
            {deployTarget === "tag" && (
              <div className="space-y-2">
                <Label className="text-sm font-medium">Select tag</Label>
                {tagsLoading ? (
                  <div className="flex items-center gap-2 text-sm text-muted-foreground py-2">
                    <RotateCw className="h-4 w-4 animate-spin" />
                    Loading tags...
                  </div>
                ) : tags.length === 0 ? (
                  <p className="text-sm text-muted-foreground py-2">
                    No tags found. Make sure the repository has tagged releases.
                  </p>
                ) : (
                  <Select value={selectedTagName} onValueChange={setSelectedTagName}>
                    <SelectTrigger className="w-full">
                      <SelectValue placeholder="Choose a tag..." />
                    </SelectTrigger>
                    <SelectContent>
                      {tags.map((tag) => (
                        <SelectItem key={tag.name} value={tag.name}>
                          <span className="flex items-center gap-2">
                            <Tag className="h-3.5 w-3.5 text-muted-foreground" />
                            <span className="font-medium">{tag.name}</span>
                            <code className="text-xs font-mono text-muted-foreground">
                              {tag.sha.slice(0, 7)}
                            </code>
                          </span>
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              </div>
            )}
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setShowDeployOptionsDialog(false);
                setDeployTarget("latest");
                setSelectedCommitSha("");
                setSelectedTagName("");
              }}
              disabled={isSubmitting}
            >
              Cancel
            </Button>
            <Button
              onClick={handleDeployWithOptions}
              disabled={
                isSubmitting ||
                (deployTarget === "commit" && !selectedCommitSha) ||
                (deployTarget === "tag" && !selectedTagName)
              }
              className="gap-2"
            >
              <Rocket className="h-4 w-4" />
              {isSubmitting ? "Deploying..." : "Deploy"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Clone App Dialog */}
      <Dialog
        open={showCloneDialog}
        onOpenChange={(open) => {
          setShowCloneDialog(open);
          if (!open) setCloneName("");
        }}
      >
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Clone App</DialogTitle>
            <DialogDescription>
              Create a deep copy of this app including its configuration, environment variables, and volumes.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="space-y-2">
              <Label htmlFor="clone-name">New App Name</Label>
              <Input
                id="clone-name"
                value={cloneName}
                onChange={(e) => setCloneName(e.target.value)}
                placeholder={`${app.name}-copy`}
              />
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setShowCloneDialog(false)}
              disabled={isCloning}
            >
              Cancel
            </Button>
            <Button onClick={handleClone} disabled={isCloning} className="gap-2">
              <Copy className="h-4 w-4" />
              {isCloning ? "Cloning..." : "Clone App"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
