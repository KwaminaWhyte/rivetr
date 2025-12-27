import { useState, useMemo } from "react";
import { Form, useNavigation, useOutletContext } from "react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import type { Route } from "./+types/deployments";
import { toast } from "sonner";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
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
import { AlertCircle, FileText, LayoutList, GitGraph } from "lucide-react";
import { DeploymentTimeline } from "@/components/deployment-timeline";
import { DeploymentLogs } from "@/components/deployment-logs";
import type { App, Deployment, DeploymentStatus, DeploymentLog } from "@/types/api";

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
  replaced: "bg-slate-400",
};

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString();
}

interface OutletContext {
  app: App;
  deployments: Deployment[];
  token: string;
}

export async function action({ request, params }: Route.ActionArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  if (intent === "rollback") {
    const deploymentId = formData.get("deploymentId");
    if (typeof deploymentId !== "string") {
      return { error: "Deployment ID is required" };
    }
    try {
      await api.rollbackDeployment(token, deploymentId);
      return { success: true, action: "rollback" };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Rollback failed" };
    }
  }

  return { error: "Unknown action" };
}

export default function AppDeploymentsTab({ actionData }: Route.ComponentProps) {
  const { app, deployments, token } = useOutletContext<OutletContext>();
  const navigation = useNavigation();
  const queryClient = useQueryClient();
  const [showRollbackDialog, setShowRollbackDialog] = useState(false);
  const [showBuildLogsDialog, setShowBuildLogsDialog] = useState(false);
  const [selectedDeploymentId, setSelectedDeploymentId] = useState<string | null>(null);
  const [deploymentView, setDeploymentView] = useState<"timeline" | "table">("timeline");

  const isSubmitting = navigation.state === "submitting";

  // Fetch build logs for selected deployment
  const { data: buildLogs = [], isLoading: buildLogsLoading } = useQuery<DeploymentLog[]>({
    queryKey: ["deployment-logs", selectedDeploymentId],
    queryFn: async () => {
      const { api } = await import("@/lib/api");
      return api.getDeploymentLogs(selectedDeploymentId!, token);
    },
    enabled: !!selectedDeploymentId && showBuildLogsDialog,
  });

  const hasActiveDeployment = useMemo(() => {
    return deployments.some((d) => isActiveDeployment(d.status));
  }, [deployments]);

  const activeDeployment = useMemo(() => {
    return deployments.find((d) => isActiveDeployment(d.status));
  }, [deployments]);

  const canRollback = (deployment: Deployment): boolean => {
    return deployment.status === "stopped" && deployment.container_id !== null;
  };

  // Handle action results
  if (actionData?.success && actionData.action === "rollback") {
    toast.success("Rollback started");
    setShowRollbackDialog(false);
    setSelectedDeploymentId(null);
    queryClient.invalidateQueries({ queryKey: ["deployments", app.id] });
  }

  if (actionData?.error) {
    toast.error(actionData.error);
  }

  return (
    <div className="space-y-6">
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

      {/* Live Deployment Logs */}
      {activeDeployment && (
        <DeploymentLogs
          deploymentId={activeDeployment.id}
          isActive={isActiveDeployment(activeDeployment.status)}
          token={token}
        />
      )}

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
            <Form method="post">
              <input type="hidden" name="intent" value="rollback" />
              <input type="hidden" name="deploymentId" value={selectedDeploymentId || ""} />
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? "Rolling back..." : "Rollback"}
              </Button>
            </Form>
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
