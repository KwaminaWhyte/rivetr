import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { api } from "@/lib/api";
import { useTeamContext } from "@/lib/team-context";
import { Button } from "@/components/ui/button";
import { ResourceChart } from "@/components/resource-chart";
import { RecentEvents } from "@/components/recent-events";
import { CostSummaryCard } from "@/components/cost-summary-card";
import { QueryError } from "@/components/query-error";
import { RunningServicesCard } from "@/components/running-services-card";
import { MetricStrip, MetricTile, toneForPercent } from "@/components/metrics";
import type { SystemStats, DiskStats } from "@/types/api";
import { Activity, Cpu, HardDrive, Database, Clock, Plus } from "lucide-react";

function formatRelativeTime(date: Date): string {
  const diffSecs = Math.floor((Date.now() - date.getTime()) / 1000);
  if (diffSecs < 5) return "just now";
  if (diffSecs < 60) return `${diffSecs}s ago`;
  const diffMins = Math.floor(diffSecs / 60);
  if (diffMins < 60) return `${diffMins}m ago`;
  const diffHours = Math.floor(diffMins / 60);
  return `${diffHours}h ago`;
}

export function meta() {
  return [
    { title: "Dashboard - Rivetr" },
    { name: "description", content: "System overview and real-time health monitoring for your services" },
  ];
}

/**
 * Format a used/total byte pair sharing a single unit so the value stays on one
 * line, e.g. "8.0 / 16.0 GB" instead of "8.0 GB / 16.0 GB" (which wraps).
 */
function formatBytesPair(used: number, total: number): string {
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = total > 0 ? Math.floor(Math.log(total) / Math.log(k)) : 0;
  const fmt = (b: number) => parseFloat((b / Math.pow(k, i)).toFixed(1));
  return `${fmt(used)} / ${fmt(total)} ${sizes[i]}`;
}

function formatUptime(seconds: number): string {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);

  if (days > 0) {
    return `${days}d ${hours}h`;
  } else if (hours > 0) {
    return `${hours}h ${minutes}m`;
  } else {
    return `${minutes}m`;
  }
}

