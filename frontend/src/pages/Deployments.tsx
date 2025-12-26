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
import { Skeleton } from "@/components/ui/skeleton";
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
  });

  const isLoading = appsLoading || deploymentsLoading;

  return (
    <div className="space-y-6">
      <h1 className="text-3xl font-bold">Deployments</h1>

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
