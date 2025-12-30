import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import type { Route } from "./+types/_index";
import { api } from "@/lib/api";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ResourceChart } from "@/components/resource-chart";
import { RecentEvents } from "@/components/recent-events";
import type { SystemStats, DiskStats } from "@/types/api";
import { Activity, Cpu, HardDrive, Database, Clock, Plus } from "lucide-react";

export function meta() {
  return [
    { title: "Dashboard - Rivetr" },
    { name: "description", content: "System overview and real-time health monitoring for your services" },
  ];
}

export async function loader({ request }: Route.LoaderArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const [stats, diskStats, events] = await Promise.all([
    api.getSystemStats(token).catch(() => null),
    api.getDiskStats(token).catch(() => null),
    api.getRecentEvents(token).catch(() => []),
  ]);
  return { stats, diskStats, events, token };
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
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
}

function StatCard({
  title,
  value,
  subtitle,
  icon,
  iconBgColor,
  trend,
  trendPositive,
}: StatCardProps) {
  return (
    <Card>
      <CardContent className="pt-4">
        <div className="flex items-start justify-between">
          <div className="space-y-1">
            <p className="text-sm text-muted-foreground">{title}</p>
            <div className="flex items-baseline gap-2">
              <p className="text-2xl font-bold">{value}</p>
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
          <div
            className={`rounded-lg p-2.5 ${iconBgColor}`}
          >
            {icon}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

export default function DashboardPage({ loaderData }: Route.ComponentProps) {
  // Use React Query with SSR initial data for real-time updates
  const { data: stats } = useQuery<SystemStats | null>({
    queryKey: ["system-stats"],
    queryFn: () => api.getSystemStats(loaderData.token),
    initialData: loaderData.stats,
    refetchInterval: 10000, // Refresh every 10 seconds
  });

  const { data: diskStats } = useQuery<DiskStats | null>({
    queryKey: ["disk-stats"],
    queryFn: () => api.getDiskStats(loaderData.token),
    initialData: loaderData.diskStats,
    refetchInterval: 30000, // Refresh every 30 seconds (disk stats change less frequently)
  });

  // Calculate memory display values - use formatBytes for accurate display
  const memoryDisplay = stats && stats.memory_total_bytes > 0
    ? `${formatBytes(stats.memory_used_bytes)} / ${formatBytes(stats.memory_total_bytes)}`
    : formatBytes(stats?.memory_used_bytes || 0);

  const memoryPercent = stats && stats.memory_total_bytes > 0
    ? (stats.memory_used_bytes / stats.memory_total_bytes) * 100
    : 0;

  // Calculate disk display values
  const diskDisplay = diskStats
    ? `${diskStats.used_human} / ${diskStats.total_human}`
    : "N/A";

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
        <Button asChild>
          <Link to="/projects">
            <Plus className="mr-2 h-4 w-4" />
            Quick Deploy
          </Link>
        </Button>
      </div>

      {/* Stats Grid */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-5">
        <StatCard
          title="Running Services"
          value={(stats?.running_apps_count ?? 0) + (stats?.running_databases_count ?? 0) + (stats?.running_services_count ?? 0)}
          icon={<Activity className="h-5 w-5 text-green-600" />}
          iconBgColor="bg-green-100 dark:bg-green-900/30"
          trend={stats ? `${stats.running_apps_count} apps, ${stats.running_databases_count} dbs, ${stats.running_services_count} svcs` : undefined}
          trendPositive
        />
        <StatCard
          title="Total CPU Usage"
          value={`${stats?.total_cpu_percent?.toFixed(1) ?? 0}%`}
          icon={<Cpu className="h-5 w-5 text-blue-600" />}
          iconBgColor="bg-blue-100 dark:bg-blue-900/30"
        />
        <StatCard
          title="Memory Usage"
          value={memoryDisplay}
          icon={<HardDrive className="h-5 w-5 text-purple-600" />}
          iconBgColor="bg-purple-100 dark:bg-purple-900/30"
        />
        <StatCard
          title="Disk Usage"
          value={diskDisplay}
          icon={<Database className="h-5 w-5 text-orange-600" />}
          iconBgColor="bg-orange-100 dark:bg-orange-900/30"
          trend={diskStats ? `${diskStats.usage_percent.toFixed(1)}% used` : undefined}
          trendPositive={diskStats ? diskStats.usage_percent < 80 : true}
        />
        <StatCard
          title="Uptime"
          value={`${stats?.uptime_percent?.toFixed(2) ?? 99.99}%`}
          subtitle={stats ? formatUptime(stats.uptime_seconds) : undefined}
          icon={<Clock className="h-5 w-5 text-amber-600" />}
          iconBgColor="bg-amber-100 dark:bg-amber-900/30"
        />
      </div>

      {/* Chart and Events */}
      <div className="grid gap-6 lg:grid-cols-3">
        <div className="lg:col-span-2">
          <ResourceChart
            cpuPercent={stats?.total_cpu_percent ?? 0}
            memoryPercent={memoryPercent}
          />
        </div>
        <div>
          <RecentEvents initialEvents={loaderData.events} token={loaderData.token} />
        </div>
      </div>
    </div>
  );
}
