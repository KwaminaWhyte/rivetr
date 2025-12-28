import { useOutletContext } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { Service } from "@/types/api";
import { ScrollText } from "lucide-react";

interface OutletContext {
  service: Service;
  token: string;
}

export default function ServiceLogsTab() {
  const { service } = useOutletContext<OutletContext>();

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <ScrollText className="h-5 w-5" />
            Service Logs
          </CardTitle>
          <CardDescription>
            View logs from the Docker Compose service
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="bg-muted p-6 rounded-lg text-center text-muted-foreground">
            <ScrollText className="h-12 w-12 mx-auto mb-4 opacity-50" />
            <p className="text-lg font-medium mb-2">Logs Coming Soon</p>
            <p className="text-sm">
              Service logs will be available in a future update.
              For now, you can view logs using <code className="bg-background px-1 rounded">docker compose logs</code> in the terminal.
            </p>
            {service.status === "running" && (
              <p className="text-sm mt-4">
                <code className="bg-background px-2 py-1 rounded text-xs">
                  docker compose -f {`<service-path>`}/docker-compose.yml logs -f
                </code>
              </p>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
