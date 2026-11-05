import { useQuery } from "@tanstack/react-query";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Server,
  HardDrive,
  MemoryStick,
  Gauge,
  Cpu,
  CheckCircle2,
  XCircle,
  AlertTriangle,
  RefreshCw,
  ShieldCheck,
} from "lucide-react";
import { QueryError } from "@/components/query-error";
import { api } from "@/lib/api";
import { useTeamContext } from "@/lib/team-context";
import { ResourceChart } from "@/components/resource-chart";
import {
  Meter,
  MetricStrip,
  MetricTile,
  StatusPill,
  toneForPercent,
  toneTextClass,
  type Tone,
} from "@/components/metrics";
import type { SystemHealthStatus, DiskStats, CheckResult, SystemStats } from "@/types/api";

export function meta() {
  return [
    { title: "Monitoring - Rivetr" },
    { name: "description", content: "System health and resource monitoring" },
  ];
}

function HealthStatusIcon({ passed, critical }: { passed: boolean; critical: boolean }) {
  if (passed) {
    return <CheckCircle2 className="h-4 w-4 text-emerald-500" aria-label="Passing" />;
  }
  if (critical) {
    return <XCircle className="h-4 w-4 text-red-500" aria-label="Failing" />;
  }
  return <AlertTriangle className="h-4 w-4 text-amber-500" aria-label="Warning" />;
}

/** `database_connectivity` -> `Database connectivity` */
function humanizeCheckName(name: string): string {
  const spaced = name.replace(/_/g, " ").trim();
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

function CheckRow({ check }: { check: CheckResult }) {
  return (
    <li className="flex items-start gap-3 px-4 py-3">
      <span className="mt-0.5 shrink-0">
        <HealthStatusIcon passed={check.passed} critical={check.critical} />
      </span>
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-sm font-medium">{humanizeCheckName(check.name)}</span>
          {check.critical && (
            <Badge variant="outline" className="text-[10px] font-normal uppercase tracking-wide">
              Critical
            </Badge>
          )}
        </div>
        <p className="mt-0.5 text-sm text-muted-foreground">{check.message}</p>
        {check.details && (
          <p className="mt-1 break-all font-mono text-xs text-muted-foreground">{check.details}</p>
        )}
      </div>
    </li>
  );
}

/** Format a byte count to a human-readable string (binary units). */
function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / Math.pow(1024, i);
  return `${value.toFixed(i === 0 ? 0 : 2)} ${units[i]}`;
}

