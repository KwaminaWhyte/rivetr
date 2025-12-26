import { useState, useMemo } from "react";
import { useOutletContext } from "react-router";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { RuntimeLogs } from "@/components/runtime-logs";
import type { App, Deployment } from "@/types/api";

interface OutletContext {
  app: App;
  deployments: Deployment[];
  token: string;
}

export default function AppLogsTab() {
  const { app, deployments, token } = useOutletContext<OutletContext>();
  const [showRuntimeLogs, setShowRuntimeLogs] = useState(true);

  const runningDeployment = useMemo(() => {
    return deployments.find((d) => d.status === "running");
  }, [deployments]);

  if (!runningDeployment) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Runtime Logs</CardTitle>
          <CardDescription>
            View real-time logs from your running container.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <p className="text-muted-foreground text-center py-8">
            No running container. Deploy your application to view runtime logs.
          </p>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Runtime Logs</CardTitle>
              <CardDescription>
                Live streaming logs from your running container.
              </CardDescription>
            </div>
            <Button
              variant="outline"
              onClick={() => setShowRuntimeLogs(!showRuntimeLogs)}
            >
              {showRuntimeLogs ? "Hide Logs" : "Show Logs"}
            </Button>
          </div>
        </CardHeader>
        {showRuntimeLogs && (
          <CardContent>
            <RuntimeLogs appId={app.id} token={token} />
          </CardContent>
        )}
      </Card>
    </div>
  );
}
