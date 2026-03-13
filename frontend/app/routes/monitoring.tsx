import { useQuery } from "@tanstack/react-query";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Activity,
  Server,
  Database,
  HardDrive,
  CheckCircle2,
  XCircle,
  AlertTriangle,
  RefreshCw,
  Container,
} from "lucide-react";
import { RadialBarChart, RadialBar, PolarAngleAxis, ResponsiveContainer, Cell } from "recharts";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";
import { useTeamContext } from "@/lib/team-context";
import { ResourceChart } from "@/components/resource-chart";
import type { SystemHealthStatus, DiskStats, CheckResult, SystemStats } from "@/types/api";

export function meta() {
  return [
    { title: "Monitoring - Rivetr" },
    { name: "description", content: "System health and resource monitoring" },
  ];
}

function HealthStatusIcon({ passed, critical }: { passed: boolean; critical: boolean }) {
  if (passed) {
    return <CheckCircle2 className="h-5 w-5 text-green-500" />;
  }
  if (critical) {
    return <XCircle className="h-5 w-5 text-red-500" />;
  }
  return <AlertTriangle className="h-5 w-5 text-yellow-500" />;
}

function CheckResultCard({ check }: { check: CheckResult }) {
  return (
    <div className="flex items-start gap-3 p-3 rounded-lg border bg-card">
      <HealthStatusIcon passed={check.passed} critical={check.critical} />
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="font-medium text-sm">{check.name}</span>
          {check.critical && (
            <Badge variant="outline" className="text-xs">Critical</Badge>
          )}
        </div>
        <p className="text-sm text-muted-foreground mt-0.5">{check.message}</p>
        {check.details && (
          <p className="text-xs text-muted-foreground mt-1 font-mono">{check.details}</p>
        )}
      </div>
    </div>
  );
}

function DiskUsageGauge({ used, total, usedHuman, totalHuman }: { used: number; total: number; usedHuman: string; totalHuman: string }) {
  const percentage = total > 0 ? (used / total) * 100 : 0;
  const color = percentage >= 90 ? "#ef4444" : percentage >= 75 ? "#eab308" : "#22c55e";
  const data = [{ value: Math.min(percentage, 100) }];

  return (
    <div className="flex items-center gap-6">
      <div className="relative w-32 h-32 shrink-0">
        <ResponsiveContainer width="100%" height="100%">
          <RadialBarChart
            cx="50%"
            cy="50%"
            innerRadius="70%"
            outerRadius="100%"
            startAngle={90}
            endAngle={-270}
            data={data}
            barSize={10}
          >
            <PolarAngleAxis type="number" domain={[0, 100]} tick={false} />
            <RadialBar dataKey="value" background={{ fill: "hsl(var(--muted))" }} cornerRadius={5}>
              <Cell fill={color} />
            </RadialBar>
          </RadialBarChart>
        </ResponsiveContainer>
        <div className="absolute inset-0 flex flex-col items-center justify-center">
          <span className="text-xl font-bold">{percentage.toFixed(0)}%</span>
          <span className="text-xs text-muted-foreground">used</span>
        </div>
      </div>
      <div className="space-y-2 text-sm">
        <div>
          <p className="text-muted-foreground text-xs">Used</p>
          <p className="font-semibold">{usedHuman}</p>
        </div>
        <div>
          <p className="text-muted-foreground text-xs">Total</p>
          <p className="font-semibold">{totalHuman}</p>
        </div>
      </div>
    </div>
  );
}

