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
import { useBreadcrumb } from "@/lib/breadcrumb-context";
import type { App, AppStatus, Deployment, DeploymentStatus, DeploymentListResponse, Project } from "@/types/api";
import {
  Play,
  Square,
  Circle,
  RotateCw,
  ChevronDown,
  Rocket,
  ExternalLink,
  Upload,
} from "lucide-react";
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

const tabs = [
  { id: "general", label: "General", path: "" },
  { id: "network", label: "Network", path: "/network" },
  { id: "settings", label: "Settings", path: "/settings" },
  { id: "deployments", label: "Deployments", path: "/deployments" },
  { id: "previews", label: "Previews", path: "/previews" },
  { id: "logs", label: "Logs", path: "/logs" },
  { id: "terminal", label: "Terminal", path: "/terminal" },
];

export default function AppDetailLayout() {
  const { id } = useParams();
  const location = useLocation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const { setItems } = useBreadcrumb();

  // Upload deploy state
  const [showUploadDialog, setShowUploadDialog] = useState(false);
  const [isUploading, setIsUploading] = useState(false);
  const [uploadFile, setUploadFile] = useState<File | null>(null);
  const [detectionResult, setDetectionResult] = useState<BuildDetectionResult | null>(null);

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

  // Handle deploy action
  const handleDeploy = async () => {
    if (!id) return;
    setIsSubmitting(true);
    try {
      await api.triggerDeploy(id);
      toast.success("Deployment started");
      queryClient.invalidateQueries({ queryKey: ["deployments", id] });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Deployment failed");
    } finally {
      setIsSubmitting(false);
    }
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
    if (!id) return;
    setIsSubmitting(true);
    try {
      await api.restartApp(id);
      toast.success("Application restarted");
      refetchStatus();
      queryClient.invalidateQueries({ queryKey: ["app", id] });
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to restart app"
      );
    } finally {
      setIsSubmitting(false);
    }
  };

  // Determine active tab from path
  const basePath = `/apps/${id}`;
  const currentPath = location.pathname;
  const activeTab =
    tabs.find((tab) => {
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
            {hasActiveDeployment && (
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
                disabled={isSubmitting || hasActiveDeployment}
                className="gap-2"
                onClick={handleRestart}
              >
                <RotateCw className="h-4 w-4" />
                Restart
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
              <DropdownMenuItem onClick={handleDeploy}>
                <Rocket className="h-4 w-4 mr-2" />
                Redeploy from Git
              </DropdownMenuItem>
              <DropdownMenuItem
                onClick={handleDeploy}
                className="text-muted-foreground"
              >
                <RotateCw className="h-4 w-4 mr-2" />
                Redeploy (clear cache)
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem onClick={() => setShowUploadDialog(true)}>
                <Upload className="h-4 w-4 mr-2" />
                Deploy from ZIP file
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
          {/* Open App button - show if running and has either domain or host_port */}
          {appStatus?.running && (app.domain || appStatus.host_port) && (
            <Button variant="outline" asChild className="gap-2">
              <a
                href={
                  app.domain
                    ? `https://${app.domain}`
                    : `http://${typeof window !== 'undefined' ? window.location.hostname : 'localhost'}:${appStatus.host_port}`
                }
                target="_blank"
                rel="noopener noreferrer"
              >
                <ExternalLink className="h-4 w-4" />
                Open App
                {!app.domain && appStatus.host_port && (
                  <span className="text-xs text-muted-foreground">
                    :{appStatus.host_port}
                  </span>
                )}
              </a>
            </Button>
          )}
        </div>
      </div>

      {/* Tabs Navigation */}
      <Tabs value={activeTab} className="w-full">
        <TabsList className="w-full justify-start">
          {tabs.map((tab) => (
            <TabsTrigger key={tab.id} value={tab.id} asChild>
              <Link to={`${basePath}${tab.path}`}>{tab.label}</Link>
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
    </div>
  );
}
