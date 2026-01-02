import { useState, useMemo, useEffect } from "react";
import { Link, useParams, useNavigate } from "react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { AlertCircle, FileText, LayoutList, GitGraph } from "lucide-react";
import { api } from "@/lib/api";
import type { App, AppEnvironment, Deployment, DeploymentStatus, DeploymentLog, DeploymentListResponse, UpdateAppRequest } from "@/types/api";
import { DeploymentLogs } from "@/components/deployment-logs";
import { ResourceLimitsCard } from "@/components/resource-limits-card";
import { ResourceMonitor } from "@/components/resource-monitor";
import { DeploymentTimeline } from "@/components/deployment-timeline";
import { EnvVarsTab } from "@/components/env-vars-tab";
import { EnvironmentBadge } from "@/components/environment-badge";

const ENVIRONMENT_OPTIONS: { value: AppEnvironment; label: string }[] = [
  { value: "development", label: "Development" },
  { value: "staging", label: "Staging" },
  { value: "production", label: "Production" },
];

const ACTIVE_STATUSES: DeploymentStatus[] = ["pending", "cloning", "building", "starting", "checking"];

function isActiveDeployment(status: DeploymentStatus): boolean {
  return ACTIVE_STATUSES.includes(status);
}

const statusColors: Record<DeploymentStatus, string> = {
  pending: "bg-yellow-500",
  cloning: "bg-blue-500",
  building: "bg-blue-500",
  starting: "bg-blue-500",
  checking: "bg-blue-500",
  running: "bg-green-500",
  failed: "bg-red-500",
  stopped: "bg-gray-500",
};

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString();
}

