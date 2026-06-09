import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { api } from "@/lib/api";
import { useTeamContext } from "@/lib/team-context";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ResourceChart } from "@/components/resource-chart";
import { RecentEvents } from "@/components/recent-events";
import { CostSummaryCard } from "@/components/cost-summary-card";
import { RunningServicesCard } from "@/components/running-services-card";
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

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
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

interface StatCardProps {
  title: string;
  value: string | number;
  subtitle?: string;
  icon: React.ReactNode;
  iconBgColor: string;
  trend?: string;
  trendPositive?: boolean;
  /** Optional destination — renders the card as a clickable link */
  to?: string;
  /** Shrink the value font for long composite strings (e.g. "8.0 GB / 16.0 GB") */
  compactValue?: boolean;
}

function StatCard({
  title,
  value,
  subtitle,
  icon,
  iconBgColor,
  trend,
  trendPositive,
  to,
  compactValue,
}: StatCardProps) {
  const card = (
    <Card
      className={
        to
          ? "h-full transition-colors hover:border-primary/40 hover:bg-muted/30"
          : "h-full"
      }
    >
      <CardContent className="pt-4">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0 space-y-1">
            <p className="text-sm text-muted-foreground">{title}</p>
            <div className="flex items-baseline gap-2">
              <p
                className={`font-bold leading-none whitespace-nowrap ${
                  compactValue ? "text-xl" : "text-2xl"
                }`}
              >
                {value}
              </p>
              {subtitle && (
                <span className="text-sm text-muted-foreground">{subtitle}</span>
              )}
            </div>
            {trend && (
              <Badge
                variant="secondary"
                className={`text-xs ${
                  trendPositive
                    ? "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400"
                    : "bg-muted text-muted-foreground"
                }`}
              >
                {trend}
              </Badge>
            )}
          </div>
          <div className={`shrink-0 rounded-lg p-2.5 ${iconBgColor}`}>
            {icon}
          </div>
        </div>
      </CardContent>
    </Card>
  );

  if (to) {
    return (
      <Link to={to} className="block">
        {card}
      </Link>
    );
  }
  return card;
}

export default function DashboardPage() {
  const { currentTeamId } = useTeamContext();

  // Use React Query for real-time updates
  // Note: system stats are always fetched, teamId is optional server-side scoping,
  // not a required parameter. Passing null means "all resources" (personal workspace).
  const { data: stats, isLoading: statsLoading } = useQuery<SystemStats | null>({
    queryKey: ["system-stats", currentTeamId],
    queryFn: () => api.getSystemStats({ teamId: currentTeamId }).catch(() => null),
    refetchInterval: 10000, // Refresh every 10 seconds
  });

  // Use the same query key as the monitoring page so both share one cached result
  const { data: diskStats, isLoading: diskLoading } = useQuery<DiskStats | null>({
    queryKey: ["diskStats"],
    queryFn: () => api.getDiskStats().catch(() => null),
    refetchInterval: 30000, // Refresh every 30 seconds (disk stats change less frequently)
  });

  const { data: events = [], dataUpdatedAt } = useQuery({
    queryKey: ["recent-events"],
    queryFn: () => api.getRecentEvents(),
    refetchInterval: 15000,
    retry: 1,
  });

  // Compact memory display: render both used and total in the same (larger) unit
  // so the value stays on a single line instead of wrapping (e.g. "8.0 / 16.0 GB").
  const memoryDisplay = stats && stats.memory_total_bytes > 0
    ? formatBytesPair(stats.memory_used_bytes, stats.memory_total_bytes)
    : formatBytes(stats?.memory_used_bytes || 0);

  const memoryPercent = stats && stats.memory_total_bytes > 0
    ? (stats.memory_used_bytes / stats.memory_total_bytes) * 100
    : 0;

  // Calculate disk display values
  const diskDisplay = diskStats
    ? `${diskStats.used_human} / ${diskStats.total_human}`
    : "N/A";

  const isLoading = statsLoading || diskLoading;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">System Overview</h1>
          <p className="text-muted-foreground mt-1">
            Real-time health and performance across your services
          </p>
        </div>
        <div className="flex items-center gap-4">
          {dataUpdatedAt > 0 && (
            <span
              className="hidden items-center gap-1.5 text-xs text-muted-foreground sm:flex"
              title={new Date(dataUpdatedAt).toLocaleString()}
            >
              <span className="relative flex h-2 w-2">
                <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-400 opacity-75" />
                <span className="relative inline-flex h-2 w-2 rounded-full bg-green-500" />
              </span>
              Updated {formatRelativeTime(new Date(dataUpdatedAt))}
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

      {/* Stats Grid */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-5">
        <StatCard
          title="Running Services"
          value={isLoading ? "..." : (stats?.running_apps_count ?? 0) + (stats?.running_databases_count ?? 0) + (stats?.running_services_count ?? 0)}
          icon={<Activity className="h-5 w-5 text-green-600" />}
          iconBgColor="bg-green-100 dark:bg-green-900/30"
          trend={stats ? `${stats.running_apps_count} apps, ${stats.running_databases_count} dbs, ${stats.running_services_count} svcs` : undefined}
          trendPositive
        />
        <StatCard
          title="Total CPU Usage"
          value={isLoading ? "..." : `${stats?.total_cpu_percent?.toFixed(1) ?? 0}%`}
          icon={<Cpu className="h-5 w-5 text-blue-600" />}
          iconBgColor="bg-blue-100 dark:bg-blue-900/30"
          to="/monitoring"
        />
        <StatCard
          title="Memory Usage"
          value={isLoading ? "..." : memoryDisplay}
          compactValue
          icon={<HardDrive className="h-5 w-5 text-purple-600" />}
          iconBgColor="bg-purple-100 dark:bg-purple-900/30"
          trend={stats && stats.memory_total_bytes > 0 ? `${memoryPercent.toFixed(1)}% used` : undefined}
          trendPositive={memoryPercent < 80}
          to="/monitoring"
        />
        <StatCard
          title="Disk Usage"
          value={isLoading ? "..." : diskDisplay}
          compactValue
          icon={<Database className="h-5 w-5 text-orange-600" />}
          iconBgColor="bg-orange-100 dark:bg-orange-900/30"
          trend={diskStats ? `${diskStats.usage_percent.toFixed(1)}% used` : undefined}
          trendPositive={diskStats ? diskStats.usage_percent < 80 : true}
          to="/monitoring"
        />
        <StatCard
          title="Server Uptime"
          value={isLoading ? "..." : stats ? formatUptime(stats.uptime_seconds) : "--"}
          subtitle="since last restart"
          icon={<Clock className="h-5 w-5 text-amber-600" />}
          iconBgColor="bg-amber-100 dark:bg-amber-900/30"
        />
      </div>

      {/* Chart and Running Services */}
      <div className="grid gap-6 lg:grid-cols-3">
        <div className="lg:col-span-2">
          <ResourceChart
            cpuPercent={stats?.total_cpu_percent ?? 0}
            memoryPercent={memoryPercent}
          />
        </div>
        <div>
          <RunningServicesCard />
        </div>
      </div>

      {/* Events and Cost */}
      <div className="grid gap-6 lg:grid-cols-2">
        <CostSummaryCard />
        <RecentEvents initialEvents={events} />
      </div>
    </div>
  );
}
