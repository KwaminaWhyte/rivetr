import { useMemo } from "react";
import { useOutletContext } from "react-router";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";

export function meta() {
  return [
    { title: "App Logs - Rivetr" },
    { name: "description", content: "View application runtime and deployment logs" },
  ];
}
import { RuntimeLogs } from "@/components/runtime-logs";
import type { App, Deployment } from "@/types/api";

interface OutletContext {
  app: App;
  deployments: Deployment[];
}

export default function AppLogsTab() {
  const { app, deployments } = useOutletContext<OutletContext>();

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

  return <RuntimeLogs appId={app.id} />;
}
