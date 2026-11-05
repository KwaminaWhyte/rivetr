import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  AlertTriangle,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Clock,
  Loader2,
  Play,
  Save,
  Sparkles,
  XCircle,
} from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { QueryError } from "@/components/query-error";
import { serverOpsApi } from "@/lib/api/server-ops";
import type { CleanupRun, MaintenanceSettings } from "@/lib/api/server-ops";
import { ConfirmActionDialog } from "@/components/servers/confirm-action-dialog";
import {
  formatBytes,
  formatDuration,
  relativeTime,
} from "@/components/servers/server-ops-utils";
import { useServerContext } from "@/components/servers/server-context";

export function meta() {
  return [{ title: "Docker Cleanup - Rivetr" }];
}

const CRON_PRESETS = [
  { label: "Daily at 03:00", value: "0 3 * * *" },
  { label: "Every 6 hours", value: "0 */6 * * *" },
  { label: "Weekly (Sunday 03:00)", value: "0 3 * * 0" },
  { label: "Hourly", value: "0 * * * *" },
];

type CleanupForm = Pick<
  MaintenanceSettings,
  | "cleanup_enabled"
  | "cleanup_cron"
  | "cleanup_trigger"
  | "cleanup_threshold_pct"
  | "remove_unused_volumes"
  | "remove_unused_networks"
  | "remove_retained_images"
>;

function toForm(s: MaintenanceSettings): CleanupForm {
  return {
    cleanup_enabled: s.cleanup_enabled,
    cleanup_cron: s.cleanup_cron,
    cleanup_trigger: s.cleanup_trigger,
    cleanup_threshold_pct: s.cleanup_threshold_pct,
    remove_unused_volumes: s.remove_unused_volumes,
    remove_unused_networks: s.remove_unused_networks,
    remove_retained_images: s.remove_retained_images,
  };
}

function RunStatus({ run }: { run: CleanupRun }) {
  switch (run.status) {
    case "running":
      return (
        <Badge variant="outline" className="gap-1">
          <Loader2 className="h-3 w-3 animate-spin" />
          Running
        </Badge>
      );
    case "success":
      return (
        <Badge variant="outline" className="gap-1 border-green-500/30 text-green-700 dark:text-green-400">
          <CheckCircle2 className="h-3 w-3" />
          Success
        </Badge>
      );
    case "failed":
      return (
        <Badge variant="destructive" className="gap-1">
          <XCircle className="h-3 w-3" />
          Failed
        </Badge>
      );
    default:
      return <Badge variant="secondary">{run.status}</Badge>;
  }
}

function RunRow({ run }: { run: CleanupRun }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="rounded-md border">
      <button
        type="button"
        className="flex w-full flex-wrap items-center gap-3 px-3 py-2 text-left text-sm hover:bg-muted/40"
        onClick={() => setOpen((o) => !o)}
        aria-expanded={open}
      >
        {open ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
        <RunStatus run={run} />
        <span title={run.started_at}>{new Date(run.started_at).toLocaleString()}</span>
        <Badge variant="secondary" className="capitalize">
          {run.trigger_type}
        </Badge>
        <span className="ml-auto flex flex-wrap items-center gap-3 text-muted-foreground">
          {run.status !== "running" && <span>freed {formatBytes(run.freed_bytes)}</span>}
          <span>{formatDuration(run.duration_ms)}</span>
          <span>
            {run.finished_at ? `finished ${relativeTime(run.finished_at)}` : `started ${relativeTime(run.started_at)}`}
          </span>
        </span>
      </button>
      {open && (
        <pre className="max-h-80 overflow-auto whitespace-pre-wrap border-t bg-muted/30 p-3 font-mono text-xs">
          {run.log || "No output yet."}
        </pre>
      )}
    </div>
  );
}

function KeepRemoveSelect({
  id,
  value,
  onChange,
  keepLabel,
  removeLabel,
}: {
  id: string;
  value: boolean;
  onChange: (remove: boolean) => void;
  keepLabel: string;
  removeLabel: string;
}) {
  return (
    <Select value={value ? "remove" : "keep"} onValueChange={(v) => onChange(v === "remove")}>
      <SelectTrigger id={id} className="w-full sm:w-64">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="keep">{keepLabel}</SelectItem>
        <SelectItem value="remove">{removeLabel}</SelectItem>
      </SelectContent>
    </Select>
  );
}

