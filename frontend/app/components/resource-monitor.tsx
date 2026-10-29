import { useQuery } from "@tanstack/react-query";
import { Cpu, HardDrive, MemoryStick, Network } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";
import { MetricStrip, MetricTile, toneForPercent } from "@/components/metrics";
import { api } from "@/lib/api";
import type { ContainerStats } from "@/types/api";

interface ResourceMonitorProps {
  /** App ID (for app stats) */
  appId?: string;
  /** Database ID (for database stats) */
  databaseId?: string;
  /** Service ID (for Docker Compose service stats) */
  serviceId?: string;
  /** Auth token for API calls (optional - uses cookies if not provided) */
  token?: string;
  /** Polling interval in milliseconds (default: 5000) */
  pollInterval?: number;
  /** CPU limit in cores (e.g. 0.25, 1.0). When provided, CPU% is shown relative to this limit */
  cpuLimit?: number;
}

// Format bytes to human-readable string
function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
}

function SectionHeader({ pollInterval }: { pollInterval: number }) {
  return (
    <div className="flex items-center justify-between">
      <h2 className="text-sm font-medium">Resource usage</h2>
      <span className="text-xs text-muted-foreground">Updated every {pollInterval / 1000}s</span>
    </div>
  );
}

export function ResourceMonitor({
  appId,
  databaseId,
  serviceId,
  token,
  pollInterval = 5000,
  cpuLimit,
}: ResourceMonitorProps) {
  const resourceId = appId || databaseId || serviceId;
  const resourceType = appId ? "app" : serviceId ? "service" : "database";

  const {
    data: stats,
    isLoading,
    error,
    isError,
  } = useQuery<ContainerStats>({
    queryKey: [`${resourceType}-stats`, resourceId],
    queryFn: () => {
      if (appId) return api.getAppStats(appId, token);
      if (serviceId) return api.getServiceStats(serviceId);
      return api.getDatabaseStats(databaseId!, token);
    },
    enabled: !!resourceId,
    refetchInterval: pollInterval,
    refetchIntervalInBackground: false,
    retry: 1,
    staleTime: pollInterval - 1000,
  });

  if (isError || (!isLoading && !stats)) {
    return (
      <div className="space-y-3">
        <SectionHeader pollInterval={pollInterval} />
        <Card>
          <CardContent className="py-8 text-center text-sm text-muted-foreground">
            {error instanceof Error
              ? error.message.includes("not found")
                ? "No running container to monitor"
                : `Error: ${error.message}`
              : "Unable to load metrics"}
          </CardContent>
        </Card>
      </div>
    );
  }

  // Docker reports cpu_percent as % of one full core (0.25 cores in use = 25%).
  // Dividing by the limit gives % of the allocated quota.
  const cpuPercent = stats
    ? cpuLimit && cpuLimit > 0
      ? Math.min(stats.cpu_percent / cpuLimit, 100)
      : stats.cpu_percent
    : 0;
  const memoryPercent =
    stats && stats.memory_limit > 0 ? (stats.memory_usage / stats.memory_limit) * 100 : undefined;
  const hasStorage = stats?.storage_bytes != null;

  return (
    <div className="space-y-3">
      <SectionHeader pollInterval={pollInterval} />
      <MetricStrip
        className={hasStorage ? "grid-cols-2 lg:grid-cols-4" : "grid-cols-1 sm:grid-cols-3"}
      >
        <MetricTile
          label="CPU"
          icon={<Cpu />}
          loading={!stats}
          value={`${cpuPercent.toFixed(1)}`}
          unit="%"
          percent={cpuPercent}
          tone={toneForPercent(cpuPercent)}
          detail={cpuLimit ? `of ${cpuLimit} core${cpuLimit !== 1 ? "s" : ""} limit` : "of one core"}
        />
        <MetricTile
          label="Memory"
          icon={<MemoryStick />}
          loading={!stats}
          value={stats ? formatBytes(stats.memory_usage) : undefined}
          percent={memoryPercent}
          tone={memoryPercent !== undefined ? toneForPercent(memoryPercent) : undefined}
          detail={stats && stats.memory_limit > 0 ? `of ${formatBytes(stats.memory_limit)}` : "no limit set"}
        />
        <MetricTile
          label="Network"
          icon={<Network />}
          loading={!stats}
          value={stats ? formatBytes(stats.network_rx) : undefined}
          detail={stats ? `received, ${formatBytes(stats.network_tx)} sent` : undefined}
        />
        {hasStorage && stats && (
          <MetricTile
            label="Storage"
            icon={<HardDrive />}
            value={formatBytes(stats.storage_bytes as number)}
            detail="data on disk"
          />
        )}
      </MetricStrip>
    </div>
  );
}
