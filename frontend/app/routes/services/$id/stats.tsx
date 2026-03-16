import { useEffect, useState } from "react";
import { useOutletContext } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { AreaChart, Area, ResponsiveContainer, Tooltip } from "recharts";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { api } from "@/lib/api";
import type { Service, ContainerStats } from "@/types/api";

export function meta() {
  return [
    { title: "Service Stats - Rivetr" },
    { name: "description", content: "CPU, RAM, storage, and network usage for this service" },
  ];
}

interface OutletContext {
  service: Service;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
}

function formatPercent(value: number): string {
  return `${value.toFixed(1)}%`;
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function Sparkline({ data, color }: { data: number[]; color: string }) {
  if (data.length < 2) return null;
  const chartData = data.map((v, i) => ({ i, v }));
  const gradientId = `spark-${color.replace("#", "")}`;
  return (
    <div className="w-20 h-6">
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={chartData} margin={{ top: 1, right: 1, bottom: 1, left: 1 }}>
          <defs>
            <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor={color} stopOpacity={0.3} />
              <stop offset="95%" stopColor={color} stopOpacity={0} />
            </linearGradient>
          </defs>
          <Tooltip content={() => null} wrapperStyle={{ display: "none" }} />
          <Area
            type="monotone"
            dataKey="v"
            stroke={color}
            strokeWidth={1.5}
            fill={`url(#${gradientId})`}
            dot={false}
            isAnimationActive={false}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}

function ProgressBar({
  value,
  max,
  label,
  showPercent = true,
  color = "bg-blue-500",
}: {
  value: number;
  max: number;
  label: string;
  showPercent?: boolean;
  color?: string;
}) {
  const percent = max > 0 ? Math.min((value / max) * 100, 100) : 0;
  const isHighUsage = percent > 80;
  return (
    <div className="space-y-1">
      <div className="flex justify-between text-xs">
        <span className="text-muted-foreground">{label}</span>
        <span className={isHighUsage ? "text-red-500 font-medium" : ""}>
          {showPercent ? formatPercent(percent) : formatBytes(value)}
          {max > 0 && !showPercent && ` / ${formatBytes(max)}`}
        </span>
      </div>
      <div className="h-2 bg-muted rounded-full overflow-hidden">
        <div
          className={`h-full transition-all duration-300 ${isHighUsage ? "bg-red-500" : color}`}
          style={{ width: `${percent}%` }}
        />
      </div>
    </div>
  );
}

function StatDisplay({
  label,
  value,
  subValue,
  icon,
  history,
  historyColor,
}: {
  label: string;
  value: string;
  subValue?: string;
  icon: React.ReactNode;
  history?: number[];
  historyColor?: string;
}) {
  return (
    <div className="flex items-center gap-3">
      <div className="text-muted-foreground">{icon}</div>
      <div className="flex-1">
        <div className="text-xs text-muted-foreground">{label}</div>
        <div className="font-medium text-sm">{value}</div>
        {subValue && <div className="text-xs text-muted-foreground">{subValue}</div>}
      </div>
      {history && history.length > 1 && (
        <Sparkline data={history} color={historyColor ?? "#3b82f6"} />
      )}
    </div>
  );
}

// Inline SVG icons (same as resource-monitor.tsx to stay consistent)
const CpuIcon = () => (
  <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
    <path strokeLinecap="round" strokeLinejoin="round" d="M8.25 3v1.5M4.5 8.25H3m18 0h-1.5M4.5 12H3m18 0h-1.5M4.5 15.75H3m18 0h-1.5M8.25 19.5V21M12 3v1.5m0 15V21m3.75-18v1.5m0 15V21m-9-1.5h10.5a2.25 2.25 0 002.25-2.25V6.75a2.25 2.25 0 00-2.25-2.25H6.75A2.25 2.25 0 004.5 6.75v10.5a2.25 2.25 0 002.25 2.25zm.75-12h9v9h-9v-9z" />
  </svg>
);

const MemoryIcon = () => (
  <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
    <path strokeLinecap="round" strokeLinejoin="round" d="M5.25 14.25h13.5m-13.5 0a3 3 0 01-3-3m3 3a3 3 0 100 6h13.5a3 3 0 100-6m-16.5-3a3 3 0 013-3h13.5a3 3 0 013 3m-19.5 0a4.5 4.5 0 01.9-2.7L5.737 5.1a3.375 3.375 0 012.7-1.35h7.126c1.062 0 2.062.5 2.7 1.35l2.587 3.45a4.5 4.5 0 01.9 2.7m0 0a3 3 0 01-3 3m0 3h.008v.008h-.008v-.008zm0-6h.008v.008h-.008v-.008zm-3 6h.008v.008h-.008v-.008zm0-6h.008v.008h-.008v-.008z" />
  </svg>
);

const NetworkIcon = () => (
  <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
    <path strokeLinecap="round" strokeLinejoin="round" d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5m-13.5-9L12 3m0 0l4.5 4.5M12 3v13.5" />
  </svg>
);

// ---------------------------------------------------------------------------
// Main page
// ---------------------------------------------------------------------------

const POLL_INTERVAL = 5000;

export default function ServiceStatsPage() {
  const { service } = useOutletContext<OutletContext>();

  const [cpuHistory, setCpuHistory] = useState<number[]>([]);
  const [memoryHistory, setMemoryHistory] = useState<number[]>([]);

  const {
    data: stats,
    isLoading,
    error,
    isError,
  } = useQuery<ContainerStats>({
    queryKey: ["service-stats", service.id],
    queryFn: () => api.getServiceStats(service.id),
    enabled: !!service.id,
    refetchInterval: POLL_INTERVAL,
    refetchIntervalInBackground: false,
    retry: 1,
    staleTime: POLL_INTERVAL - 1000,
  });

  useEffect(() => {
    if (stats) {
      setCpuHistory((prev) => [...prev.slice(-19), stats.cpu_percent]);
      const memPct =
        stats.memory_limit > 0 ? (stats.memory_usage / stats.memory_limit) * 100 : 0;
      setMemoryHistory((prev) => [...prev.slice(-19), memPct]);
    }
  }, [stats]);

  const isStopped = service.status !== "running";

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold">Resource Usage</h2>
        <p className="text-sm text-muted-foreground">
          Live CPU, RAM, and network metrics aggregated across all containers in this service.
          {isStopped && (
            <span className="ml-1 text-yellow-600 dark:text-yellow-400">
              (Start the service to see live stats.)
            </span>
          )}
        </p>
      </div>

      {/* Loading skeleton */}
      {isLoading && !stats && (
        <Card>
          <CardHeader>
            <CardTitle>Resource Usage</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center justify-center py-8 text-muted-foreground">
              Loading metrics...
            </div>
          </CardContent>
        </Card>
      )}

      {/* Error state */}
      {isError && (
        <Card>
          <CardHeader>
            <CardTitle>Resource Usage</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center justify-center py-8 text-muted-foreground text-sm">
              {error instanceof Error
                ? `Error: ${error.message}`
                : "Unable to load metrics"}
            </div>
          </CardContent>
        </Card>
      )}

      {/* Stats display */}
      {stats && !isError && (
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle>Resource Usage</CardTitle>
              <span className="text-xs text-muted-foreground">
                Updated every {POLL_INTERVAL / 1000}s
              </span>
            </div>
          </CardHeader>
          <CardContent>
            <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
              {/* CPU */}
              <div className="space-y-3">
                <StatDisplay
                  label="CPU Usage"
                  value={formatPercent(stats.cpu_percent)}
                  icon={<CpuIcon />}
                  history={cpuHistory}
                  historyColor={stats.cpu_percent > 80 ? "#ef4444" : "#3b82f6"}
                />
                <ProgressBar
                  value={stats.cpu_percent}
                  max={100}
                  label="CPU"
                  color="bg-blue-500"
                />
              </div>

              {/* Memory */}
              <div className="space-y-3">
                <StatDisplay
                  label="Memory Usage"
                  value={formatBytes(stats.memory_usage)}
                  subValue={
                    stats.memory_limit > 0
                      ? `of ${formatBytes(stats.memory_limit)}`
                      : "no limit"
                  }
                  icon={<MemoryIcon />}
                  history={memoryHistory}
                  historyColor={
                    stats.memory_limit > 0 &&
                    stats.memory_usage / stats.memory_limit > 0.8
                      ? "#ef4444"
                      : "#22c55e"
                  }
                />
                {stats.memory_limit > 0 ? (
                  <ProgressBar
                    value={stats.memory_usage}
                    max={stats.memory_limit}
                    label="Memory"
                    showPercent={false}
                    color="bg-green-500"
                  />
                ) : (
                  <div className="h-2 bg-muted rounded-full overflow-hidden">
                    <div className="h-full bg-green-500 w-0" />
                  </div>
                )}
              </div>

              {/* Network I/O */}
              <div className="space-y-3">
                <StatDisplay
                  label="Network I/O"
                  value={`${formatBytes(stats.network_rx)} / ${formatBytes(stats.network_tx)}`}
                  subValue="Received / Transmitted"
                  icon={<NetworkIcon />}
                />
                <div className="grid grid-cols-2 gap-2 text-xs">
                  <div className="bg-muted/50 rounded px-2 py-1.5">
                    <span className="text-muted-foreground">RX: </span>
                    <span className="font-medium">{formatBytes(stats.network_rx)}</span>
                  </div>
                  <div className="bg-muted/50 rounded px-2 py-1.5">
                    <span className="text-muted-foreground">TX: </span>
                    <span className="font-medium">{formatBytes(stats.network_tx)}</span>
                  </div>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Stopped / no stats yet */}
      {isStopped && !isLoading && (
        <Card>
          <CardContent className="py-8 text-center text-muted-foreground text-sm">
            No live metrics available — start the service to begin monitoring.
          </CardContent>
        </Card>
      )}
    </div>
  );
}
