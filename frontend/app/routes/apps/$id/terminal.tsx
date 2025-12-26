import { useMemo } from "react";
import { useOutletContext } from "react-router";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Terminal as TerminalIcon } from "lucide-react";
import type { App, Deployment } from "@/types/api";

interface OutletContext {
  app: App;
  deployments: Deployment[];
}

export default function AppTerminalTab() {
  const { app, deployments } = useOutletContext<OutletContext>();

  const runningDeployment = useMemo(() => {
    return deployments.find((d) => d.status === "running");
  }, [deployments]);

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <TerminalIcon className="h-5 w-5" />
                Container Terminal
              </CardTitle>
              <CardDescription>
                Access a shell inside your running container.
              </CardDescription>
            </div>
            <Badge variant="outline">Coming Soon</Badge>
          </div>
        </CardHeader>
        <CardContent>
          {runningDeployment ? (
            <div className="bg-gray-900 rounded-lg p-6 text-center">
              <div className="text-gray-400 mb-4">
                <TerminalIcon className="h-16 w-16 mx-auto mb-4 opacity-50" />
                <p className="text-lg">Browser-based terminal access</p>
                <p className="text-sm text-gray-500 mt-2">
                  This feature is currently under development. Soon you'll be able to
                  run commands directly inside your container from this interface.
                </p>
              </div>
              <div className="mt-6 text-xs text-gray-600">
                Container ID: {runningDeployment.container_id?.slice(0, 12) || "N/A"}
              </div>
            </div>
          ) : (
            <p className="text-muted-foreground text-center py-8">
              No running container. Deploy your application to access the terminal.
            </p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
