import { useState, useMemo } from "react";
import { useOutletContext } from "react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { DeploymentTimeline } from "@/components/deployment-timeline";
import { DeploymentLogs } from "@/components/deployment-logs";
import { api } from "@/lib/api";
import type { App, Deployment, DeploymentStatus, DeploymentLog } from "@/types/api";

const ACTIVE_STATUSES: DeploymentStatus[] = ["pending", "cloning", "building", "starting", "checking"];

function isActiveDeployment(status: DeploymentStatus): boolean {
  return ACTIVE_STATUSES.includes(status);
}

interface OutletContext {
  app: App;
  deployments: Deployment[];
}

export default function AppDeploymentsTab() {
  const { app, deployments } = useOutletContext<OutletContext>();
  const queryClient = useQueryClient();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [showRollbackDialog, setShowRollbackDialog] = useState(false);
  const [showBuildLogsDialog, setShowBuildLogsDialog] = useState(false);
  const [selectedDeploymentId, setSelectedDeploymentId] = useState<string | null>(null);

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

  const canRollback = (deployment: Deployment): boolean => {
    return deployment.status === "stopped" && deployment.container_id !== null;
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
      queryClient.invalidateQueries({ queryKey: ["deployments", app.id] });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Rollback failed");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="space-y-6">
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
        </CardContent>
      </Card>

      {/* Live Deployment Logs */}
      {activeDeployment && (
        <DeploymentLogs
          deploymentId={activeDeployment.id}
          isActive={isActiveDeployment(activeDeployment.status)}
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
