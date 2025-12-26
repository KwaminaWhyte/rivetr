import { useOutletContext } from "react-router";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ResourceLimitsCard } from "@/components/resource-limits-card";
import { ResourceMonitor } from "@/components/resource-monitor";
import { EnvironmentBadge } from "@/components/environment-badge";
import type { App, Deployment } from "@/types/api";

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString();
}

interface OutletContext {
  app: App;
  deployments: Deployment[];
}

export default function AppGeneralTab() {
  const { app, deployments } = useOutletContext<OutletContext>();
  const runningDeployment = deployments.find((d) => d.status === "running");

  return (
    <div className="space-y-6">
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
    </div>
  );
}