export default function AppDetailPage() {
  const { id } = useParams();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [showRollbackDialog, setShowRollbackDialog] = useState(false);
  const [showEditDialog, setShowEditDialog] = useState(false);
  const [showBuildLogsDialog, setShowBuildLogsDialog] = useState(false);
  const [selectedDeploymentId, setSelectedDeploymentId] = useState<string | null>(null);
  const [editFormData, setEditFormData] = useState<UpdateAppRequest>({});
  const [deploymentView, setDeploymentView] = useState<"timeline" | "table">("timeline");

  // Use React Query to fetch data client-side
  const { data: app, isLoading: appLoading } = useQuery<App>({
    queryKey: ["app", id],
    queryFn: () => api.getApp(id!),
    enabled: !!id,
  });

  const { data: deploymentsData } = useQuery<DeploymentListResponse>({
    queryKey: ["deployments", id],
    queryFn: () => api.getDeployments(id!, { per_page: 20 }),
    enabled: !!id,
    refetchInterval: (query) => {
      const data = query.state.data;
      if (!data || data.items.length === 0) return 5000;
      const hasActive = data.items.some((d: Deployment) => isActiveDeployment(d.status));
      return hasActive ? 2000 : 30000;
    },
    refetchIntervalInBackground: false,
  });

  const deployments = deploymentsData?.items ?? [];

  // Fetch build logs for selected deployment
  const { data: buildLogs = [], isLoading: buildLogsLoading } = useQuery<DeploymentLog[]>({
    queryKey: ["deployment-logs", selectedDeploymentId],
    queryFn: () => api.getDeploymentLogs(selectedDeploymentId!),
    enabled: !!selectedDeploymentId && showBuildLogsDialog,
  });

  const hasActiveDeployment = useMemo(() => {
    return deployments.some((d) => isActiveDeployment(d.status));
  }, [deployments]);

  const activeDeployment = useMemo(() => {
    return deployments.find((d) => isActiveDeployment(d.status));
  }, [deployments]);

  const runningDeployment = useMemo(() => {
    return deployments.find((d) => d.status === "running");
  }, [deployments]);

  const canRollback = (deployment: Deployment): boolean => {
    return deployment.status === "stopped" && deployment.container_id !== null;
  };

  // Populate edit form when app loads
  useEffect(() => {
    if (app) {
      setEditFormData({
        name: app.name,
        git_url: app.git_url,
        branch: app.branch,
        dockerfile: app.dockerfile,
        domain: app.domain || undefined,
        port: app.port,
        healthcheck: app.healthcheck || undefined,
        environment: app.environment,
      });
    }
  }, [app]);

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

  // Handle rollback action
  const handleRollback = async () => {
    if (!selectedDeploymentId) return;
    setIsSubmitting(true);
    try {
      await api.rollbackDeployment(selectedDeploymentId);
      toast.success("Rollback started");
      setShowRollbackDialog(false);
      setSelectedDeploymentId(null);
      queryClient.invalidateQueries({ queryKey: ["deployments", id] });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Rollback failed");
    } finally {
      setIsSubmitting(false);
    }
  };

  // Handle update action
  const handleUpdate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!id) return;
    setIsSubmitting(true);
    try {
      await api.updateApp(id, editFormData);
      toast.success("Application updated");
      setShowEditDialog(false);
      queryClient.invalidateQueries({ queryKey: ["app", id] });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Update failed");
    } finally {
      setIsSubmitting(false);
    }
  };


  const handleEditChange = (field: keyof UpdateAppRequest, value: string | number | undefined) => {
    setEditFormData((prev) => ({ ...prev, [field]: value }));
  };

  if (appLoading) {
    return (
      <div className="space-y-6">
        <div className="animate-pulse">
          <div className="h-8 bg-muted rounded w-48 mb-4"></div>
          <div className="h-4 bg-muted rounded w-96"></div>
        </div>
      </div>
    );
  }

  if (!app) {
    return (
      <div className="space-y-6">
        <h1 className="text-3xl font-bold">Application Not Found</h1>
        <Card>
          <CardContent className="py-8 text-center text-muted-foreground">
            The application you're looking for doesn't exist or has been deleted.
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center gap-3">
            <h1 className="text-3xl font-bold">{app.name}</h1>
            <EnvironmentBadge environment={app.environment} />
          </div>
          <p className="text-muted-foreground">{app.git_url}</p>
        </div>
        <div className="flex gap-2">
          <Button onClick={handleDeploy} disabled={isSubmitting}>
            {isSubmitting ? "Deploying..." : "Deploy"}
          </Button>
          <Button variant="outline" onClick={() => setShowEditDialog(true)}>
            Edit
          </Button>
        </div>
      </div>

      <div className="grid gap-6 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Configuration</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <div className="text-sm text-muted-foreground">Environment</div>
                <div className="font-medium mt-1">
                  <EnvironmentBadge environment={app.environment} />
                </div>
              </div>
              <div>
                <div className="text-sm text-muted-foreground">Branch</div>
                <div className="font-medium">{app.branch}</div>
              </div>
              <div>
                <div className="text-sm text-muted-foreground">Port</div>
                <div className="font-medium">{app.port}</div>
              </div>
              <div>
                <div className="text-sm text-muted-foreground">Dockerfile</div>
                <div className="font-medium">{app.dockerfile}</div>
              </div>
              <div>
                <div className="text-sm text-muted-foreground">Domain</div>
                <div className="font-medium">{app.domain || "-"}</div>
              </div>
              <div>
                <div className="text-sm text-muted-foreground">Healthcheck</div>
                <div className="font-medium">{app.healthcheck || "-"}</div>
              </div>
              <div>
                <div className="text-sm text-muted-foreground">CPU Limit</div>
                <div className="font-medium">{app.cpu_limit ? `${app.cpu_limit} cores` : "-"}</div>
              </div>
              <div>
                <div className="text-sm text-muted-foreground">Memory Limit</div>
                <div className="font-medium">{app.memory_limit || "-"}</div>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Details</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div>
              <div className="text-sm text-muted-foreground">App ID</div>
              <div className="font-mono text-sm">{app.id}</div>
            </div>
            <div>
              <div className="text-sm text-muted-foreground">Created</div>
              <div className="font-medium">{formatDate(app.created_at)}</div>
            </div>
            <div>
              <div className="text-sm text-muted-foreground">Updated</div>
              <div className="font-medium">{formatDate(app.updated_at)}</div>
            </div>
          </CardContent>
        </Card>
      </div>

      <ResourceLimitsCard app={app} />

      {runningDeployment && <ResourceMonitor appId={app.id} />}

      <EnvVarsTab appId={app.id} />

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle className="flex items-center gap-2">
              Deployments
              {hasActiveDeployment && (
                <span className="flex items-center gap-1.5 text-sm font-normal text-blue-600">
                  <span className="relative flex h-2 w-2">
                    <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-blue-400 opacity-75"></span>
                    <span className="relative inline-flex h-2 w-2 rounded-full bg-blue-500"></span>
                  </span>
                  In Progress
                </span>
              )}
            </CardTitle>
            <div className="flex items-center gap-1 bg-muted rounded-lg p-1">
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant={deploymentView === "timeline" ? "secondary" : "ghost"}
                      size="sm"
                      className="h-8 px-3"
                      onClick={() => setDeploymentView("timeline")}
                    >
                      <GitGraph className="h-4 w-4" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>Timeline view</TooltipContent>
                </Tooltip>
              </TooltipProvider>
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant={deploymentView === "table" ? "secondary" : "ghost"}
                      size="sm"
                      className="h-8 px-3"
                      onClick={() => setDeploymentView("table")}
                    >
                      <LayoutList className="h-4 w-4" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>Table view</TooltipContent>
                </Tooltip>
              </TooltipProvider>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          {deploymentView === "timeline" ? (
            <DeploymentTimeline
              deployments={deployments}
              branch={app.branch}
              onViewLogs={(deploymentId) => {
                setSelectedDeploymentId(deploymentId);
                setShowBuildLogsDialog(true);
              }}
              onRollback={(deploymentId) => {
                setSelectedDeploymentId(deploymentId);
                setShowRollbackDialog(true);
              }}
              canRollback={canRollback}
            />
          ) : deployments.length === 0 ? (
            <p className="text-muted-foreground py-4 text-center">
              No deployments yet. Click Deploy to start your first deployment.
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Status</TableHead>
                  <TableHead>Commit</TableHead>
                  <TableHead>Started</TableHead>
                  <TableHead>Duration</TableHead>
                  <TableHead>Container ID</TableHead>
                  <TableHead className="w-24">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {deployments.map((deploy) => (
                  <TableRow key={deploy.id}>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <Badge className={`${statusColors[deploy.status]} text-white`}>
                          {deploy.status}
                        </Badge>
                        {deploy.status === "failed" && deploy.error_message && (
                          <TooltipProvider>
                            <Tooltip>
                              <TooltipTrigger asChild>
                                <AlertCircle className="h-4 w-4 text-red-500 cursor-help" />
                              </TooltipTrigger>
                              <TooltipContent side="right" className="max-w-sm">
                                <p className="font-medium text-red-500 mb-1">Error</p>
                                <p className="text-sm whitespace-pre-wrap">{deploy.error_message}</p>
                              </TooltipContent>
                            </Tooltip>
                          </TooltipProvider>
                        )}
                      </div>
                    </TableCell>
                    <TableCell>
                      {deploy.commit_sha ? (
                        <TooltipProvider>
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <span className="font-mono text-xs cursor-help">
                                {deploy.commit_sha.slice(0, 7)}
                              </span>
                            </TooltipTrigger>
                            <TooltipContent side="top" className="max-w-sm">
                              <p className="text-sm">{deploy.commit_message || "No commit message"}</p>
                            </TooltipContent>
                          </Tooltip>
                        </TooltipProvider>
                      ) : (
                        "-"
                      )}
                    </TableCell>
                    <TableCell>{formatDate(deploy.started_at)}</TableCell>
                    <TableCell>
                      {(() => {
                        const start = new Date(deploy.started_at).getTime();
                        const end = deploy.finished_at ? new Date(deploy.finished_at).getTime() : Date.now();
                        const durationMs = end - start;
                        const seconds = Math.floor(durationMs / 1000);
                        const minutes = Math.floor(seconds / 60);
                        if (minutes > 0) return `${minutes}m ${seconds % 60}s`;
                        return `${seconds}s`;
                      })()}
                    </TableCell>
                    <TableCell className="font-mono text-xs">
                      {deploy.container_id?.slice(0, 12) || "-"}
                    </TableCell>
                    <TableCell>
                      <div className="flex gap-1">
                        <TooltipProvider>
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => {
                                  setSelectedDeploymentId(deploy.id);
                                  setShowBuildLogsDialog(true);
                                }}
                              >
                                <FileText className="h-4 w-4" />
                              </Button>
                            </TooltipTrigger>
                            <TooltipContent>View build logs</TooltipContent>
                          </Tooltip>
                        </TooltipProvider>
                        {canRollback(deploy) && (
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => {
                              setSelectedDeploymentId(deploy.id);
                              setShowRollbackDialog(true);
                            }}
                          >
                            Rollback
                          </Button>
                        )}
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {activeDeployment && (
        <DeploymentLogs
          deploymentId={activeDeployment.id}
          isActive={isActiveDeployment(activeDeployment.status)}
        />
      )}

      {/* Edit app dialog */}
      <Dialog open={showEditDialog} onOpenChange={setShowEditDialog}>
        <DialogContent className="max-w-2xl">
          <form onSubmit={handleUpdate}>
            <DialogHeader>
              <DialogTitle>Edit Application</DialogTitle>
              <DialogDescription>
                Update your application settings. Changes will take effect on the next deployment.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="edit-name">Name</Label>
                  <Input
                    id="edit-name"
                    value={editFormData.name || ""}
                    onChange={(e) => handleEditChange("name", e.target.value)}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="edit-git_url">Git URL</Label>
                  <Input
                    id="edit-git_url"
                    value={editFormData.git_url || ""}
                    onChange={(e) => handleEditChange("git_url", e.target.value)}
                  />
                </div>
              </div>
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="edit-branch">Branch</Label>
                  <Input
                    id="edit-branch"
                    value={editFormData.branch || ""}
                    onChange={(e) => handleEditChange("branch", e.target.value)}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="edit-port">Port</Label>
                  <Input
                    id="edit-port"
                    type="number"
                    value={editFormData.port || ""}
                    onChange={(e) => handleEditChange("port", parseInt(e.target.value) || undefined)}
                  />
                </div>
              </div>
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="edit-dockerfile">Dockerfile</Label>
                  <Input
                    id="edit-dockerfile"
                    value={editFormData.dockerfile || ""}
                    onChange={(e) => handleEditChange("dockerfile", e.target.value)}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="edit-domain">Domain</Label>
                  <Input
                    id="edit-domain"
                    placeholder="app.example.com"
                    value={editFormData.domain || ""}
                    onChange={(e) => handleEditChange("domain", e.target.value || undefined)}
                  />
                </div>
              </div>
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="edit-healthcheck">Healthcheck Path</Label>
                  <Input
                    id="edit-healthcheck"
                    placeholder="/health"
                    value={editFormData.healthcheck || ""}
                    onChange={(e) => handleEditChange("healthcheck", e.target.value || undefined)}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="edit-environment">Environment</Label>
                  <Select
                    value={editFormData.environment || "development"}
                    onValueChange={(value) => handleEditChange("environment", value)}
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
                </div>
              </div>
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setShowEditDialog(false)}>
                Cancel
              </Button>
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? "Saving..." : "Save Changes"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Rollback confirmation dialog */}
      <Dialog open={showRollbackDialog} onOpenChange={setShowRollbackDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Rollback Deployment</DialogTitle>
            <DialogDescription>
              This will start a new deployment using the image from the selected
              previous deployment. The current running container will be replaced.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setShowRollbackDialog(false);
                setSelectedDeploymentId(null);
              }}
            >
              Cancel
            </Button>
            <Button onClick={handleRollback} disabled={isSubmitting}>
              {isSubmitting ? "Rolling back..." : "Rollback"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Build logs dialog */}
      <Dialog open={showBuildLogsDialog} onOpenChange={(open) => {
        setShowBuildLogsDialog(open);
        if (!open) setSelectedDeploymentId(null);
      }}>
        <DialogContent className="max-w-4xl max-h-[80vh]">
          <DialogHeader>
            <DialogTitle>Build Logs</DialogTitle>
            <DialogDescription>
              Deployment logs for {selectedDeploymentId?.slice(0, 8)}...
            </DialogDescription>
          </DialogHeader>
          <div className="bg-gray-900 rounded-lg p-4 max-h-[50vh] overflow-y-auto font-mono text-sm">
            {buildLogsLoading ? (
              <div className="text-gray-500 text-center py-4">Loading logs...</div>
            ) : buildLogs.length === 0 ? (
              <div className="text-gray-500 text-center py-4">No logs available</div>
            ) : (
              <div className="space-y-1">
                {buildLogs.map((log) => (
                  <div key={log.id} className="flex gap-2 text-gray-300">
                    <span className="text-gray-500 flex-shrink-0">
                      {new Date(log.timestamp).toLocaleTimeString()}
                    </span>
                    <span
                      className={`px-1.5 py-0.5 rounded text-xs text-white flex-shrink-0 ${
                        log.level === "error" ? "bg-red-500" :
                        log.level === "warn" ? "bg-yellow-500" :
                        log.level === "info" ? "bg-blue-500" : "bg-gray-500"
                      }`}
                    >
                      {log.level.toUpperCase()}
                    </span>
                    <span className="whitespace-pre-wrap break-all">
                      {log.message}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => {
              setShowBuildLogsDialog(false);
              setSelectedDeploymentId(null);
            }}>
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