export default function DashboardPage() {
  const { currentTeamId } = useTeamContext();

  // Use React Query for real-time updates
  // Note: system stats are always fetched, teamId is optional server-side scoping,
  // not a required parameter. Passing null means "all resources" (personal workspace).
  const {
    data: stats,
    isLoading: statsLoading,
    error: statsError,
    refetch: refetchStats,
  } = useQuery<SystemStats>({
    queryKey: ["system-stats", currentTeamId],
    queryFn: () => api.getSystemStats({ teamId: currentTeamId }),
    refetchInterval: 10000, // Refresh every 10 seconds
    retry: 1,
  });

  // Use the same query key as the monitoring page so both share one cached result
  const {
    data: diskStats,
    isLoading: diskLoading,
    error: diskError,
    refetch: refetchDisk,
  } = useQuery<DiskStats>({
    queryKey: ["diskStats"],
    queryFn: () => api.getDiskStats(),
    refetchInterval: 30000, // Refresh every 30 seconds (disk stats change less frequently)
    retry: 1,
  });

  const { data: events = [], dataUpdatedAt } = useQuery({
    queryKey: ["recent-events"],
    queryFn: () => api.getRecentEvents(),
    refetchInterval: 15000,
    retry: 1,
  });

  const memoryPercent =
    stats && stats.memory_total_bytes > 0
      ? (stats.memory_used_bytes / stats.memory_total_bytes) * 100
      : 0;

  const runningTotal = stats
    ? (stats.running_apps_count ?? 0) +
      (stats.running_databases_count ?? 0) +
      (stats.running_services_count ?? 0)
    : undefined;

  const loadError = statsError ?? diskError;

  return (
    <div className="space-y-5">
      {/* Header */}
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Overview</h1>
          <p className="mt-0.5 text-sm text-muted-foreground">
            Health and resource use across your apps, databases and services
          </p>
        </div>
        <div className="flex items-center gap-4">
          {dataUpdatedAt > 0 && (
            <span
              className="hidden items-center gap-1.5 text-xs text-muted-foreground sm:flex"
              title={new Date(dataUpdatedAt).toLocaleString()}
            >
              <span className="relative flex h-2 w-2" aria-hidden="true">
                <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-400 opacity-75" />
                <span className="relative inline-flex h-2 w-2 rounded-full bg-green-500" />
              </span>
              Live, updated {formatRelativeTime(new Date(dataUpdatedAt))}
            </span>
          )}
          <Button asChild>
            <Link to="/projects">
              <Plus className="mr-2 h-4 w-4" />
              Quick Deploy
            </Link>
          </Button>
        </div>
      </div>

      {loadError && (
        <QueryError
          what="system stats"
          error={loadError}
          onRetry={() => {
            refetchStats();
            refetchDisk();
          }}
        />
      )}

      {/* Key metrics */}
      <MetricStrip className="grid-cols-2 lg:grid-cols-5">
        <MetricTile
          label="Running"
          icon={<Activity />}
          loading={statsLoading}
          value={runningTotal}
          detail={
            stats
              ? `${stats.running_apps_count} apps, ${stats.running_databases_count} dbs, ${stats.running_services_count} services`
              : undefined
          }
          to="/projects"
          className="col-span-2 lg:col-span-1"
        />
        <MetricTile
          label="CPU"
          icon={<Cpu />}
          loading={statsLoading}
          value={stats ? (stats.total_cpu_percent ?? 0).toFixed(1) : undefined}
          unit={stats ? "%" : undefined}
          percent={stats ? (stats.total_cpu_percent ?? 0) : undefined}
          tone={stats ? toneForPercent(stats.total_cpu_percent ?? 0) : undefined}
          detail={stats && stats.cpu_count > 0 ? `${stats.cpu_count} cores` : undefined}
          to="/monitoring"
        />
        <MetricTile
          label="Memory"
          icon={<HardDrive />}
          loading={statsLoading}
          value={stats && stats.memory_total_bytes > 0 ? memoryPercent.toFixed(1) : undefined}
          unit={stats && stats.memory_total_bytes > 0 ? "%" : undefined}
          percent={stats && stats.memory_total_bytes > 0 ? memoryPercent : undefined}
          detail={
            stats && stats.memory_total_bytes > 0
              ? formatBytesPair(stats.memory_used_bytes, stats.memory_total_bytes)
              : undefined
          }
          to="/monitoring"
        />
        <MetricTile
          label="Disk"
          icon={<Database />}
          loading={diskLoading}
          value={diskStats ? diskStats.usage_percent.toFixed(1) : undefined}
          unit={diskStats ? "%" : undefined}
          percent={diskStats?.usage_percent}
          detail={diskStats ? `${diskStats.used_human} / ${diskStats.total_human}` : undefined}
          to="/monitoring"
        />
        <MetricTile
          label="Uptime"
          icon={<Clock />}
          loading={statsLoading}
          value={stats ? formatUptime(stats.uptime_seconds) : undefined}
          detail={stats ? "since last restart" : undefined}
        />
      </MetricStrip>

      {/* Chart and running services share one height: the list scrolls inside its card */}
      <div className="grid gap-4 lg:grid-cols-3">
        <div className="lg:col-span-2">
          <ResourceChart
            cpuPercent={stats?.total_cpu_percent ?? 0}
            memoryPercent={memoryPercent}
            height={268}
            className="h-full"
          />
        </div>
        <div className="relative">
          <RunningServicesCard className="max-h-96 lg:absolute lg:inset-0 lg:max-h-none" />
        </div>
      </div>

      {/* Cost and activity */}
      <div className="grid gap-4 lg:grid-cols-2">
        <CostSummaryCard />
        <RecentEvents initialEvents={events} />
      </div>
    </div>
  );
}
