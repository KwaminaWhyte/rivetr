import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { api } from "@/lib/api";
import { ResourceChart } from "@/components/ResourceChart";
import { RecentEvents } from "@/components/RecentEvents";
import type { SystemStats } from "@/types/api";
import { Activity, Cpu, HardDrive, Clock, Plus } from "lucide-react";

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

export function DashboardPage() {
  const { data: stats, isLoading } = useQuery<SystemStats>({
    queryKey: ["system-stats"],
    queryFn: () => api.getSystemStats(),
    refetchInterval: 10000, // Refresh every 10 seconds
  });

  // Calculate memory display values
  const memoryUsedGB = stats ? (stats.memory_used_bytes / (1024 * 1024 * 1024)).toFixed(1) : "0";
  const memoryTotalGB = stats ? (stats.memory_total_bytes / (1024 * 1024 * 1024)).toFixed(0) : "0";
  const memoryDisplay = stats && stats.memory_total_bytes > 0
    ? `${memoryUsedGB} / ${memoryTotalGB} GB`
    : formatBytes(stats?.memory_used_bytes || 0);

  const memoryPercent = stats && stats.memory_total_bytes > 0
    ? (stats.memory_used_bytes / stats.memory_total_bytes) * 100
    : 0;

  if (isLoading) {
    return (
      <div className="space-y-6">
        {/* Header skeleton */}
        <div className="flex items-center justify-between">
          <div>
            <div className="h-8 w-48 bg-muted animate-pulse rounded" />
            <div className="h-4 w-72 bg-muted animate-pulse rounded mt-2" />
          </div>
          <div className="h-9 w-28 bg-muted animate-pulse rounded" />
        </div>

        {/* Stats skeleton */}
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
          {[1, 2, 3, 4].map((i) => (
            <Card key={i}>
              <CardContent className="pt-4">
                <div className="flex items-start justify-between">
                  <div className="space-y-2">
                    <div className="h-4 w-24 bg-muted animate-pulse rounded" />
                    <div className="h-7 w-16 bg-muted animate-pulse rounded" />
                  </div>
                  <div className="h-10 w-10 bg-muted animate-pulse rounded-lg" />
                </div>
              </CardContent>
            </Card>
          ))}
        </div>

        {/* Chart and Events skeleton */}
        <div className="grid gap-6 lg:grid-cols-3">
          <div className="lg:col-span-2">
            <Card>
              <CardHeader>
                <div className="h-5 w-48 bg-muted animate-pulse rounded" />
              </CardHeader>
              <CardContent>
                <div className="h-[200px] bg-muted animate-pulse rounded" />
              </CardContent>
            </Card>
          </div>
          <div>
            <Card>
              <CardHeader>
                <div className="h-5 w-32 bg-muted animate-pulse rounded" />
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  {[1, 2, 3, 4].map((i) => (
                    <div key={i} className="h-12 bg-muted animate-pulse rounded" />
                  ))}
                </div>
              </CardContent>
            </Card>
          </div>
        </div>
      </div>
    );
  }

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
          <Link to="/apps/new">
            <Plus className="mr-2 h-4 w-4" />
            Quick Deploy
          </Link>
        </Button>
      </div>

      {/* Stats Grid */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <StatCard
          title="Running Services"
          value={stats?.running_apps_count ?? 0}
          icon={<Activity className="h-5 w-5 text-green-600" />}
          iconBgColor="bg-green-100 dark:bg-green-900/30"
          trend={stats?.running_apps_count ? `${Math.round((stats.running_apps_count / Math.max(stats.total_apps_count, 1)) * 100)}% active` : undefined}
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
          <RecentEvents />
        </div>
      </div>
    </div>
  );
}
