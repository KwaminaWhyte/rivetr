import { useState } from "react";
import { Link } from "react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { RotateCw, Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { api } from "@/lib/api";
import type { ScheduledRestart, ScheduledRestartRun } from "@/types/api";

/** 5-field cron presets (minute hour day month weekday, UTC). */
const PRESETS = [
  { label: "Daily 03:00", value: "0 3 * * *" },
  { label: "Every 6 hours", value: "0 */6 * * *" },
  { label: "Sundays 04:00", value: "0 4 * * 0" },
];

function when(value: string | null): string {
  if (!value) return "Not scheduled";
  const d = new Date(value);
  return Number.isNaN(d.getTime()) ? value : d.toLocaleString();
}

function runVariant(
  status: ScheduledRestartRun["status"],
): "default" | "secondary" | "destructive" | "outline" {
  switch (status) {
    case "succeeded":
      return "default";
    case "failed":
      return "destructive";
    case "skipped":
      return "secondary";
    default:
      return "outline";
  }
}

/**
 * Scheduled restarts for an app: restart the app's containers on a cron
 * schedule through the normal restart path (zero downtime where supported).
 */
export function ScheduledRestartsCard({ appId }: { appId: string }) {
  const queryClient = useQueryClient();
  const [cron, setCron] = useState("0 3 * * *");

  const schedules = useQuery<ScheduledRestart[]>({
    queryKey: ["scheduled-restarts", appId],
    queryFn: () => api.getScheduledRestarts(appId),
    refetchInterval: 60000,
  });
  const runs = useQuery<ScheduledRestartRun[]>({
    queryKey: ["scheduled-restart-runs", appId],
    queryFn: () => api.getScheduledRestartRuns(appId, 20),
    refetchInterval: 60000,
  });

  const refresh = () => {
    queryClient.invalidateQueries({ queryKey: ["scheduled-restarts", appId] });
    queryClient.invalidateQueries({ queryKey: ["scheduled-restart-runs", appId] });
  };

  const create = useMutation({
    mutationFn: () => api.createScheduledRestart(appId, { cron_expression: cron.trim(), enabled: true }),
    onSuccess: () => {
      toast.success("Restart schedule added");
      refresh();
    },
    onError: () => toast.error("Invalid cron expression"),
  });
  const toggle = useMutation({
    mutationFn: (s: ScheduledRestart) =>
      api.updateScheduledRestart(appId, s.id, { enabled: !s.enabled }),
    onSuccess: refresh,
    onError: (e: Error) => toast.error(e.message || "Failed to update schedule"),
  });
  const remove = useMutation({
    mutationFn: (id: string) => api.deleteScheduledRestart(appId, id),
    onSuccess: () => {
      toast.success("Restart schedule deleted");
      refresh();
    },
    onError: (e: Error) => toast.error(e.message || "Failed to delete schedule"),
  });

  const list = schedules.data ?? [];
  const history = runs.data ?? [];

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <RotateCw className="h-5 w-5" />
          Scheduled restarts
        </CardTitle>
        <CardDescription>
          Restart the app&apos;s containers on a cron schedule (UTC), using the same zero-downtime
          restart as the Restart button. Stopped apps are skipped. If Rivetr was down at a
          scheduled time, one missed run is caught up on startup (within 24 hours). Failures are
          sent to channels subscribed to &quot;deployment failed&quot;.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        <form
          className="flex flex-col gap-2 sm:flex-row sm:items-end"
          onSubmit={(e) => {
            e.preventDefault();
            if (cron.trim()) create.mutate();
          }}
        >
          <div className="flex-1 space-y-1.5">
            <Label htmlFor="restart-cron">Cron expression</Label>
            <Input
              id="restart-cron"
              value={cron}
              onChange={(e) => setCron(e.target.value)}
              placeholder="0 3 * * *"
              className="font-mono"
            />
            <div className="flex flex-wrap gap-1.5">
              {PRESETS.map((p) => (
                <Button
                  key={p.value}
                  type="button"
                  size="sm"
                  variant="outline"
                  className="h-7 text-xs"
                  onClick={() => setCron(p.value)}
                >
                  {p.label}
                </Button>
              ))}
            </div>
          </div>
          <Button type="submit" className="gap-2" disabled={create.isPending || !cron.trim()}>
            <Plus className="h-4 w-4" />
            Add schedule
          </Button>
        </form>

        {list.length === 0 ? (
          <p className="text-sm text-muted-foreground">No restart schedules yet.</p>
        ) : (
          <div className="space-y-2">
            {list.map((s) => (
              <div
                key={s.id}
                className="flex flex-col gap-2 rounded-md border p-3 sm:flex-row sm:items-center sm:justify-between"
              >
                <div className="space-y-0.5">
                  <p className="font-mono text-sm">{s.cron_expression}</p>
                  <p className="text-xs text-muted-foreground">
                    Next: {s.enabled ? when(s.next_restart) : "Paused"} · Last:{" "}
                    {s.last_restart ? when(s.last_restart) : "Never"}
                  </p>
                </div>
                <div className="flex items-center gap-3">
                  <Switch
                    aria-label="Enable schedule"
                    checked={s.enabled}
                    onCheckedChange={() => toggle.mutate(s)}
                  />
                  <Button
                    aria-label="Delete restart schedule"
                    variant="ghost"
                    size="sm"
                    className="text-destructive hover:text-destructive"
                    onClick={() => remove.mutate(s.id)}
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </Button>
                </div>
              </div>
            ))}
          </div>
        )}

        <div className="space-y-2">
          <h3 className="text-sm font-medium">Recent runs</h3>
          {history.length === 0 ? (
            <p className="text-sm text-muted-foreground">No runs yet.</p>
          ) : (
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Status</TableHead>
                    <TableHead>Scheduled for</TableHead>
                    <TableHead>Ran at</TableHead>
                    <TableHead>Details</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {history.map((r) => (
                    <TableRow key={r.id}>
                      <TableCell>
                        <div className="flex items-center gap-1.5">
                          <Badge variant={runVariant(r.status)}>{r.status}</Badge>
                          {r.catch_up ? (
                            <Badge variant="outline" className="text-xs">
                              catch-up
                            </Badge>
                          ) : null}
                        </div>
                      </TableCell>
                      <TableCell className="text-sm">{when(r.scheduled_for)}</TableCell>
                      <TableCell className="text-sm">{when(r.started_at)}</TableCell>
                      <TableCell className="max-w-xs text-sm">
                        {r.error_message ? (
                          <span className="text-muted-foreground">{r.error_message}</span>
                        ) : r.deployment_id ? (
                          <Link
                            className="underline underline-offset-2"
                            to={`/apps/${appId}/deployments/${r.deployment_id}`}
                          >
                            View restart
                          </Link>
                        ) : (
                          <span className="text-muted-foreground">-</span>
                        )}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
