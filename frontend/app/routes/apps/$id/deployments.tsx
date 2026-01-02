import { useState, useMemo } from "react";
import { useOutletContext, useSearchParams } from "react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
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
import type { App, Deployment, DeploymentStatus, DeploymentLog, DeploymentListResponse } from "@/types/api";
import { ChevronLeft, ChevronRight } from "lucide-react";

const ACTIVE_STATUSES: DeploymentStatus[] = ["pending", "cloning", "building", "starting", "checking"];

function isActiveDeployment(status: DeploymentStatus): boolean {
  return ACTIVE_STATUSES.includes(status);
}

interface OutletContext {
  app: App;
  deployments: Deployment[];
  deploymentsData?: DeploymentListResponse;
}

export default function AppDeploymentsTab() {
  const { app, deploymentsData: parentDeploymentsData } = useOutletContext<OutletContext>();
  const queryClient = useQueryClient();
  const [searchParams, setSearchParams] = useSearchParams();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [showRollbackDialog, setShowRollbackDialog] = useState(false);
  const [showBuildLogsDialog, setShowBuildLogsDialog] = useState(false);
  const [selectedDeploymentId, setSelectedDeploymentId] = useState<string | null>(null);

  // Read pagination params from URL
  const page = parseInt(searchParams.get("page") || "1");
  const perPage = parseInt(searchParams.get("per_page") || "20");

  // Fetch deployments with pagination - only if page > 1, otherwise use parent data
  const { data: deploymentsData, isLoading } = useQuery<DeploymentListResponse>({
    queryKey: ["deployments", app.id, page, perPage],
    queryFn: () => api.getDeployments(app.id, { page, per_page: perPage }),
    enabled: page > 1, // Only fetch when on page 2+
    refetchInterval: (query) => {
      const data = query.state.data;
      if (!data || data.items.length === 0) return 5000;
      const hasActive = data.items.some((d: Deployment) => isActiveDeployment(d.status));
      return hasActive ? 2000 : 30000;
    },
    refetchIntervalInBackground: false,
  });

  // Use parent data for page 1, otherwise use fetched data
  const data = page === 1 ? parentDeploymentsData : deploymentsData;
  const deployments = data?.items ?? [];

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

  // Update URL with new page
  const goToPage = (newPage: number) => {
    const newParams = new URLSearchParams(searchParams);
    if (newPage > 1) {
      newParams.set("page", String(newPage));
    } else {
      newParams.delete("page");
    }
    setSearchParams(newParams);
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

  // Pagination info
  const total = data?.total ?? 0;
  const totalPages = data?.total_pages ?? 1;
  const currentPage = data?.page ?? page;

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
          {total > 0 && (
            <CardDescription>
              Showing {deployments.length} of {total} deployments
            </CardDescription>
          )}
        </CardHeader>
        <CardContent>
          {isLoading && page > 1 ? (
            <div className="space-y-4">
              {[1, 2, 3].map((i) => (
                <div key={i} className="h-16 bg-muted rounded animate-pulse" />
              ))}
            </div>
          ) : (
            <>
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

              {/* Pagination Controls */}
              {totalPages > 1 && (
                <div className="flex items-center justify-between mt-6 pt-4 border-t">
                  <div className="text-sm text-muted-foreground">
                    Page {currentPage} of {totalPages}
                  </div>
                  <div className="flex items-center gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => goToPage(currentPage - 1)}
                      disabled={currentPage <= 1}
                    >
                      <ChevronLeft className="h-4 w-4" />
                      Previous
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => goToPage(currentPage + 1)}
                      disabled={currentPage >= totalPages}
                    >
                      Next
                      <ChevronRight className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              )}
            </>
          )}
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
