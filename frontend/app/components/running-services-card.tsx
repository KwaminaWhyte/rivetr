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

  // Fetch apps
  const { data: apps = [] } = useQuery<App[]>({
    queryKey: ["apps", currentTeamId],
    queryFn: () => api.getApps(currentTeamId ?? undefined),
    refetchInterval: 30000,
  });

  // Fetch databases
  const { data: databases = [] } = useQuery<ManagedDatabase[]>({
    queryKey: ["databases", currentTeamId],
    queryFn: () => api.getDatabases(currentTeamId ?? undefined),
    refetchInterval: 30000,
  });

  // Fetch services
  const { data: services = [] } = useQuery<Service[]>({
    queryKey: ["services", currentTeamId],
    queryFn: () => api.getServices(currentTeamId ?? undefined),
    refetchInterval: 30000,
  });

  // Get stats for running apps
  const runningApps = apps.filter(app => app.current_deployment?.status === "running");
  const appStatsQueries = useQuery({
    queryKey: ["app-stats", runningApps.map(a => a.id)],
    queryFn: async () => {
      const statsPromises = runningApps.slice(0, 5).map(async (app) => {
        try {
          const stats = await api.getAppStats(app.id);
          return {
            id: app.id,
            name: app.name,
            type: "app" as const,
            status: "running",
            cpu_percent: stats?.cpu_percent || 0,
            memory_usage: stats?.memory_usage || 0,
            memory_limit: stats?.memory_limit || 0,
            url: `/apps/${app.id}`,
          };
        } catch {
          return {
            id: app.id,
            name: app.name,
            type: "app" as const,
            status: "running",
            cpu_percent: 0,
            memory_usage: 0,
            memory_limit: 0,
            url: `/apps/${app.id}`,
          };
        }
      });
      return Promise.all(statsPromises);
    },
    enabled: runningApps.length > 0,
    refetchInterval: 15000,
  });

  // Get stats for running databases
  const runningDatabases = databases.filter(db => db.status === "running");
  const dbStatsQueries = useQuery({
    queryKey: ["db-stats", runningDatabases.map(d => d.id)],
    queryFn: async () => {
      const statsPromises = runningDatabases.slice(0, 5).map(async (db) => {
        try {
          const stats = await api.getDatabaseStats(db.id);
          return {
            id: db.id,
            name: db.name,
            type: "database" as const,
            status: "running",
            cpu_percent: stats?.cpu_percent || 0,
            memory_usage: stats?.memory_usage || 0,
            memory_limit: stats?.memory_limit || 0,
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
      });
      return Promise.all(statsPromises);
    },
    enabled: runningDatabases.length > 0,
    refetchInterval: 15000,
  });

  // For services, we don't have individual stats endpoints yet
  // so we'll just show basic info without resource usage
  const runningServices = services.filter(s => s.status === "running");
  const serviceInfos: RunningServiceInfo[] = runningServices.slice(0, 5).map(service => ({
    id: service.id,
    name: service.name,
    type: "service" as const,
    status: "running",
    cpu_percent: 0,
    memory_usage: 0,
    memory_limit: 0,
    url: `/services/${service.id}`,
  }));

  // Combine all running services
  const allRunningServices: RunningServiceInfo[] = [
    ...(appStatsQueries.data || []),
    ...(dbStatsQueries.data || []),
    ...serviceInfos,
  ].sort((a, b) => b.memory_usage - a.memory_usage);

  // Take top 5
  const topServices = allRunningServices.slice(0, 5);

  const getIcon = (type: string) => {
    switch (type) {
      case "app":
        return <Activity className="h-4 w-4 text-blue-500" />;
      case "database":
        return <Database className="h-4 w-4 text-purple-500" />;
      case "service":
        return <Server className="h-4 w-4 text-green-500" />;
      default:
        return <Activity className="h-4 w-4" />;
    }
  };

  const getTypeBadge = (type: string) => {
    const colors = {
      app: "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400",
      database: "bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400",
      service: "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400",
    };
    return colors[type as keyof typeof colors] || "";
  };

  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-medium">Running Services</CardTitle>
      </CardHeader>
      <CardContent>
        {topServices.length === 0 ? (
          <div className="text-sm text-muted-foreground text-center py-4">
            No running services
          </div>
        ) : (
          <div className="space-y-3">
            {topServices.map((service) => (
              <Link
                key={`${service.type}-${service.id}`}
                to={service.url}
                className="block"
              >
                <div className="flex items-center justify-between p-2 rounded-lg hover:bg-muted/50 transition-colors">
                  <div className="flex items-center gap-2 min-w-0">
                    {getIcon(service.type)}
                    <div className="min-w-0">
                      <div className="font-medium text-sm truncate">{service.name}</div>
                      <Badge variant="secondary" className={`text-xs ${getTypeBadge(service.type)}`}>
                        {service.type}
                      </Badge>
                    </div>
                  </div>
                  <div className="flex items-center gap-4 text-xs text-muted-foreground shrink-0">
                    <div className="flex items-center gap-1" title="CPU Usage">
                      <Cpu className="h-3 w-3" />
                      <span>{service.cpu_percent.toFixed(1)}%</span>
                    </div>
                    <div className="flex items-center gap-1" title="Memory Usage">
                      <HardDrive className="h-3 w-3" />
                      <span>{formatBytes(service.memory_usage)}</span>
                    </div>
                  </div>
                </div>
              </Link>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