/** Format an uptime duration (in seconds) to a compact human string. */
function formatUptime(seconds: number): string {
  if (!seconds || seconds <= 0) return "--";
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${minutes}m`;
  return `${minutes}m`;
}

function DetailRow({
  label,
  children,
  sub,
}: {
  label: string;
  children: React.ReactNode;
  sub?: React.ReactNode;
}) {
  return (
    <div className="flex items-start justify-between gap-4 px-4 py-3">
      <dt className="text-sm text-muted-foreground">{label}</dt>
      <dd className="min-w-0 text-right">
        <div className="text-sm font-medium tabular-nums">{children}</div>
        {sub && <div className="mt-0.5 text-xs text-muted-foreground tabular-nums">{sub}</div>}
      </dd>
    </div>
  );
}

function ServiceStatus({
  ok,
  okLabel,
  badLabel,
  badTone = "bad",
}: {
  ok: boolean | undefined;
  okLabel: string;
  badLabel: string;
  badTone?: Tone;
}) {
  if (ok === undefined) return <StatusPill tone="neutral">Checking</StatusPill>;
  return ok ? (
    <StatusPill tone="ok">{okLabel}</StatusPill>
  ) : (
    <StatusPill tone={badTone}>{badLabel}</StatusPill>
  );
}

export default function MonitoringPage() {
  const { currentTeamId } = useTeamContext();

  // Query for system stats, scoped to current team (teamId null = global/personal workspace)
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

  // Query for health status with polling
  const {
    data: health,
    isLoading: healthLoading,
    error: healthError,
    refetch: refetchHealth,
  } = useQuery<SystemHealthStatus>({
    queryKey: ["systemHealth"],
    queryFn: () => api.getSystemHealth(),
    refetchInterval: 30000, // Refresh every 30 seconds
    retry: 1,
  });

  // Query for disk stats with polling, same key as dashboard so both pages share one cache entry
  const {
    data: disk,
    isLoading: diskLoading,
    error: diskError,
    refetch: refetchDisk,
  } = useQuery<DiskStats>({
    queryKey: ["diskStats"],
    queryFn: () => api.getDiskStats(),
    refetchInterval: 60000, // Refresh every minute
    retry: 1,
  });

  const handleRefresh = () => {
    refetchStats();
    refetchHealth();
    refetchDisk();
  };

  // Host-wide memory: prefer the true host figures (total - available) from
  // /proc/meminfo; fall back to the container-aggregate used bytes when the
  // host figures are unavailable (e.g. non-Linux dev host).
  const hostMemTotal = stats?.host_memory_total_bytes || stats?.memory_total_bytes || 0;
  const hostMemUsed =
    stats && stats.host_memory_total_bytes > 0
      ? stats.host_memory_used_bytes
      : stats?.memory_used_bytes || 0;
  const hostMemAvailable =
    stats && stats.host_memory_total_bytes > 0
      ? stats.host_memory_available_bytes
      : Math.max(hostMemTotal - hostMemUsed, 0);

  // Memory percentage used by the resource chart / cards.
  const memoryPercent = hostMemTotal > 0 ? (hostMemUsed / hostMemTotal) * 100 : 0;

  const swapTotal = stats?.swap_total_bytes || 0;
  const swapUsed = stats?.swap_used_bytes || 0;
  const swapPercent = swapTotal > 0 ? (swapUsed / swapTotal) * 100 : 0;

  // Load average relative to CPU count (1.0 per-core == fully loaded).
  const cpuCount = stats?.cpu_count || 1;
  const load1 = stats?.load_average_1m ?? 0;
  const loadPerCorePercent = cpuCount > 0 ? (load1 / cpuCount) * 100 : 0;

  // CPU steal: since-boot share of cycles the hypervisor took for other tenants.
  // High = oversubscribed/contended host (a hosting-provider issue, not your apps).
  const cpuSteal = stats?.cpu_steal_percent ?? 0;
  const stealTone: Tone = cpuSteal >= 25 ? "bad" : cpuSteal >= 5 ? "warn" : "ok";
  const stealNote =
    cpuSteal >= 25
      ? "host heavily oversubscribed"
      : cpuSteal >= 5
        ? "host contention"
        : "host not oversubscribed";

  const checks = health?.checks ?? [];
  const passingChecks = checks.filter((c) => c.passed).length;
  const checksTone: Tone =
    checks.length === 0
      ? "neutral"
      : checks.some((c) => !c.passed && c.critical)
        ? "bad"
        : passingChecks === checks.length
          ? "ok"
          : "warn";

  const overallPill = healthLoading ? (
    <StatusPill tone="neutral">Checking</StatusPill>
  ) : health ? (
    <StatusPill tone={health.healthy ? "ok" : "bad"}>
      {health.healthy ? "All systems healthy" : "Needs attention"}
    </StatusPill>
  ) : (
    <StatusPill tone="neutral">Status unavailable</StatusPill>
  );

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Monitoring</h1>
          <p className="mt-0.5 text-sm text-muted-foreground">
            Host resources, system health and history
            {health?.version ? ` · Rivetr v${health.version}` : ""}
          </p>
        </div>
        <div className="flex items-center gap-3">
          {overallPill}
          <Button variant="outline" size="sm" onClick={handleRefresh} className="gap-2">
            <RefreshCw className="h-4 w-4" />
            Refresh
          </Button>
        </div>
      </div>

      {(statsError || healthError || diskError) && (
        <QueryError
          what="monitoring data"
          error={statsError ?? healthError ?? diskError}
          onRetry={handleRefresh}
        />
      )}

      {/* Utilisation at a glance */}
      <MetricStrip className="grid-cols-2 lg:grid-cols-4">
        <MetricTile
          label="CPU"
          icon={<Cpu />}
          loading={statsLoading}
          value={stats ? stats.total_cpu_percent.toFixed(1) : undefined}
          unit={stats ? "%" : undefined}
          percent={stats?.total_cpu_percent}
          detail={stats ? `across ${cpuCount} ${cpuCount === 1 ? "core" : "cores"}` : undefined}
        />
        <MetricTile
          label="Memory"
          icon={<MemoryStick />}
          loading={statsLoading}
          value={stats && hostMemTotal > 0 ? memoryPercent.toFixed(1) : undefined}
          unit={stats && hostMemTotal > 0 ? "%" : undefined}
          percent={stats && hostMemTotal > 0 ? memoryPercent : undefined}
          detail={
            stats && hostMemTotal > 0
              ? `${formatBytes(hostMemUsed)} / ${formatBytes(hostMemTotal)}`
              : undefined
          }
        />
        <MetricTile
          label="Disk"
          icon={<HardDrive />}
          loading={diskLoading}
          value={disk ? disk.usage_percent.toFixed(1) : undefined}
          unit={disk ? "%" : undefined}
          percent={disk?.usage_percent}
          detail={disk ? `${disk.used_human} / ${disk.total_human}` : undefined}
        />
        <MetricTile
          label="Load average"
          icon={<Gauge />}
          loading={statsLoading}
          value={stats && (stats.cpu_count > 0 || load1 > 0) ? load1.toFixed(2) : undefined}
          unit={stats && (stats.cpu_count > 0 || load1 > 0) ? "1m" : undefined}
          percent={stats ? loadPerCorePercent : undefined}
          tone={stats ? toneForPercent(loadPerCorePercent) : undefined}
          detail={
            stats
              ? `${(stats.load_average_5m ?? 0).toFixed(2)} 5m, ${(stats.load_average_15m ?? 0).toFixed(2)} 15m, ${loadPerCorePercent.toFixed(0)}% per core`
              : undefined
          }
        />
      </MetricStrip>

      {/* History */}
      <ResourceChart
        cpuPercent={stats?.total_cpu_percent ?? 0}
        memoryPercent={memoryPercent}
        height={260}
      />

      <div className="grid items-start gap-4 lg:grid-cols-5">
        {/* Host details */}
        <Card className="lg:col-span-2">
          <CardHeader className="pb-0">
            <CardTitle>Host</CardTitle>
          </CardHeader>
          <CardContent className="px-0">
            <dl className="divide-y divide-border">
              <DetailRow label="Uptime" sub={stats ? `${stats.uptime_percent.toFixed(2)}% availability` : undefined}>
                {stats && stats.uptime_seconds > 0 ? formatUptime(stats.uptime_seconds) : "--"}
              </DetailRow>
              <DetailRow label="CPU steal" sub={stats ? stealNote : undefined}>
                <span className={toneTextClass(stealTone)}>
                  {stats ? `${cpuSteal.toFixed(1)}%` : "--"}
                </span>
              </DetailRow>
              <DetailRow
                label="Memory available"
                sub={hostMemTotal > 0 ? `of ${formatBytes(hostMemTotal)}` : undefined}
              >
                {hostMemTotal > 0 ? formatBytes(hostMemAvailable) : "--"}
              </DetailRow>
              {swapTotal > 0 && (
                <div className="px-4 py-3">
                  <div className="flex items-start justify-between gap-4">
                    <dt className="text-sm text-muted-foreground">Swap</dt>
                    <dd className="text-sm font-medium tabular-nums">
                      {formatBytes(swapUsed)} / {formatBytes(swapTotal)} ({swapPercent.toFixed(0)}%)
                    </dd>
                  </div>
                  <Meter value={swapPercent} label="Swap used" className="mt-2" />
                </div>
              )}
              <DetailRow label="Disk free" sub={disk ? `on ${disk.path}` : undefined}>
                {disk ? disk.free_human : "--"}
              </DetailRow>
              <DetailRow label="Database">
                <ServiceStatus ok={health?.database_healthy} okLabel="Connected" badLabel="Error" />
              </DetailRow>
              <DetailRow label="Container runtime">
                <ServiceStatus
                  ok={health?.runtime_healthy}
                  okLabel="Available"
                  badLabel="Unavailable"
                  badTone="warn"
                />
              </DetailRow>
            </dl>
          </CardContent>
        </Card>

        {/* Health checks */}
        <Card className="lg:col-span-3">
          <CardHeader className="pb-0">
            <div className="flex items-center justify-between gap-3">
              <CardTitle className="flex items-center gap-2">
                <ShieldCheck className="h-4 w-4 text-muted-foreground" />
                Health checks
              </CardTitle>
              {checks.length > 0 && (
                <StatusPill tone={checksTone}>
                  {passingChecks} of {checks.length} passing
                </StatusPill>
              )}
            </div>
          </CardHeader>
          <CardContent className="px-0">
            {checks.length > 0 ? (
              <ul className="divide-y divide-border">
                {checks.map((check) => (
                  <CheckRow key={check.name} check={check} />
                ))}
              </ul>
            ) : healthLoading ? (
              <div className="space-y-3 px-4">
                {[1, 2, 3, 4].map((i) => (
                  <Skeleton key={i} className="h-12 w-full" />
                ))}
              </div>
            ) : (
              <div className="flex flex-col items-center justify-center px-4 py-10 text-center">
                <Server className="mb-3 h-10 w-10 text-muted-foreground" />
                <h3 className="text-base font-medium">No health data</h3>
                <p className="mt-1 max-w-sm text-sm text-muted-foreground">
                  Unable to fetch system health information. The backend may be unavailable.
                </p>
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
