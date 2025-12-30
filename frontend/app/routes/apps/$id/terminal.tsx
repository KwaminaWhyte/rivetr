import { useMemo } from "react";
import { useOutletContext } from "react-router";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Terminal as TerminalIcon } from "lucide-react";
import { ContainerTerminal } from "@/components/container-terminal";
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
            {runningDeployment && (
              <span className="text-xs text-muted-foreground">
                Container: {runningDeployment.container_id?.slice(0, 12) || "N/A"}
              </span>
            )}
          </div>
        </CardHeader>
        <CardContent>
          {runningDeployment ? (
            <ContainerTerminal appId={app.id} />
          ) : (
            <div className="bg-gray-900 rounded-lg p-6 text-center">
              <div className="text-gray-400 mb-4">
                <TerminalIcon className="h-16 w-16 mx-auto mb-4 opacity-50" />
                <p className="text-lg">No running container</p>
                <p className="text-sm text-gray-500 mt-2">
                  Deploy your application to access the terminal.
                </p>
              </div>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
