import { useState } from "react";
import { useParams } from "react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";

export function meta() {
  return [
    { title: "Monitoring - Rivetr" },
    { name: "description", content: "Application health checks and alert rules" },
  ];
}
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { monitoringApi } from "@/lib/api/monitoring";
import type {
  LogSearchResult,
  LogRetentionPolicy,
  UptimeSummary,
  UptimeCheck,
  ScheduledRestart,
  CreateScheduledRestartRequest,
} from "@/types/api";
import {
  Search,
  Clock,
  Activity,
  Shield,
  Trash2,
  Plus,
  Save,
  RefreshCw,
  CheckCircle2,
  XCircle,
  AlertTriangle,
} from "lucide-react";

// ---------------------------------------------------------------------------
// Cron presets for scheduled restarts
// ---------------------------------------------------------------------------

const RESTART_CRON_PRESETS = [
  { label: "Every 6 hours", value: "0 */6 * * *" },
  { label: "Daily at 3 AM", value: "0 3 * * *" },
  { label: "Daily at midnight", value: "0 0 * * *" },
  { label: "Weekly (Sunday 3 AM)", value: "0 3 * * 0" },
  { label: "Monthly (1st at 3 AM)", value: "0 3 1 * *" },
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatDate(dateStr: string | null): string {
  if (!dateStr) return "Never";
  try {
    const date = new Date(dateStr);
    return date.toLocaleString();
  } catch {
    return dateStr;
  }
}

function StatusIcon({ status }: { status: string }) {
  switch (status) {
    case "up":
      return <CheckCircle2 className="h-4 w-4 text-green-500" />;
    case "down":
      return <XCircle className="h-4 w-4 text-red-500" />;
    case "degraded":
      return <AlertTriangle className="h-4 w-4 text-yellow-500" />;
    default:
      return null;
  }
}

function AvailabilityBadge({ percent }: { percent: number }) {
  if (percent >= 99.9) {
    return <Badge className="bg-green-500 text-white text-lg px-3 py-1">{percent.toFixed(2)}%</Badge>;
  }
  if (percent >= 99.0) {
    return <Badge className="bg-green-600 text-white text-lg px-3 py-1">{percent.toFixed(2)}%</Badge>;
  }
  if (percent >= 95.0) {
    return (
      <Badge className="bg-yellow-500 text-white text-lg px-3 py-1">{percent.toFixed(2)}%</Badge>
    );
  }
  return (
    <Badge variant="destructive" className="text-lg px-3 py-1">
      {percent.toFixed(2)}%
    </Badge>
  );
}

// ---------------------------------------------------------------------------
// Log Search Section
// ---------------------------------------------------------------------------

function LogSearchSection({ appId }: { appId: string }) {
  const [searchQuery, setSearchQuery] = useState("");
  const [logLevel, setLogLevel] = useState<string>("all");
  const [fromDate, setFromDate] = useState("");
  const [toDate, setToDate] = useState("");
  const [hasSearched, setHasSearched] = useState(false);

  const {
    data: results,
    isLoading,
    refetch,
  } = useQuery<LogSearchResult[]>({
    queryKey: ["logSearch", appId, searchQuery, logLevel, fromDate, toDate],
    queryFn: () =>
      monitoringApi.searchLogs(appId, {
        q: searchQuery || undefined,
        level: logLevel !== "all" ? logLevel : undefined,
        from: fromDate || undefined,
        to: toDate || undefined,
        limit: 100,
      }),
    enabled: hasSearched,
  });

  const handleSearch = () => {
    setHasSearched(true);
    refetch();
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Search className="h-5 w-5" />
          Log Search
        </CardTitle>
        <CardDescription>
          Search through deployment logs across all deployments for this app.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex flex-wrap gap-3">
          <div className="flex-1 min-w-[200px]">
            <Input
              placeholder="Search logs..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleSearch()}
            />
          </div>
          <Select value={logLevel} onValueChange={setLogLevel}>
            <SelectTrigger className="w-[130px]">
              <SelectValue placeholder="Log level" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Levels</SelectItem>
              <SelectItem value="info">Info</SelectItem>
              <SelectItem value="warn">Warning</SelectItem>
              <SelectItem value="error">Error</SelectItem>
              <SelectItem value="debug">Debug</SelectItem>
            </SelectContent>
          </Select>
          <Input
            type="datetime-local"
            value={fromDate}
            onChange={(e) => setFromDate(e.target.value)}
            className="w-[200px]"
            placeholder="From"
          />
          <Input
            type="datetime-local"
            value={toDate}
            onChange={(e) => setToDate(e.target.value)}
            className="w-[200px]"
            placeholder="To"
          />
          <Button onClick={handleSearch} disabled={isLoading} className="gap-2">
            <Search className="h-4 w-4" />
            {isLoading ? "Searching..." : "Search"}
          </Button>
        </div>

        {hasSearched && results && (
          <div className="border rounded-md">
            {results.length === 0 ? (
              <p className="text-center text-muted-foreground py-8">No logs found matching your criteria.</p>
            ) : (
              <div className="max-h-[400px] overflow-auto">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead className="w-[180px]">Timestamp</TableHead>
                      <TableHead className="w-[80px]">Level</TableHead>
                      <TableHead>Message</TableHead>
                      <TableHead className="w-[120px]">Deployment</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {results.map((log) => (
                      <TableRow key={log.id}>
                        <TableCell className="text-xs font-mono">
                          {formatDate(log.timestamp)}
                        </TableCell>
                        <TableCell>
                          <Badge
                            variant={
                              log.level === "error"
                                ? "destructive"
                                : log.level === "warn"
                                  ? "secondary"
                                  : "outline"
                            }
                            className="text-xs"
                          >
                            {log.level}
                          </Badge>
                        </TableCell>
                        <TableCell>
                          <span className="text-sm font-mono whitespace-pre-wrap break-all">
                            {searchQuery
                              ? highlightText(log.message, searchQuery)
                              : log.message}
                          </span>
                        </TableCell>
                        <TableCell className="text-xs font-mono text-muted-foreground">
                          {log.deployment_id.slice(0, 8)}...
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            )}
            {results && results.length > 0 && (
              <p className="text-xs text-muted-foreground p-2 border-t">
                Showing {results.length} result{results.length !== 1 ? "s" : ""}
              </p>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

/** Simple text highlighting for search matches */
function highlightText(text: string, query: string) {
  if (!query) return text;
  const parts = text.split(new RegExp(`(${escapeRegex(query)})`, "gi"));
  return (
    <>
      {parts.map((part, i) =>
        part.toLowerCase() === query.toLowerCase() ? (
          <mark key={i} className="bg-yellow-200 dark:bg-yellow-800 rounded px-0.5">
            {part}
          </mark>
        ) : (
          <span key={i}>{part}</span>
        )
      )}
    </>
  );
}

function escapeRegex(str: string) {
  return str.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

// ---------------------------------------------------------------------------
// Uptime Section
// ---------------------------------------------------------------------------

function UptimeSection({ appId }: { appId: string }) {
  const [period, setPeriod] = useState<"24h" | "7d" | "30d">("24h");

  const { data: uptimeSummary, isLoading: summaryLoading } = useQuery<UptimeSummary>({
    queryKey: ["uptime", appId],
    queryFn: () => monitoringApi.getUptime(appId),
    refetchInterval: 60000,
  });

  const { data: history } = useQuery<UptimeCheck[]>({
    queryKey: ["uptimeHistory", appId, period],
    queryFn: () => monitoringApi.getUptimeHistory(appId, period),
    refetchInterval: 60000,
  });

  if (summaryLoading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Activity className="h-5 w-5" />
            Uptime
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="animate-pulse">
            <div className="h-8 bg-muted rounded w-1/4 mb-4"></div>
            <div className="h-32 bg-muted rounded"></div>
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Activity className="h-5 w-5" />
          Uptime
        </CardTitle>
        <CardDescription>
          Health check monitoring for your application. Checks run every 60 seconds.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Summary stats */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="text-center">
            <p className="text-sm text-muted-foreground mb-1">Availability</p>
            <AvailabilityBadge percent={uptimeSummary?.availability_percent ?? 100} />
          </div>
          <div className="text-center">
            <p className="text-sm text-muted-foreground mb-1">Total Checks</p>
            <p className="text-2xl font-bold">{uptimeSummary?.total_checks ?? 0}</p>
          </div>
          <div className="text-center">
            <p className="text-sm text-muted-foreground mb-1">Avg Response</p>
            <p className="text-2xl font-bold">
              {uptimeSummary?.avg_response_time_ms != null
                ? `${Math.round(uptimeSummary.avg_response_time_ms)}ms`
                : "-"}
            </p>
          </div>
          <div className="text-center">
            <p className="text-sm text-muted-foreground mb-1">Down Events</p>
            <p className="text-2xl font-bold text-red-500">
              {uptimeSummary?.down_checks ?? 0}
            </p>
          </div>
        </div>

        {/* Period selector + timeline */}
        <div className="space-y-3">
          <div className="flex items-center gap-2">
            <Label className="text-sm font-medium">Period:</Label>
            <div className="flex gap-1">
              {(["24h", "7d", "30d"] as const).map((p) => (
                <Button
                  key={p}
                  variant={period === p ? "default" : "outline"}
                  size="sm"
                  onClick={() => setPeriod(p)}
                >
                  {p}
                </Button>
              ))}
            </div>
          </div>

          {/* Uptime bar visualization */}
          {history && history.length > 0 && (
            <div>
              <p className="text-xs text-muted-foreground mb-2">
                Status Timeline ({history.length} checks)
              </p>
              <div className="flex gap-[1px] items-end h-8">
                {history.slice(-120).map((check, i) => (
                  <div
                    key={check.id || i}
                    className={`flex-1 min-w-[2px] rounded-sm ${
                      check.status === "up"
                        ? "bg-green-500"
                        : check.status === "degraded"
                          ? "bg-yellow-500"
                          : "bg-red-500"
                    }`}
                    style={{
                      height: check.response_time_ms
                        ? `${Math.min(100, Math.max(20, (check.response_time_ms / 50)))}%`
                        : "100%",
                    }}
                    title={`${check.status} - ${check.response_time_ms ?? "?"}ms - ${formatDate(check.checked_at)}`}
                  />
                ))}
              </div>
              <div className="flex justify-between text-xs text-muted-foreground mt-1">
                <span>{formatDate(history[0]?.checked_at)}</span>
                <span>{formatDate(history[history.length - 1]?.checked_at)}</span>
              </div>
            </div>
          )}

          {/* Recent checks table */}
          {uptimeSummary?.recent_checks && uptimeSummary.recent_checks.length > 0 && (
            <div className="border rounded-md max-h-[200px] overflow-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Status</TableHead>
                    <TableHead>Response Time</TableHead>
                    <TableHead>Status Code</TableHead>
                    <TableHead>Checked At</TableHead>
                    <TableHead>Error</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {uptimeSummary.recent_checks.map((check) => (
                    <TableRow key={check.id}>
                      <TableCell>
                        <div className="flex items-center gap-1.5">
                          <StatusIcon status={check.status} />
                          <span className="capitalize text-sm">{check.status}</span>
                        </div>
                      </TableCell>
                      <TableCell className="text-sm">
                        {check.response_time_ms != null ? `${check.response_time_ms}ms` : "-"}
                      </TableCell>
                      <TableCell className="text-sm font-mono">
                        {check.status_code ?? "-"}
                      </TableCell>
                      <TableCell className="text-xs">{formatDate(check.checked_at)}</TableCell>
                      <TableCell className="text-xs text-destructive max-w-[200px] truncate">
                        {check.error_message || "-"}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}

          {(!uptimeSummary || uptimeSummary.total_checks === 0) && (
            <p className="text-sm text-muted-foreground text-center py-4">
              No uptime data yet. Add a health check URL to your app to start monitoring.
            </p>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// Log Retention Section
// ---------------------------------------------------------------------------

function LogRetentionSection({ appId }: { appId: string }) {
  const queryClient = useQueryClient();
  const [retentionDays, setRetentionDays] = useState<number>(30);
  const [maxSizeMb, setMaxSizeMb] = useState<string>("");
  const [isLoaded, setIsLoaded] = useState(false);

  const { data: policy } = useQuery<LogRetentionPolicy>({
    queryKey: ["logRetention", appId],
    queryFn: () => monitoringApi.getLogRetention(appId),
  });

  // Sync form state when policy loads
  if (policy && !isLoaded) {
    setRetentionDays(policy.retention_days);
    setMaxSizeMb(policy.max_size_mb != null ? policy.max_size_mb.toString() : "");
    setIsLoaded(true);
  }

  const updateMutation = useMutation({
    mutationFn: () =>
      monitoringApi.updateLogRetention(appId, {
        retention_days: retentionDays,
        max_size_mb: maxSizeMb ? parseInt(maxSizeMb, 10) : null,
      }),
    onSuccess: () => {
      toast.success("Log retention policy updated");
      queryClient.invalidateQueries({ queryKey: ["logRetention", appId] });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to update retention policy");
    },
  });

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Shield className="h-5 w-5" />
          Log Retention
        </CardTitle>
        <CardDescription>
          Configure how long deployment logs are kept for this app.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="space-y-2">
            <Label htmlFor="retention-days">Retention Period (days)</Label>
            <Input
              id="retention-days"
              type="number"
              min={1}
              max={365}
              value={retentionDays}
              onChange={(e) => setRetentionDays(parseInt(e.target.value, 10) || 30)}
            />
            <p className="text-xs text-muted-foreground">
              Logs older than this will be automatically deleted.
            </p>
          </div>
          <div className="space-y-2">
            <Label htmlFor="max-size">Max Size (MB, optional)</Label>
            <Input
              id="max-size"
              type="number"
              min={1}
              value={maxSizeMb}
              onChange={(e) => setMaxSizeMb(e.target.value)}
              placeholder="No limit"
            />
            <p className="text-xs text-muted-foreground">
              When exceeded, oldest logs are deleted. Leave empty for no limit.
            </p>
          </div>
        </div>
        <Button
          onClick={() => updateMutation.mutate()}
          disabled={updateMutation.isPending}
          className="gap-2"
        >
          <Save className="h-4 w-4" />
          {updateMutation.isPending ? "Saving..." : "Save Retention Policy"}
        </Button>
      </CardContent>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// Scheduled Restarts Section
// ---------------------------------------------------------------------------

function ScheduledRestartsSection({ appId }: { appId: string }) {
  const queryClient = useQueryClient();
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [formCron, setFormCron] = useState("0 3 * * *");
  const [formEnabled, setFormEnabled] = useState(true);
  const [deleteConfirm, setDeleteConfirm] = useState<ScheduledRestart | null>(null);

  const { data: restarts, isLoading } = useQuery<ScheduledRestart[]>({
    queryKey: ["scheduledRestarts", appId],
    queryFn: () => monitoringApi.getScheduledRestarts(appId),
    refetchInterval: 30000,
  });

  const createMutation = useMutation({
    mutationFn: (data: CreateScheduledRestartRequest) =>
      monitoringApi.createScheduledRestart(appId, data),
    onSuccess: () => {
      toast.success("Scheduled restart created");
      queryClient.invalidateQueries({ queryKey: ["scheduledRestarts", appId] });
      setShowCreateDialog(false);
      setFormCron("0 3 * * *");
      setFormEnabled(true);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to create scheduled restart");
    },
  });

  const toggleMutation = useMutation({
    mutationFn: ({ restartId, enabled }: { restartId: string; enabled: boolean }) =>
      monitoringApi.updateScheduledRestart(appId, restartId, { enabled }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["scheduledRestarts", appId] });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to toggle restart");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (restartId: string) =>
      monitoringApi.deleteScheduledRestart(appId, restartId),
    onSuccess: () => {
      toast.success("Scheduled restart deleted");
      queryClient.invalidateQueries({ queryKey: ["scheduledRestarts", appId] });
      setDeleteConfirm(null);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to delete scheduled restart");
    },
  });

  return (
    <>
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <RefreshCw className="h-5 w-5" />
                Scheduled Restarts
              </CardTitle>
              <CardDescription>
                Automatically restart your app container on a cron schedule.
              </CardDescription>
            </div>
            <Button
              onClick={() => setShowCreateDialog(true)}
              size="sm"
              className="gap-2"
            >
              <Plus className="h-4 w-4" />
              Add Restart
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="animate-pulse">
              <div className="h-20 bg-muted rounded"></div>
            </div>
          ) : !restarts || restarts.length === 0 ? (
            <p className="text-sm text-muted-foreground text-center py-6">
              No scheduled restarts configured. Add one to automatically restart your app on a
              schedule.
            </p>
          ) : (
            <div className="border rounded-md">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Cron Expression</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Last Restart</TableHead>
                    <TableHead>Next Restart</TableHead>
                    <TableHead className="w-[100px]">Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {restarts.map((restart) => (
                    <TableRow key={restart.id}>
                      <TableCell>
                        <code className="text-xs bg-muted px-2 py-0.5 rounded">
                          {restart.cron_expression}
                        </code>
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <Switch
                            checked={restart.enabled}
                            onCheckedChange={(checked) =>
                              toggleMutation.mutate({
                                restartId: restart.id,
                                enabled: checked,
                              })
                            }
                          />
                          {restart.enabled ? (
                            <Badge className="bg-green-500 text-white">Enabled</Badge>
                          ) : (
                            <Badge variant="secondary">Disabled</Badge>
                          )}
                        </div>
                      </TableCell>
                      <TableCell className="text-sm">
                        {formatDate(restart.last_restart)}
                      </TableCell>
                      <TableCell className="text-sm">
                        {formatDate(restart.next_restart)}
                      </TableCell>
                      <TableCell>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => setDeleteConfirm(restart)}
                          className="text-destructive hover:text-destructive"
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Create Dialog */}
      <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Add Scheduled Restart</DialogTitle>
            <DialogDescription>
              Configure a cron schedule to automatically restart your app container.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="cron">Cron Expression</Label>
              <Input
                id="cron"
                value={formCron}
                onChange={(e) => setFormCron(e.target.value)}
                className="font-mono text-sm"
                placeholder="0 3 * * *"
              />
              <div className="flex flex-wrap gap-1.5 mt-1">
                {RESTART_CRON_PRESETS.map((preset) => (
                  <button
                    key={preset.value}
                    type="button"
                    onClick={() => setFormCron(preset.value)}
                    className="text-xs px-2 py-0.5 rounded border hover:bg-muted transition-colors"
                  >
                    {preset.label}
                  </button>
                ))}
              </div>
            </div>
            <div className="flex items-center space-x-2">
              <Switch
                id="enabled"
                checked={formEnabled}
                onCheckedChange={setFormEnabled}
              />
              <Label htmlFor="enabled">Enabled</Label>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowCreateDialog(false)}>
              Cancel
            </Button>
            <Button
              onClick={() =>
                createMutation.mutate({
                  cron_expression: formCron,
                  enabled: formEnabled,
                })
              }
              disabled={!formCron || createMutation.isPending}
            >
              {createMutation.isPending ? "Creating..." : "Create"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <Dialog open={!!deleteConfirm} onOpenChange={(open) => !open && setDeleteConfirm(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Scheduled Restart</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete this scheduled restart? This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteConfirm(null)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={() => deleteConfirm && deleteMutation.mutate(deleteConfirm.id)}
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending ? "Deleting..." : "Delete"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

// ---------------------------------------------------------------------------
// Main Monitoring Page
// ---------------------------------------------------------------------------

export default function MonitoringPage() {
  const { id: appId } = useParams();

  if (!appId) return null;

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold">Monitoring</h2>
        <p className="text-sm text-muted-foreground">
          Log search, uptime tracking, retention policies, and scheduled restarts.
        </p>
      </div>

      <LogSearchSection appId={appId} />
      <UptimeSection appId={appId} />
      <LogRetentionSection appId={appId} />
      <ScheduledRestartsSection appId={appId} />
    </div>
  );
}
