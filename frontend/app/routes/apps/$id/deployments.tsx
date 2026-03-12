import { useState, useMemo } from "react";
import { useOutletContext, useSearchParams, useNavigate } from "react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";

export function meta() {
  return [
    { title: "Deployments - Rivetr" },
    { name: "description", content: "View and manage application deployments" },
  ];
}
import { toast } from "sonner";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
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
import { apiRequest } from "@/lib/api/core";
import type { App, AppStatus, Deployment, DeploymentStatus, DeploymentLog, DeploymentListResponse } from "@/types/api";
import { ChevronLeft, ChevronRight, CheckCircle, XCircle, Clock, CalendarClock, Shield, Zap, HeartPulse, RefreshCw, GitCompare } from "lucide-react";

interface DeploymentDiff {
  deployment_id: string;
  previous_deployment_id: string | null;
  current_sha: string | null;
  previous_sha: string | null;
  commits_count: number;
  summary: string;
  files_changed: string[];
  commit_messages: string[];
}

const ACTIVE_STATUSES: DeploymentStatus[] = ["pending", "cloning", "building", "starting", "checking"];

function isActiveDeployment(status: DeploymentStatus): boolean {
  return ACTIVE_STATUSES.includes(status);
}

interface OutletContext {
  app: App;
  deployments: Deployment[];
  deploymentsData?: DeploymentListResponse;
}

// Deployment phase config with display metadata
const PHASE_CONFIG: Record<string, { label: string; colorClass: string; bgClass: string; borderClass: string; icon: React.ReactNode }> = {
  stable: {
    label: "Stable",
    colorClass: "text-green-700 dark:text-green-400",
    bgClass: "bg-green-50 dark:bg-green-950/20",
    borderClass: "border-green-200 dark:border-green-800",
    icon: <Shield className="h-4 w-4" />,
  },
  deploying: {
    label: "Deploying",
    colorClass: "text-blue-700 dark:text-blue-400",
    bgClass: "bg-blue-50 dark:bg-blue-950/20",
    borderClass: "border-blue-200 dark:border-blue-800",
    icon: <Zap className="h-4 w-4 animate-pulse" />,
  },
  health_checking: {
    label: "Health Checking",
    colorClass: "text-yellow-700 dark:text-yellow-400",
    bgClass: "bg-yellow-50 dark:bg-yellow-950/20",
    borderClass: "border-yellow-200 dark:border-yellow-800",
    icon: <HeartPulse className="h-4 w-4 animate-pulse" />,
  },
  switching: {
    label: "Switching Traffic",
    colorClass: "text-orange-700 dark:text-orange-400",
    bgClass: "bg-orange-50 dark:bg-orange-950/20",
    borderClass: "border-orange-200 dark:border-orange-800",
    icon: <RefreshCw className="h-4 w-4 animate-spin" />,
  },
};

function formatUptime(seconds: number | null | undefined): string {
  if (!seconds) return "";
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
  return `${Math.floor(seconds / 86400)}d ${Math.floor((seconds % 86400) / 3600)}h`;
}