export default function MonitoringPage() {
  const { currentTeamId } = useTeamContext();

  // Query for system stats, scoped to current team (teamId null = global/personal workspace)
  const { data: stats, refetch: refetchStats } = useQuery<SystemStats | null>({
    queryKey: ["system-stats", currentTeamId],
    queryFn: () => api.getSystemStats({ teamId: currentTeamId }).catch(() => null),
    refetchInterval: 10000, // Refresh every 10 seconds
  });

  // Query for health status with polling
  const { data: health, isLoading: healthLoading, refetch: refetchHealth } = useQuery<SystemHealthStatus | null>({
    queryKey: ["systemHealth"],
    queryFn: () => api.getSystemHealth().catch(() => null),
    refetchInterval: 30000, // Refresh every 30 seconds
  });

  // Query for disk stats with polling — same key as dashboard so both pages share one cache entry
  const { data: disk, isLoading: diskLoading, refetch: refetchDisk } = useQuery<DiskStats | null>({
    queryKey: ["diskStats"],
    queryFn: () => api.getDiskStats().catch(() => null),
    refetchInterval: 60000, // Refresh every minute
  });

  const handleRefresh = () => {
    refetchStats();
    refetchHealth();
    refetchDisk();
  };

  // Calculate memory percentage
  const memoryPercent = stats && stats.memory_total_bytes > 0
    ? (stats.memory_used_bytes / stats.memory_total_bytes) * 100
    : 0;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Monitoring</h1>
          <p className="text-muted-foreground">
            System health and resource monitoring
          </p>
        </div>
        <Button variant="outline" size="sm" onClick={handleRefresh} className="gap-2">
          <RefreshCw className="h-4 w-4" />
          Refresh
        </Button>
      </div>

      {/* Overall Status Cards */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">System Health</CardTitle>
            <Activity className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            {healthLoading ? (
              <div className="text-2xl font-bold">Loading...</div>
            ) : health ? (
              <>
                <div className="flex items-center gap-2">
                  {health.healthy ? (
                    <>
                      <CheckCircle2 className="h-5 w-5 text-green-500" />
                      <span className="text-2xl font-bold text-green-600">Healthy</span>
                    </>
                  ) : (
                    <>
                      <XCircle className="h-5 w-5 text-red-500" />
                      <span className="text-2xl font-bold text-red-600">Unhealthy</span>
                    </>
                  )}
                </div>
                <p className="text-xs text-muted-foreground mt-1">
                  v{health.version}
                </p>
              </>
            ) : (
              <div className="text-2xl font-bold text-muted-foreground">--</div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Database</CardTitle>
            <Database className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            {health ? (
              <div className="flex items-center gap-2">
                {health.database_healthy ? (
                  <>
                    <CheckCircle2 className="h-5 w-5 text-green-500" />
                    <span className="text-2xl font-bold text-green-600">Connected</span>
                  </>
                ) : (
                  <>
                    <XCircle className="h-5 w-5 text-red-500" />
                    <span className="text-2xl font-bold text-red-600">Error</span>
                  </>
                )}
              </div>
            ) : (
              <div className="text-2xl font-bold text-muted-foreground">--</div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Container Runtime</CardTitle>
            <Container className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            {health ? (
              <div className="flex items-center gap-2">
                {health.runtime_healthy ? (
                  <>
                    <CheckCircle2 className="h-5 w-5 text-green-500" />
                    <span className="text-2xl font-bold text-green-600">Available</span>
                  </>
                ) : (
                  <>
                    <AlertTriangle className="h-5 w-5 text-yellow-500" />
                    <span className="text-2xl font-bold text-yellow-600">Unavailable</span>
                  </>
                )}
              </div>
            ) : (
              <div className="text-2xl font-bold text-muted-foreground">--</div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Disk Space</CardTitle>
            <HardDrive className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            {diskLoading ? (
              <div className="text-2xl font-bold">Loading...</div>
            ) : disk ? (
              <>
                <div className="text-2xl font-bold">
                  {disk.usage_percent.toFixed(1)}% used
                </div>
                <p className="text-xs text-muted-foreground mt-1">
                  {disk.used_human} / {disk.total_human}
                </p>
              </>
            ) : (
              <div className="text-2xl font-bold text-muted-foreground">--</div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Resource Utilization Chart */}
      <ResourceChart
        cpuPercent={stats?.total_cpu_percent ?? 0}
        memoryPercent={memoryPercent}
      />

      {/* Disk Usage Details */}
      {disk && (
        <Card>
          <CardHeader>
            <CardTitle>Disk Usage</CardTitle>
            <CardDescription>Storage on {disk.path}</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex items-center justify-between gap-8">
              <DiskUsageGauge
                used={disk.used_bytes}
                total={disk.total_bytes}
                usedHuman={disk.used_human}
                totalHuman={disk.total_human}
              />
              <div className="flex-1 grid grid-cols-1 gap-3 text-sm">
                <div className="flex justify-between items-center p-3 rounded-lg bg-muted/40">
                  <span className="text-muted-foreground">Used</span>
                  <span className="font-semibold">{disk.used_human}</span>
                </div>
                <div className="flex justify-between items-center p-3 rounded-lg bg-muted/40">
                  <span className="text-muted-foreground">Free</span>
                  <span className="font-semibold">{disk.free_human}</span>
                </div>
                <div className="flex justify-between items-center p-3 rounded-lg bg-muted/40">
                  <span className="text-muted-foreground">Total</span>
                  <span className="font-semibold">{disk.total_human}</span>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Health Check Details */}
      <Card>
        <CardHeader>
          <CardTitle>Health Checks</CardTitle>
          <CardDescription>
            Detailed status of all system components
          </CardDescription>
        </CardHeader>
        <CardContent>
          {health && health.checks.length > 0 ? (
            <div className="grid gap-3 md:grid-cols-2">
              {health.checks.map((check, index) => (
                <CheckResultCard key={index} check={check} />
              ))}
            </div>
          ) : healthLoading ? (
            <div className="flex flex-col items-center justify-center py-12 text-center">
              <RefreshCw className="h-8 w-8 text-muted-foreground mb-4 animate-spin" />
              <p className="text-muted-foreground">Loading health checks...</p>
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-12 text-center">
              <Server className="h-12 w-12 text-muted-foreground mb-4" />
              <h3 className="text-lg font-medium">No Health Data</h3>
              <p className="text-muted-foreground max-w-sm mt-2">
                Unable to fetch system health information. The backend may be unavailable.
              </p>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