export default function DockerCleanupPage() {
  const { serverId } = useServerContext();
  const queryClient = useQueryClient();
  const [form, setForm] = useState<CleanupForm | null>(null);
  const [confirmRun, setConfirmRun] = useState(false);

  const settingsQuery = useQuery({
    queryKey: ["server-maintenance", serverId],
    queryFn: () => serverOpsApi.getSettings(serverId),
  });

  const runsQuery = useQuery({
    queryKey: ["server-cleanup-runs", serverId],
    queryFn: () => serverOpsApi.listCleanupRuns(serverId),
    refetchInterval: (q) =>
      q.state.data?.some((r) => r.status === "running") ? 3000 : false,
  });

  useEffect(() => {
    if (settingsQuery.data) setForm(toForm(settingsQuery.data));
  }, [settingsQuery.data]);

  const saveMutation = useMutation({
    mutationFn: (data: CleanupForm) => serverOpsApi.updateSettings(serverId, data),
    onSuccess: (saved) => {
      toast.success("Cleanup settings saved");
      queryClient.setQueryData(["server-maintenance", serverId], saved);
    },
    onError: (e: Error) => toast.error(e.message || "Failed to save settings"),
  });

  const runMutation = useMutation({
    mutationFn: () => serverOpsApi.runCleanup(serverId),
    onSuccess: () => {
      toast.success("Cleanup started");
      setConfirmRun(false);
      queryClient.invalidateQueries({ queryKey: ["server-cleanup-runs", serverId] });
      queryClient.invalidateQueries({ queryKey: ["server-images", serverId] });
    },
    onError: (e: Error) => toast.error(e.message || "Failed to start cleanup"),
  });

  const saved = settingsQuery.data ? toForm(settingsQuery.data) : null;
  const dirty = !!form && !!saved && JSON.stringify(form) !== JSON.stringify(saved);
  const running = runsQuery.data?.some((r) => r.status === "running") ?? false;
  const set = <K extends keyof CleanupForm>(k: K, v: CleanupForm[K]) =>
    setForm((f) => (f ? { ...f, [k]: v } : f));

  if (settingsQuery.error) {
    return (
      <QueryError
        what="cleanup settings"
        error={settingsQuery.error}
        onRetry={() => settingsQuery.refetch()}
      />
    );
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Sparkles className="h-5 w-5" />
                Docker Cleanup
              </CardTitle>
              <CardDescription>
                Removes dangling and unused images and the build cache. Running containers and
                the images they use are never touched.
              </CardDescription>
            </div>
            <Button onClick={() => setConfirmRun(true)} disabled={running || runMutation.isPending}>
              {running ? <Loader2 className="h-4 w-4 animate-spin" /> : <Play className="h-4 w-4" />}
              {running ? "Cleanup running" : "Run Cleanup"}
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          <div className="flex items-start gap-2 rounded-md border bg-muted/40 p-3 text-sm text-muted-foreground">
            <Clock className="mt-0.5 h-4 w-4 shrink-0" />
            <p>
              Scheduled maintenance runs on the schedule below with the options saved on this page.
              Times are in UTC.
            </p>
          </div>
        </CardContent>
      </Card>

      {!form ? (
        <div className="flex justify-center py-8">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      ) : (
        <>
          <Card>
            <CardHeader>
              <CardTitle>Schedule</CardTitle>
            </CardHeader>
            <CardContent className="space-y-5">
              <div className="flex items-center justify-between gap-4">
                <div>
                  <Label htmlFor="cleanup-enabled">Scheduled cleanup</Label>
                  <p className="text-sm text-muted-foreground">Run cleanup automatically.</p>
                </div>
                <Switch
                  id="cleanup-enabled"
                  checked={form.cleanup_enabled}
                  onCheckedChange={(v) => set("cleanup_enabled", v)}
                />
              </div>
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="cleanup-cron">Cleanup frequency (cron)</Label>
                  <Input
                    id="cleanup-cron"
                    className="font-mono"
                    value={form.cleanup_cron}
                    onChange={(e) => set("cleanup_cron", e.target.value)}
                    placeholder="0 3 * * *"
                  />
                  <div className="flex flex-wrap gap-1">
                    {CRON_PRESETS.map((p) => (
                      <Button
                        key={p.value}
                        type="button"
                        variant="outline"
                        size="xs"
                        onClick={() => set("cleanup_cron", p.value)}
                      >
                        {p.label}
                      </Button>
                    ))}
                  </div>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="cleanup-trigger">Trigger</Label>
                  <Select
                    value={form.cleanup_trigger}
                    onValueChange={(v) => set("cleanup_trigger", v as CleanupForm["cleanup_trigger"])}
                  >
                    <SelectTrigger id="cleanup-trigger" className="w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="schedule">Run on every schedule</SelectItem>
                      <SelectItem value="threshold">Only when disk usage is above a threshold</SelectItem>
                    </SelectContent>
                  </Select>
                  {form.cleanup_trigger === "threshold" && (
                    <div className="flex items-center gap-2 pt-1">
                      <Input
                        type="number"
                        min={1}
                        max={99}
                        className="w-24"
                        aria-label="Disk usage threshold percent"
                        value={form.cleanup_threshold_pct}
                        onChange={(e) => set("cleanup_threshold_pct", Number(e.target.value))}
                      />
                      <span className="text-sm text-muted-foreground">% disk usage</span>
                    </div>
                  )}
                </div>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Advanced cleanup</CardTitle>
              <CardDescription className="flex items-center gap-1.5 text-amber-700 dark:text-amber-400">
                <AlertTriangle className="h-4 w-4" />
                These options can remove recoverable data.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-5">
              <div className="grid gap-2 sm:grid-cols-[1fr_auto] sm:items-center">
                <div>
                  <Label htmlFor="opt-volumes">Unused volumes</Label>
                  <p className="text-sm text-muted-foreground">
                    Volumes not attached to any container, including named volumes. Data in them is
                    deleted for good.
                  </p>
                </div>
                <KeepRemoveSelect
                  id="opt-volumes"
                  value={form.remove_unused_volumes}
                  onChange={(v) => set("remove_unused_volumes", v)}
                  keepLabel="Keep unused volumes"
                  removeLabel="Remove unused volumes"
                />
              </div>
              <div className="grid gap-2 sm:grid-cols-[1fr_auto] sm:items-center">
                <div>
                  <Label htmlFor="opt-networks">Unused networks</Label>
                  <p className="text-sm text-muted-foreground">
                    Networks with no containers. Built-in networks and Rivetr networks are kept.
                  </p>
                </div>
                <KeepRemoveSelect
                  id="opt-networks"
                  value={form.remove_unused_networks}
                  onChange={(v) => set("remove_unused_networks", v)}
                  keepLabel="Keep unused networks"
                  removeLabel="Remove unused networks"
                />
              </div>
              <div className="grid gap-2 sm:grid-cols-[1fr_auto] sm:items-center">
                <div>
                  <Label htmlFor="opt-images">Application images</Label>
                  <p className="text-sm text-muted-foreground">
                    Images of earlier deployments that Rivetr keeps for rollbacks.
                  </p>
                </div>
                <KeepRemoveSelect
                  id="opt-images"
                  value={form.remove_retained_images}
                  onChange={(v) => set("remove_retained_images", v)}
                  keepLabel="Keep retained images"
                  removeLabel="Remove retained images"
                />
              </div>
            </CardContent>
          </Card>

          <div className="flex justify-end gap-2">
            <Button
              variant="outline"
              disabled={!dirty || saveMutation.isPending}
              onClick={() => saved && setForm(saved)}
            >
              Reset
            </Button>
            <Button disabled={!dirty || saveMutation.isPending} onClick={() => saveMutation.mutate(form)}>
              {saveMutation.isPending ? <Loader2 className="h-4 w-4 animate-spin" /> : <Save className="h-4 w-4" />}
              Save
            </Button>
          </div>
        </>
      )}

      <Card>
        <CardHeader>
          <CardTitle>Recent executions</CardTitle>
        </CardHeader>
        <CardContent>
          {runsQuery.error ? (
            <QueryError what="cleanup runs" error={runsQuery.error} onRetry={() => runsQuery.refetch()} />
          ) : runsQuery.isLoading ? (
            <div className="flex justify-center py-6">
              <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
            </div>
          ) : (runsQuery.data ?? []).length === 0 ? (
            <p className="py-6 text-center text-sm text-muted-foreground">No cleanups have run yet.</p>
          ) : (
            <div className="space-y-2">
              {runsQuery.data!.map((run) => (
                <RunRow key={run.id} run={run} />
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      <ConfirmActionDialog
        open={confirmRun}
        onOpenChange={setConfirmRun}
        title="Run Docker cleanup now?"
        description={
          <>
            <p>
              Removes dangling images, images no container uses and the build cache
              {dirty ? ". Unsaved changes on this page are not used; save them first" : ""}.
            </p>
            {saved?.remove_unused_volumes && (
              <p className="font-medium text-destructive">
                Unused volumes will also be removed. Their data cannot be recovered.
              </p>
            )}
            {saved?.remove_unused_networks && <p>Unused networks will also be removed.</p>}
            {saved?.remove_retained_images && (
              <p className="text-amber-600 dark:text-amber-400">
                Rollback images will be removed too.
              </p>
            )}
          </>
        }
        confirmLabel="Run Cleanup"
        confirmText={saved?.remove_unused_volumes ? "remove volumes" : undefined}
        pending={runMutation.isPending}
        onConfirm={() => runMutation.mutate()}
      />
    </div>
  );
}
