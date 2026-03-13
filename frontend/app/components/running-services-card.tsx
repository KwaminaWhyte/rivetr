import { useQuery } from "@tanstack/react-query";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { api } from "@/lib/api";
import { useTeamContext } from "@/lib/team-context";
import { Cpu, HardDrive, Activity, Database, Server } from "lucide-react";
import { Link } from "react-router";
import type { App, ManagedDatabase, Service } from "@/types/api";

interface RunningServiceInfo {
  id: string;
  name: string;
  type: "app" | "database" | "service";
  status: string;
  cpu_percent: number;
  memory_usage: number;
  memory_limit: number;
  url: string;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

export function RunningServicesCard() {
  const { currentTeamId } = useTeamContext();

  const { data: apps = [] } = useQuery<App[]>({
    queryKey: ["apps", currentTeamId],
    queryFn: () => api.getApps(currentTeamId ? { teamId: currentTeamId } : undefined),
    refetchInterval: 30000,
  });

  const { data: databases = [] } = useQuery<ManagedDatabase[]>({
    queryKey: ["databases", currentTeamId],
    queryFn: () => api.getDatabases(currentTeamId ? { teamId: currentTeamId } : {}),
    refetchInterval: 30000,
  });

  const { data: services = [] } = useQuery<Service[]>({
    queryKey: ["services", currentTeamId],
    queryFn: () => api.getServices(currentTeamId ? { teamId: currentTeamId } : undefined),
    refetchInterval: 30000,
  });

  // Fetch stats for all apps. The backend returns zeroed stats (not 404) for
  // non-running apps, so we filter out entries with zero memory usage to show
  // only apps that are actually consuming resources.
  const appStatsQuery = useQuery({
    queryKey: ["app-stats", apps.map((a) => a.id)],
    queryFn: async () => {
      const results = await Promise.all(
        apps.map(async (app) => {
          try {
            const stats = await api.getAppStats(app.id);
            // Skip apps that are not running (backend returns zeroed stats)
            if (!stats || (stats.cpu_percent === 0 && stats.memory_usage === 0)) {
              return null;
            }
            return {
              id: app.id,
              name: app.name,
              type: "app" as const,
              status: "running",
              cpu_percent: stats.cpu_percent,
              memory_usage: stats.memory_usage,
              memory_limit: stats.memory_limit,
              url: `/apps/${app.id}`,
            };
          } catch {
            // Unexpected error — omit from list
            return null;
          }
        })
      );
      return results.filter((r): r is NonNullable<typeof r> => r !== null);
    },
    enabled: apps.length > 0,
    refetchInterval: 15000,
  });

  // Fetch stats for all running databases
  const runningDatabases = databases.filter((db) => db.status === "running");
  const dbStatsQuery = useQuery({
    queryKey: ["db-stats", runningDatabases.map((d) => d.id)],
    queryFn: async () => {
      const results = await Promise.all(
        runningDatabases.map(async (db) => {
          try {
            const stats = await api.getDatabaseStats(db.id);
            return {
              id: db.id,
              name: db.name,
              type: "database" as const,
              status: "running",
              cpu_percent: stats?.cpu_percent ?? 0,
              memory_usage: stats?.memory_usage ?? 0,
              memory_limit: stats?.memory_limit ?? 0,
              url: `/databases/${db.id}`,
            };
          } catch {
            return {
              id: db.id,
              name: db.name,
              type: "database" as const,
              status: "running",
              cpu_percent: 0,
              memory_usage: 0,
              memory_limit: 0,
              url: `/databases/${db.id}`,
            };
          }
        })
      );
      return results;
    },
    enabled: runningDatabases.length > 0,
    refetchInterval: 15000,
  });

  const runningServices = services.filter((s) => s.status === "running");
  const serviceInfos: RunningServiceInfo[] = runningServices.map((service) => ({
    id: service.id,
    name: service.name,
    type: "service" as const,
    status: "running",
    cpu_percent: 0,
    memory_usage: 0,
    memory_limit: 0,
    url: `/services/${service.id}`,
  }));

  const allRunning: RunningServiceInfo[] = [
    ...(appStatsQuery.data ?? []),
    ...(dbStatsQuery.data ?? []),
    ...serviceInfos,
  ].sort((a, b) => b.memory_usage - a.memory_usage);

  const getIcon = (type: string) => {
    switch (type) {
      case "app":
        return <Activity className="h-4 w-4 text-blue-500 shrink-0" />;
      case "database":
        return <Database className="h-4 w-4 text-purple-500 shrink-0" />;
      case "service":
        return <Server className="h-4 w-4 text-green-500 shrink-0" />;
      default:
        return <Activity className="h-4 w-4 shrink-0" />;
    }
  };

  const badgeClass = {
    app: "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400",
    database: "bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400",
    service: "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400",
  };

  return (
    <Card className="flex flex-col">
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium">Running Services</CardTitle>
          {allRunning.length > 0 && (
            <span className="text-xs text-muted-foreground">{allRunning.length} active</span>
          )}
        </div>
      </CardHeader>
      <CardContent className="flex-1">
        {allRunning.length === 0 ? (
          <div className="text-sm text-muted-foreground text-center py-4">
            No running services
          </div>
        ) : (
          <div className="space-y-1 max-h-70 overflow-y-auto pr-1">
            {allRunning.map((service) => (
              <Link
                key={`${service.type}-${service.id}`}
                to={service.url}
                className="block"
              >
                <div className="flex items-center justify-between px-2 py-2 rounded-lg hover:bg-muted/50 transition-colors">
                  <div className="flex items-center gap-2 min-w-0">
                    {getIcon(service.type)}
                    <div className="min-w-0">
                      <div className="font-medium text-sm truncate">{service.name}</div>
                      <Badge
                        variant="secondary"
                        className={`text-xs ${badgeClass[service.type]}`}
                      >
                        {service.type}
                      </Badge>
                    </div>
                  </div>
                  {service.type === "service" ? (
                    // Docker Compose services don't have per-container stats available
                    <span className="text-xs text-muted-foreground shrink-0">running</span>
                  ) : (
                    <div className="flex items-center gap-3 text-xs text-muted-foreground shrink-0">
                      <div className="flex items-center gap-1" title="CPU">
                        <Cpu className="h-3 w-3" />
                        <span>{service.cpu_percent.toFixed(1)}%</span>
                      </div>
                      <div className="flex items-center gap-1" title="Memory">
                        <HardDrive className="h-3 w-3" />
                        <span>{formatBytes(service.memory_usage)}</span>
                      </div>
                    </div>
                  )}
                </div>
              </Link>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
