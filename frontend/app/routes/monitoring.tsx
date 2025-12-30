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
  Container
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";
import type { SystemHealthStatus, DiskStats, CheckResult } from "@/types/api";

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

function DiskUsageBar({ used, total }: { used: number; total: number }) {
  const percentage = total > 0 ? (used / total) * 100 : 0;
  const getBarColor = () => {
    if (percentage >= 90) return "bg-red-500";
    if (percentage >= 75) return "bg-yellow-500";
    return "bg-green-500";
  };

  return (
    <div className="w-full">
      <div className="h-2 bg-muted rounded-full overflow-hidden">
        <div
          className={`h-full ${getBarColor()} transition-all duration-300`}
          style={{ width: `${Math.min(percentage, 100)}%` }}
        />
      </div>
    </div>
  );
}

export default function MonitoringPage() {
  // Query for health status with polling
  const { data: health, isLoading: healthLoading, refetch: refetchHealth } = useQuery<SystemHealthStatus | null>({
    queryKey: ["systemHealth"],
    queryFn: () => api.getSystemHealth().catch(() => null),
    refetchInterval: 30000, // Refresh every 30 seconds
  });

  // Query for disk stats with polling
  const { data: disk, isLoading: diskLoading, refetch: refetchDisk } = useQuery<DiskStats | null>({
    queryKey: ["diskStats"],
    queryFn: () => api.getDiskStats().catch(() => null),
    refetchInterval: 60000, // Refresh every minute
  });

  const handleRefresh = () => {
    refetchHealth();
    refetchDisk();
  };

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

      {/* Disk Usage Details */}
      {disk && (
        <Card>
          <CardHeader>
            <CardTitle>Disk Usage</CardTitle>
            <CardDescription>Storage consumption on {disk.path}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <DiskUsageBar used={disk.used_bytes} total={disk.total_bytes} />
            <div className="grid grid-cols-3 gap-4 text-sm">
              <div>
                <p className="text-muted-foreground">Used</p>
                <p className="font-medium">{disk.used_human}</p>
              </div>
              <div>
                <p className="text-muted-foreground">Free</p>
                <p className="font-medium">{disk.free_human}</p>
              </div>
              <div>
                <p className="text-muted-foreground">Total</p>
                <p className="font-medium">{disk.total_human}</p>
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
