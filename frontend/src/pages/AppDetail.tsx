import { useState, useMemo, useEffect } from "react";
import { useParams, useNavigate } from "react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
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
import { Skeleton } from "@/components/ui/skeleton";
import { api } from "@/lib/api";
import type { App, Deployment, DeploymentStatus, UpdateAppRequest } from "@/types/api";
import { DeploymentLogs } from "@/components/DeploymentLogs";
import { RuntimeLogs } from "@/components/RuntimeLogs";

// Active deployment statuses that require frequent polling
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

export function AppDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [showRollbackDialog, setShowRollbackDialog] = useState(false);
  const [showEditDialog, setShowEditDialog] = useState(false);
  const [selectedDeploymentId, setSelectedDeploymentId] = useState<string | null>(null);
  const [showRuntimeLogs, setShowRuntimeLogs] = useState(false);
  const [editFormData, setEditFormData] = useState<UpdateAppRequest>({});

  const {
    data: app,
    isLoading: appLoading,
    error: appError,
  } = useQuery<App>({
    queryKey: ["app", id],
    queryFn: () => api.getApp(id!),
    enabled: !!id,
  });

  const { data: deployments = [], isLoading: deploymentsLoading } = useQuery<
    Deployment[]
  >({
    queryKey: ["deployments", id],
    queryFn: () => api.getDeployments(id!),
    enabled: !!id,
    // Smart polling: poll every 2s when active, every 30s when idle
    refetchInterval: (query) => {
      const data = query.state.data;
      if (!data || data.length === 0) return 5000;
      const hasActive = data.some((d: Deployment) => isActiveDeployment(d.status));
      return hasActive ? 2000 : 30000;
    },
    refetchIntervalInBackground: false,
  });

  // Check if there are any active deployments (for UI indicators)
  const hasActiveDeployment = useMemo(() => {
    return deployments.some((d) => isActiveDeployment(d.status));
  }, [deployments]);

  // Get the most recent active deployment for log streaming
  const activeDeployment = useMemo(() => {
    return deployments.find((d) => isActiveDeployment(d.status));
  }, [deployments]);

  const deployMutation = useMutation({
    mutationFn: () => api.triggerDeploy(id!),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["deployments", id] });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: () => api.deleteApp(id!),
    onSuccess: () => {
      navigate("/apps");
    },
  });

  const rollbackMutation = useMutation({
    mutationFn: (deploymentId: string) => api.rollbackDeployment(deploymentId),
    onSuccess: () => {
      toast.success("Rollback started");
      queryClient.invalidateQueries({ queryKey: ["deployments", id] });
      setShowRollbackDialog(false);
      setSelectedDeploymentId(null);
    },
    onError: (error: Error) => {
      toast.error(`Rollback failed: ${error.message}`);
    },
  });

  // Check if there's a running deployment (for runtime logs)
  const runningDeployment = useMemo(() => {
    return deployments.find((d) => d.status === "running");
  }, [deployments]);

  // Check if a deployment can be rolled back to
  const canRollback = (deployment: Deployment): boolean => {
    // Can rollback to any previous successful deployment that's not currently running
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
      });
    }
  }, [app]);

  const updateMutation = useMutation({
    mutationFn: (data: UpdateAppRequest) => api.updateApp(id!, data),
    onSuccess: () => {
      toast.success("Application updated");
      queryClient.invalidateQueries({ queryKey: ["app", id] });
      setShowEditDialog(false);
    },
    onError: (error: Error) => {
      toast.error(`Update failed: ${error.message}`);
    },
  });

  const handleEditChange = (field: keyof UpdateAppRequest, value: string | number | undefined) => {
    setEditFormData((prev) => ({ ...prev, [field]: value }));
  };

  if (appLoading) {
    return (
      <div className="space-y-6">
        <Skeleton className="h-10 w-48" />
        <Card>
          <CardContent className="py-8">
            <Skeleton className="h-32 w-full" />
          </CardContent>
        </Card>
      </div>
    );
  }

  if (appError || !app) {
    return (
      <div className="space-y-6">
        <h1 className="text-3xl font-bold">Application Not Found</h1>
        <Card>
          <CardContent className="py-8 text-center text-muted-foreground">
            The application you're looking for doesn't exist or has been
            deleted.
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">{app.name}</h1>
          <p className="text-muted-foreground">{app.git_url}</p>
        </div>
        <div className="flex gap-2">
          <Button
            onClick={() => deployMutation.mutate()}
            disabled={deployMutation.isPending}
          >
            {deployMutation.isPending ? "Deploying..." : "Deploy"}
          </Button>
          {runningDeployment && (
            <Button
              variant="outline"
              onClick={() => setShowRuntimeLogs(!showRuntimeLogs)}
            >
              {showRuntimeLogs ? "Hide Logs" : "View Logs"}
            </Button>
          )}
          <Button
            variant="outline"
            onClick={() => setShowEditDialog(true)}
          >
            Edit
          </Button>
          <Button
            variant="destructive"
            onClick={() => setShowDeleteDialog(true)}
          >
            Delete
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

      <Card>
        <CardHeader>
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
        </CardHeader>
        <CardContent>
          {deploymentsLoading ? (
            <div className="space-y-4">
              {[1, 2, 3].map((i) => (
                <Skeleton key={i} className="h-12 w-full" />
              ))}
            </div>
          ) : deployments.length === 0 ? (
            <p className="text-muted-foreground py-4 text-center">
              No deployments yet. Click Deploy to start your first deployment.
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Status</TableHead>
                  <TableHead>Started</TableHead>
                  <TableHead>Finished</TableHead>
                  <TableHead>Container ID</TableHead>
                  <TableHead className="w-24">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {deployments.map((deploy) => (
                  <TableRow key={deploy.id}>
                    <TableCell>
                      <Badge
                        className={`${statusColors[deploy.status]} text-white`}
                      >
                        {deploy.status}
                      </Badge>
                    </TableCell>
                    <TableCell>{formatDate(deploy.started_at)}</TableCell>
                    <TableCell>
                      {deploy.finished_at ? formatDate(deploy.finished_at) : "-"}
                    </TableCell>
                    <TableCell className="font-mono text-xs">
                      {deploy.container_id?.slice(0, 12) || "-"}
                    </TableCell>
                    <TableCell>
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
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Show logs for active deployment */}
      {activeDeployment && (
        <DeploymentLogs
          deploymentId={activeDeployment.id}
          isActive={isActiveDeployment(activeDeployment.status)}
        />
      )}

      {/* Runtime logs for running container */}
      {showRuntimeLogs && runningDeployment && app && (
        <RuntimeLogs appId={app.id} />
      )}

      {/* Edit app dialog */}
      <Dialog open={showEditDialog} onOpenChange={setShowEditDialog}>
        <DialogContent className="max-w-2xl">
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
            <div className="space-y-2">
              <Label htmlFor="edit-healthcheck">Healthcheck Path</Label>
              <Input
                id="edit-healthcheck"
                placeholder="/health"
                value={editFormData.healthcheck || ""}
                onChange={(e) => handleEditChange("healthcheck", e.target.value || undefined)}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowEditDialog(false)}>
              Cancel
            </Button>
            <Button
              onClick={() => updateMutation.mutate(editFormData)}
              disabled={updateMutation.isPending}
            >
              {updateMutation.isPending ? "Saving..." : "Save Changes"}
            </Button>
          </DialogFooter>
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
            <Button
              onClick={() => {
                if (selectedDeploymentId) {
                  rollbackMutation.mutate(selectedDeploymentId);
                }
              }}
              disabled={rollbackMutation.isPending}
            >
              {rollbackMutation.isPending ? "Rolling back..." : "Rollback"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

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
            <Button
              variant="outline"
              onClick={() => setShowDeleteDialog(false)}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={() => deleteMutation.mutate()}
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending ? "Deleting..." : "Delete"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