export default function AppDeploymentsTab() {
  const { app, deploymentsData: parentDeploymentsData } = useOutletContext<OutletContext>();
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [showRollbackDialog, setShowRollbackDialog] = useState(false);
  const [showBuildLogsDialog, setShowBuildLogsDialog] = useState(false);
  const [selectedDeploymentId, setSelectedDeploymentId] = useState<string | null>(null);

  // Fetch app status for deployment phase indicator
  const { data: appStatus } = useQuery<AppStatus>({
    queryKey: ["app-status-detail", app.id],
    queryFn: () => api.getAppStatus(app.id),
    refetchInterval: 5000,
  });

  // Approval workflow state
  const [showRejectDialog, setShowRejectDialog] = useState(false);
  const [rejectDeploymentId, setRejectDeploymentId] = useState<string | null>(null);
  const [rejectionReason, setRejectionReason] = useState("");

  // Diff dialog state
  const [showDiffDialog, setShowDiffDialog] = useState(false);
  const [diffDeploymentId, setDiffDeploymentId] = useState<string | null>(null);
  const [diffData, setDiffData] = useState<DeploymentDiff | null>(null);
  const [diffLoading, setDiffLoading] = useState(false);

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

  // Handle approve deployment
  const handleApprove = async (deploymentId: string) => {
    setIsSubmitting(true);
    try {
      await api.approveDeployment(deploymentId);
      toast.success("Deployment approved and queued");
      queryClient.invalidateQueries({ queryKey: ["deployments", app.id] });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to approve deployment");
    } finally {
      setIsSubmitting(false);
    }
  };

  // Open diff dialog for a deployment
  const handleViewDiff = async (deploymentId: string) => {
    setDiffDeploymentId(deploymentId);
    setDiffData(null);
    setShowDiffDialog(true);
    setDiffLoading(true);
    try {
      const data = await apiRequest<DeploymentDiff>(`/deployments/${deploymentId}/diff`);
      setDiffData(data);
    } catch (error) {
      toast.error("Failed to load deployment diff");
      setShowDiffDialog(false);
    } finally {
      setDiffLoading(false);
    }
  };

  // Handle reject deployment
  const handleReject = async () => {
    if (!rejectDeploymentId) return;
    setIsSubmitting(true);
    try {
      await api.rejectDeployment(rejectDeploymentId, { reason: rejectionReason || undefined });
      toast.success("Deployment rejected");
      setShowRejectDialog(false);
      setRejectDeploymentId(null);
      setRejectionReason("");
      queryClient.invalidateQueries({ queryKey: ["deployments", app.id] });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to reject deployment");
    } finally {
      setIsSubmitting(false);
    }
  };

  // Pagination info
  const total = data?.total ?? 0;
  const totalPages = data?.total_pages ?? 1;
  const currentPage = data?.page ?? page;

  // Pending-approval deployments for this app
  const pendingDeployments = deployments.filter(
    (d) => d.approval_status === "pending"
  );

  const phase = appStatus?.deployment_phase ?? "stable";
  const phaseConfig = PHASE_CONFIG[phase] ?? PHASE_CONFIG.stable;

  return (
    <div className="space-y-6">
      {/* Deployment Phase Status Banner */}
      <div className={`flex items-center justify-between rounded-lg border p-3 ${phaseConfig.bgClass} ${phaseConfig.borderClass}`}>
        <div className={`flex items-center gap-2 font-medium text-sm ${phaseConfig.colorClass}`}>
          {phaseConfig.icon}
          <span>Deployment Status: {phaseConfig.label}</span>
        </div>
        <div className="flex items-center gap-4 text-xs text-muted-foreground">
          {appStatus?.container_id && (
            <span className="font-mono" title="Container ID">
              Container: {appStatus.container_id.slice(0, 12)}
            </span>
          )}
          {appStatus?.uptime_seconds !== null && appStatus?.uptime_seconds !== undefined && (
            <span>
              Uptime: {formatUptime(appStatus.uptime_seconds)}
            </span>
          )}
          {appStatus?.active_deployment_id && (
            <span className="font-mono" title="Active Deployment ID">
              Deploy: {appStatus.active_deployment_id.slice(0, 8)}
            </span>
          )}
        </div>
      </div>

      {/* Pending Approvals Section */}
      {pendingDeployments.length > 0 && (
        <Card className="border-yellow-300 bg-yellow-50 dark:bg-yellow-950/20">
          <CardHeader className="pb-3">
            <CardTitle className="flex items-center gap-2 text-yellow-700 dark:text-yellow-400">
              <Clock className="h-5 w-5" />
              Pending Approvals ({pendingDeployments.length})
            </CardTitle>
            <CardDescription>
              These deployments are waiting for admin approval before they run.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {pendingDeployments.map((d) => (
                <div
                  key={d.id}
                  className="flex items-center justify-between rounded-md border border-yellow-200 bg-white dark:bg-background p-3"
                >
                  <div className="space-y-0.5">
                    <p className="text-sm font-medium font-mono">
                      {d.id.slice(0, 8)}
                      {d.commit_sha && (
                        <span className="ml-2 text-muted-foreground">
                          @ {d.commit_sha.slice(0, 7)}
                        </span>
                      )}
                    </p>
                    {d.commit_message && (
                      <p className="text-xs text-muted-foreground truncate max-w-sm">
                        {d.commit_message.split("\n")[0]}
                      </p>
                    )}
                    {d.scheduled_at && (
                      <p className="text-xs text-muted-foreground flex items-center gap-1">
                        <CalendarClock className="h-3 w-3" />
                        Scheduled for {new Date(d.scheduled_at).toLocaleString()}
                      </p>
                    )}
                  </div>
                  <div className="flex items-center gap-2">
                    <Button
                      size="sm"
                      variant="outline"
                      className="gap-1.5 text-green-600 border-green-300 hover:bg-green-50"
                      disabled={isSubmitting}
                      onClick={() => handleApprove(d.id)}
                    >
                      <CheckCircle className="h-3.5 w-3.5" />
                      Approve
                    </Button>
                    <Button
                      size="sm"
                      variant="outline"
                      className="gap-1.5 text-red-600 border-red-300 hover:bg-red-50"
                      disabled={isSubmitting}
                      onClick={() => {
                        setRejectDeploymentId(d.id);
                        setShowRejectDialog(true);
                      }}
                    >
                      <XCircle className="h-3.5 w-3.5" />
                      Reject
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

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
                  navigate(`/apps/${app.id}/deployments/${deploymentId}`);
                }}
                onRollback={(deploymentId) => {
                  setSelectedDeploymentId(deploymentId);
                  setShowRollbackDialog(true);
                }}
                onViewDiff={handleViewDiff}
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

      {/* Reject deployment dialog */}
      <Dialog
        open={showRejectDialog}
        onOpenChange={(open) => {
          setShowRejectDialog(open);
          if (!open) {
            setRejectDeploymentId(null);
            setRejectionReason("");
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Reject Deployment</DialogTitle>
            <DialogDescription>
              Optionally provide a reason for rejecting this deployment. The
              deployment will be marked as failed.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-2">
            <Label htmlFor="rejection-reason">Reason (optional)</Label>
            <Textarea
              id="rejection-reason"
              placeholder="e.g. Needs more testing, missing feature X..."
              value={rejectionReason}
              onChange={(e) => setRejectionReason(e.target.value)}
              rows={3}
            />
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setShowRejectDialog(false);
                setRejectDeploymentId(null);
                setRejectionReason("");
              }}
              disabled={isSubmitting}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={handleReject}
              disabled={isSubmitting}
              className="gap-2"
            >
              <XCircle className="h-4 w-4" />
              {isSubmitting ? "Rejecting..." : "Reject Deployment"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Build logs dialog */}
      <Dialog open={showBuildLogsDialog} onOpenChange={(open) => {
        setShowBuildLogsDialog(open);
        if (!open) setSelectedDeploymentId(null);
      }}>
        <DialogContent className="w-[95vw] sm:max-w-5xl max-h-[85vh]">
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

      {/* Deployment diff dialog */}
      <Dialog open={showDiffDialog} onOpenChange={(open) => {
        setShowDiffDialog(open);
        if (!open) {
          setDiffDeploymentId(null);
          setDiffData(null);
        }
      }}>
        <DialogContent className="w-[95vw] sm:max-w-4xl max-h-[85vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <GitCompare className="h-5 w-5" />
              Deployment Diff
            </DialogTitle>
            <DialogDescription>
              Changes compared to the previous successful deployment
              {diffDeploymentId && ` (${diffDeploymentId.slice(0, 8)})`}
            </DialogDescription>
          </DialogHeader>

          {diffLoading ? (
            <div className="flex items-center justify-center py-12">
              <div className="flex flex-col items-center gap-3 text-muted-foreground">
                <RefreshCw className="h-8 w-8 animate-spin" />
                <span className="text-sm">Loading diff...</span>
              </div>
            </div>
          ) : diffData ? (
            <div className="space-y-4">
              {/* SHA range */}
              {(diffData.current_sha || diffData.previous_sha) && (
                <div className="rounded-lg border bg-muted/30 p-3 space-y-2">
                  <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Commit Range</p>
                  <div className="flex items-center gap-2 text-sm font-mono">
                    <span className="text-muted-foreground">
                      {diffData.previous_sha ? diffData.previous_sha.slice(0, 7) : "—"}
                    </span>
                    <span className="text-muted-foreground">→</span>
                    <span className="font-medium">
                      {diffData.current_sha ? diffData.current_sha.slice(0, 7) : "—"}
                    </span>
                  </div>
                </div>
              )}

              {/* Summary */}
              <div className="rounded-lg border bg-muted/30 p-3">
                <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide mb-1">Summary</p>
                <p className="text-sm">{diffData.summary}</p>
                {diffData.commits_count > 0 && (
                  <p className="text-xs text-muted-foreground mt-1">
                    {diffData.commits_count} commit{diffData.commits_count !== 1 ? "s" : ""} ahead
                  </p>
                )}
              </div>

              {/* Commit messages */}
              {diffData.commit_messages.length > 0 && (
                <div className="space-y-2">
                  <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                    Commits ({diffData.commit_messages.length})
                  </p>
                  <div className="space-y-1 max-h-48 overflow-y-auto rounded-lg border p-2">
                    {diffData.commit_messages.map((msg, i) => (
                      <div key={i} className="flex items-start gap-2 text-sm py-1">
                        <span className="text-muted-foreground mt-0.5 flex-shrink-0">•</span>
                        <span className="break-words">{msg}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Files changed */}
              {diffData.files_changed.length > 0 && (
                <div className="space-y-2">
                  <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                    Files Changed ({diffData.files_changed.length})
                  </p>
                  <div className="space-y-1 max-h-48 overflow-y-auto rounded-lg border p-2 font-mono">
                    {diffData.files_changed.map((file, i) => (
                      <div key={i} className="text-xs text-muted-foreground py-0.5 truncate" title={file}>
                        {file}
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {diffData.commit_messages.length === 0 && diffData.files_changed.length === 0 && (
                <div className="text-center py-6 text-muted-foreground text-sm">
                  No detailed diff information available for this deployment.
                </div>
              )}
            </div>
          ) : null}

          <DialogFooter>
            <Button variant="outline" onClick={() => setShowDiffDialog(false)}>
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
