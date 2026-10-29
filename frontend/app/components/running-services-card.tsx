import { useQuery } from "@tanstack/react-query";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { api } from "@/lib/api";
import { cn } from "@/lib/utils";
import { QueryError } from "@/components/query-error";
import { useTeamContext } from "@/lib/team-context";
import { Activity, Database, Server } from "lucide-react";
import { Link } from "react-router";
import type { Service } from "@/types/api";

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

const TYPE_META = {
  app: { label: "App", Icon: Activity, tint: "text-blue-500" },
  database: { label: "Database", Icon: Database, tint: "text-purple-500" },
  service: { label: "Service", Icon: Server, tint: "text-emerald-500" },
} as const;

export function RunningServicesCard({ className }: { className?: string } = {}) {
  const { currentTeamId } = useTeamContext();

  const { data: services = [] } = useQuery<Service[]>({
    queryKey: ["services", currentTeamId],
    queryFn: () => api.getServices(currentTeamId ? { teamId: currentTeamId } : undefined),
    refetchInterval: 30000,
  });

  // One request for CPU/memory of every running app and database (was one request
  // per resource, each waiting on Docker for a stats sample).
  const resourcesQuery = useQuery({
    queryKey: ["system-resources", currentTeamId],
    queryFn: () => api.getResourceStats({ teamId: currentTeamId ?? undefined }),
    refetchInterval: 15000,
    retry: 1,
  });

  const resourceInfos: RunningServiceInfo[] = (resourcesQuery.data ?? []).map((r) => ({
    id: r.id,
    name: r.name,
    type: r.type,
    status: "running",
    cpu_percent: r.cpu_percent,
    memory_usage: r.memory_usage,
    memory_limit: r.memory_limit,
    url: r.type === "app" ? `/apps/${r.id}` : `/databases/${r.id}`,
  }));

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
    ...resourceInfos,
    ...serviceInfos,
  ].sort((a, b) => b.memory_usage - a.memory_usage);

  const isLoading = resourcesQuery.isLoading;

  return (
    <Card className={cn("min-h-0", className)}>
      <CardHeader className="pb-0">
        <div className="flex items-center justify-between">
          <CardTitle>Running services</CardTitle>
          {allRunning.length > 0 && (
            <span className="text-xs tabular-nums text-muted-foreground">
              {allRunning.length} active
            </span>
          )}
        </div>
      </CardHeader>
      <CardContent className="flex min-h-0 flex-1 flex-col px-0">
        {resourcesQuery.isError && resourcesQuery.data === undefined ? (
          <div className="px-4">
            <QueryError
              what="running services"
              error={resourcesQuery.error}
              onRetry={() => resourcesQuery.refetch()}
            />
          </div>
        ) : isLoading ? (
          <div className="space-y-3 px-4">
            {[1, 2, 3, 4, 5].map((i) => (
              <Skeleton key={i} className="h-9 w-full" />
            ))}
          </div>
        ) : allRunning.length === 0 ? (
          <div className="px-4 py-6 text-center text-sm text-muted-foreground">
            No running services
          </div>
        ) : (
          <>
            <div className="grid grid-cols-[1fr_4rem_5rem] gap-2 px-4 pb-1 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
              <span>Name</span>
              <span className="text-right">CPU</span>
              <span className="text-right">Memory</span>
            </div>
            <ul className="min-h-0 flex-1 overflow-y-auto px-2">
              {allRunning.map((service) => {
                const { label, Icon, tint } = TYPE_META[service.type];
                // Compose services have no per-container stats, so show "running".
                const hasStats = service.type !== "service";
                return (
                  <li key={`${service.type}-${service.id}`}>
                    <Link
                      to={service.url}
                      className="grid grid-cols-[1fr_4rem_5rem] items-center gap-2 rounded-md px-2 py-2 outline-none transition-colors hover:bg-muted/50 focus-visible:ring-2 focus-visible:ring-ring"
                    >
                      <span className="flex min-w-0 items-center gap-2.5">
                        <Icon className={cn("h-4 w-4 shrink-0", tint)} aria-hidden="true" />
                        <span className="min-w-0">
                          <span className="block truncate text-sm font-medium leading-tight">
                            {service.name}
                          </span>
                          <span className="block text-xs text-muted-foreground">{label}</span>
                        </span>
                      </span>
                      {hasStats ? (
                        <>
                          <span className="text-right font-mono text-xs tabular-nums text-muted-foreground">
                            {service.cpu_percent.toFixed(1)}%
                          </span>
                          <span className="text-right font-mono text-xs tabular-nums text-muted-foreground">
                            {formatBytes(service.memory_usage)}
                          </span>
                        </>
                      ) : (
                        <span className="col-span-2 text-right text-xs text-muted-foreground">
                          running
                        </span>
                      )}
                    </Link>
                  </li>
                );
              })}
            </ul>
          </>
        )}
      </CardContent>
    </Card>
  );
}
