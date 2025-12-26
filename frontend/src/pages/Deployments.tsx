import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Skeleton } from "@/components/ui/skeleton";
import { AlertCircle } from "lucide-react";
import { api } from "@/lib/api";
import type { App, Deployment, DeploymentStatus } from "@/types/api";
import { Link } from "react-router";

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

// Active deployment statuses that require frequent polling
const ACTIVE_STATUSES: DeploymentStatus[] = ["pending", "cloning", "building", "starting", "checking"];

function isActiveDeployment(status: DeploymentStatus): boolean {
  return ACTIVE_STATUSES.includes(status);
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString();
}

export function DeploymentsPage() {
  const { data: apps = [], isLoading: appsLoading } = useQuery<App[]>({
    queryKey: ["apps"],
    queryFn: () => api.getApps(),
  });

  // Fetch deployments for all apps
  const { data: allDeployments = [], isLoading: deploymentsLoading } = useQuery<
    (Deployment & { appName: string })[]
  >({
    queryKey: ["all-deployments", apps.map((a) => a.id)],
    queryFn: async () => {
      const deploymentPromises = apps.map(async (app) => {
        const deployments = await api.getDeployments(app.id);
        return deployments.map((d) => ({ ...d, appName: app.name }));
      });
      const results = await Promise.all(deploymentPromises);
      return results
        .flat()
        .sort(
          (a, b) =>
            new Date(b.started_at).getTime() - new Date(a.started_at).getTime()
        );
    },
    enabled: apps.length > 0,
    // Smart polling: poll every 2s when active, every 30s when idle
    refetchInterval: (query) => {
      const data = query.state.data;
      if (!data || data.length === 0) return 10000;
      const hasActive = data.some((d) => isActiveDeployment(d.status));
      return hasActive ? 2000 : 30000;
    },
    refetchIntervalInBackground: false,
  });

  // Check if there are any active deployments (for UI indicators)
  const hasActiveDeployment = useMemo(() => {
    return allDeployments.some((d) => isActiveDeployment(d.status));
  }, [allDeployments]);

  const isLoading = appsLoading || deploymentsLoading;

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-3">
        <h1 className="text-3xl font-bold">Deployments</h1>
        {hasActiveDeployment && (
          <span className="flex items-center gap-1.5 text-sm font-normal text-blue-600">
            <span className="relative flex h-2 w-2">
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-blue-400 opacity-75"></span>
              <span className="relative inline-flex h-2 w-2 rounded-full bg-blue-500"></span>
            </span>
            In Progress
          </span>
        )}
      </div>

      <Card>
        <CardHeader>
          <CardTitle>All Deployments</CardTitle>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-4">
              {[1, 2, 3].map((i) => (
                <Skeleton key={i} className="h-12 w-full" />
              ))}
            </div>
          ) : allDeployments.length === 0 ? (
            <p className="text-muted-foreground py-4 text-center">
              No deployments yet. Deploy an app to see activity here.
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Application</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Started</TableHead>
                  <TableHead>Finished</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {allDeployments.slice(0, 20).map((deploy) => (
                  <TableRow key={deploy.id}>
                    <TableCell>
                      <Link
                        to={`/apps/${deploy.app_id}`}
                        className="font-medium hover:underline"
                      >
                        {deploy.appName}
                      </Link>
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <Badge
                          className={`${statusColors[deploy.status]} text-white`}
                        >
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
                    <TableCell>{formatDate(deploy.started_at)}</TableCell>
                    <TableCell>
                      {deploy.finished_at ? formatDate(deploy.finished_at) : "-"}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
