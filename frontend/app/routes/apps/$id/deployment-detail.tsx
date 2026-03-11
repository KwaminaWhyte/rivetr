import { useParams, Link } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { DeploymentLogs } from "@/components/deployment-logs";
import { api } from "@/lib/api";
import {
  ArrowLeft,
  CheckCircle2,
  XCircle,
  Clock,
  Loader2,
  GitCommit,
  Calendar,
  Timer,
} from "lucide-react";
import type { Deployment } from "@/types/api";

const STATUS_CONFIG: Record<
  string,
  { label: string; icon: React.ReactNode; variant: "default" | "destructive" | "secondary" | "outline" }
> = {
  pending: {
    label: "Pending",
    icon: <Clock className="h-4 w-4" />,
    variant: "secondary",
  },
  cloning: {
    label: "Cloning",
    icon: <Loader2 className="h-4 w-4 animate-spin" />,
    variant: "secondary",
  },
  building: {
    label: "Building",
    icon: <Loader2 className="h-4 w-4 animate-spin" />,
    variant: "secondary",
  },
  starting: {
    label: "Starting",
    icon: <Loader2 className="h-4 w-4 animate-spin" />,
    variant: "secondary",
  },
  checking: {
    label: "Health Checking",
    icon: <Loader2 className="h-4 w-4 animate-spin" />,
    variant: "secondary",
  },
  success: {
    label: "Success",
    icon: <CheckCircle2 className="h-4 w-4" />,
    variant: "default",
  },
  failed: {
    label: "Failed",
    icon: <XCircle className="h-4 w-4" />,
    variant: "destructive",
  },
};

const ACTIVE_STATUSES = ["pending", "cloning", "building", "starting", "checking"];

function formatDateTime(dateStr: string | null | undefined): string {
  if (!dateStr) return "—";
  return new Date(dateStr).toLocaleString();
}

function durationSeconds(start: string | null | undefined, end: string | null | undefined): string {
  if (!start || !end) return "—";
  const diff = Math.round((new Date(end).getTime() - new Date(start).getTime()) / 1000);
  if (diff < 60) return `${diff}s`;
  return `${Math.floor(diff / 60)}m ${diff % 60}s`;
}

export default function DeploymentDetailPage() {
  const { id: appId, deploymentId } = useParams<{ id: string; deploymentId: string }>();

  const { data: deployment, isLoading } = useQuery<Deployment>({
    queryKey: ["deployment", deploymentId],
    queryFn: () => api.getDeployment(deploymentId!),
    enabled: !!deploymentId,
    // Poll while active so status updates live
    refetchInterval: (query) => {
      const d = query.state.data;
      if (!d) return 2000;
      return ACTIVE_STATUSES.includes(d.status) ? 2000 : false;
    },
  });

  const isActive = deployment ? ACTIVE_STATUSES.includes(deployment.status) : false;
  const statusCfg = deployment ? (STATUS_CONFIG[deployment.status] ?? STATUS_CONFIG.pending) : null;

  return (
    <div className="space-y-6 p-6">
      {/* Back navigation */}
      <div className="flex items-center gap-3">
        <Button variant="ghost" size="sm" asChild>
          <Link to={`/apps/${appId}/deployments`}>
            <ArrowLeft className="h-4 w-4 mr-1" />
            Back to Deployments
          </Link>
        </Button>
      </div>

      {isLoading ? (
        <div className="flex items-center justify-center py-16 text-muted-foreground">
          <Loader2 className="h-6 w-6 animate-spin mr-2" />
          Loading deployment…
        </div>
      ) : deployment ? (
        <>
          {/* Header card */}
          <Card>
            <CardHeader className="pb-3">
              <div className="flex items-start justify-between gap-4">
                <div className="space-y-1">
                  <CardTitle className="text-lg font-semibold">
                    Deployment{" "}
                    <span className="font-mono text-muted-foreground">
                      {deployment.id.slice(0, 8)}
                    </span>
                  </CardTitle>
                  {deployment.commit_message && (
                    <p className="text-sm text-muted-foreground line-clamp-2">
                      {deployment.commit_message}
                    </p>
                  )}
                </div>

                {statusCfg && (
                  <Badge
                    variant={statusCfg.variant}
                    className="flex items-center gap-1.5 shrink-0"
                  >
                    {statusCfg.icon}
                    {statusCfg.label}
                  </Badge>
                )}
              </div>
            </CardHeader>

            <CardContent className="space-y-4">
              <div className="grid grid-cols-2 gap-x-8 gap-y-3 sm:grid-cols-4 text-sm">
                {deployment.commit_sha && (
                  <div className="space-y-0.5">
                    <p className="text-xs text-muted-foreground flex items-center gap-1">
                      <GitCommit className="h-3 w-3" /> Commit
                    </p>
                    <p className="font-mono">{deployment.commit_sha.slice(0, 7)}</p>
                  </div>
                )}

                <div className="space-y-0.5">
                  <p className="text-xs text-muted-foreground flex items-center gap-1">
                    <Calendar className="h-3 w-3" /> Started
                  </p>
                  <p>{formatDateTime(deployment.started_at)}</p>
                </div>

                {deployment.finished_at && (
                  <div className="space-y-0.5">
                    <p className="text-xs text-muted-foreground flex items-center gap-1">
                      <Calendar className="h-3 w-3" /> Finished
                    </p>
                    <p>{formatDateTime(deployment.finished_at)}</p>
                  </div>
                )}

                <div className="space-y-0.5">
                  <p className="text-xs text-muted-foreground flex items-center gap-1">
                    <Timer className="h-3 w-3" /> Duration
                  </p>
                  <p>{durationSeconds(deployment.started_at, deployment.finished_at)}</p>
                </div>
              </div>

              {deployment.error_message && (
                <>
                  <Separator />
                  <div className="rounded-md bg-destructive/10 border border-destructive/20 p-3 space-y-1">
                    <p className="text-xs font-medium text-destructive">Error</p>
                    <pre className="text-xs text-destructive/80 whitespace-pre-wrap break-words font-mono">
                      {deployment.error_message}
                    </pre>
                  </div>
                </>
              )}
            </CardContent>
          </Card>

          {/* Live / historical logs */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide">
                Build Logs
              </h2>
              {isActive && (
                <span className="flex items-center gap-1.5 text-xs text-blue-600">
                  <span className="relative flex h-2 w-2">
                    <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-blue-400 opacity-75"></span>
                    <span className="relative inline-flex h-2 w-2 rounded-full bg-blue-500"></span>
                  </span>
                  Live
                </span>
              )}
            </div>

            <DeploymentLogs deploymentId={deployment.id} isActive={isActive} />
          </div>
        </>
      ) : (
        <div className="text-center py-16 text-muted-foreground">
          Deployment not found.
        </div>
      )}
    </div>
  );
}
