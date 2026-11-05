import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Archive, Calendar, Loader2, Plus, Settings, Trash2 } from "lucide-react";

import { BackupCatalog, formatRelativeTime } from "@/components/backup-catalog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { api } from "@/lib/api";
import { backupsApi } from "@/lib/api/backups";
import { s3Api } from "@/lib/api/s3";
import type { S3StorageConfigResponse, Volume } from "@/types/api";
import type {
  BackupCatalogEntry,
  BackupSource,
  VolumeBackupSchedule,
  VolumeSnapshotMode,
} from "@/types/backups";

const CRON_PRESETS: { label: string; value: string }[] = [
  { label: "Daily at 03:00 UTC", value: "0 3 * * *" },
  { label: "Every 6 hours", value: "0 */6 * * *" },
  { label: "Hourly", value: "0 * * * *" },
  { label: "Weekly, Sunday 03:00 UTC", value: "0 3 * * 0" },
];

function destinationLabel(
  destination: BackupSource,
  configId: string | null | undefined,
  configs: S3StorageConfigResponse[],
): string {
  if (destination === "local") return "local disk";
  const name = configs.find((c) => c.id === configId)?.name;
  return `S3${name ? ` (${name})` : ""}`;
}

function VolumeSchedule({
  volume,
  s3Configs,
}: {
  volume: Volume;
  s3Configs: S3StorageConfigResponse[];
}) {
  const queryClient = useQueryClient();
  const key = ["volume-backup-schedule", volume.id];
  const [editing, setEditing] = useState(false);
  const [enabled, setEnabled] = useState(true);
  const [cron, setCron] = useState("0 3 * * *");
  const [retention, setRetention] = useState(7);
  const [destination, setDestination] = useState<string>("local");
  const [mode, setMode] = useState<VolumeSnapshotMode>("live");

  const { data: schedule, isLoading } = useQuery<VolumeBackupSchedule | null>({
    queryKey: key,
    queryFn: () => backupsApi.getVolumeSchedule(volume.id),
  });

  const startEditing = () => {
    if (schedule) {
      setEnabled(schedule.enabled !== 0);
      setCron(schedule.cron_expression);
      setRetention(schedule.retention_count);
      setDestination(
        schedule.destination === "s3" && schedule.s3_config_id ? schedule.s3_config_id : "local",
      );
      setMode(schedule.snapshot_mode);
    }
    setEditing(true);
  };

  const saveMutation = useMutation({
    mutationFn: () =>
      backupsApi.upsertVolumeSchedule(volume.id, {
        enabled,
        cron_expression: cron.trim(),
        retention_count: retention,
        destination: destination === "local" ? "local" : "s3",
        s3_config_id: destination === "local" ? null : destination,
        snapshot_mode: mode,
      }),
    onSuccess: () => {
      toast.success("Backup schedule saved");
      setEditing(false);
      queryClient.invalidateQueries({ queryKey: key });
    },
    onError: (e: Error) => toast.error(e.message || "Failed to save schedule"),
  });

  const deleteMutation = useMutation({
    mutationFn: () => backupsApi.deleteVolumeSchedule(volume.id),
    onSuccess: () => {
      toast.success("Backup schedule removed");
      queryClient.invalidateQueries({ queryKey: key });
    },
    onError: (e: Error) => toast.error(e.message || "Failed to remove schedule"),
  });

  if (isLoading) {
    return <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />;
  }

  if (editing) {
    return (
      <div className="space-y-4 rounded-md border p-4">
        <div className="flex items-center justify-between">
          <Label htmlFor={`vol-sched-enabled-${volume.id}`}>Enable scheduled backups</Label>
          <Switch
            id={`vol-sched-enabled-${volume.id}`}
            checked={enabled}
            onCheckedChange={setEnabled}
          />
        </div>
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="space-y-2">
            <Label htmlFor={`vol-sched-cron-${volume.id}`}>Schedule (cron, UTC)</Label>
            <Input
              id={`vol-sched-cron-${volume.id}`}
              value={cron}
              onChange={(e) => setCron(e.target.value)}
              placeholder="0 3 * * *"
              className="font-mono"
            />
            <div className="flex flex-wrap gap-1">
              {CRON_PRESETS.map((p) => (
                <Button
                  key={p.value}
                  type="button"
                  variant="outline"
                  size="sm"
                  className="h-6 px-2 text-xs"
                  onClick={() => setCron(p.value)}
                >
                  {p.label}
                </Button>
              ))}
            </div>
          </div>
          <div className="space-y-2">
            <Label>Destination</Label>
            <Select value={destination} onValueChange={setDestination}>
              <SelectTrigger aria-label="Backup destination">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="local">Local disk</SelectItem>
                {s3Configs.map((c) => (
                  <SelectItem key={c.id} value={c.id}>
                    S3 ({c.name})
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-2">
            <Label htmlFor={`vol-sched-retention-${volume.id}`}>Keep last N backups</Label>
            <Input
              id={`vol-sched-retention-${volume.id}`}
              type="number"
              min={1}
              max={365}
              value={retention}
              onChange={(e) => setRetention(Math.max(1, parseInt(e.target.value || "1", 10)))}
            />
          </div>
          <div className="space-y-2">
            <Label>Snapshot</Label>
            <Select value={mode} onValueChange={(v) => setMode(v as VolumeSnapshotMode)}>
              <SelectTrigger aria-label="Snapshot mode">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="live">Live (read-only copy, no downtime)</SelectItem>
                <SelectItem value="stop">Stop the app during the backup (consistent)</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>
        <div className="flex justify-end gap-2">
          <Button variant="outline" onClick={() => setEditing(false)}>
            Cancel
          </Button>
          <Button onClick={() => saveMutation.mutate()} disabled={saveMutation.isPending}>
            {saveMutation.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            Save schedule
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-wrap items-center justify-between gap-2">
      {schedule ? (
        <div className="space-y-1 text-sm">
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant={schedule.enabled ? "default" : "secondary"}>
              {schedule.enabled ? "Active" : "Disabled"}
            </Badge>
            <code className="text-xs">{schedule.cron_expression}</code>
            <span className="text-muted-foreground">
              to {destinationLabel(schedule.destination, schedule.s3_config_id, s3Configs)}
            </span>
          </div>
          <p className="text-muted-foreground">
            Keeping last {schedule.retention_count} •{" "}
            {schedule.snapshot_mode === "stop" ? "stops the app while copying" : "live copy"}
            {schedule.enabled && schedule.next_run_at && (
              <> • next run {new Date(schedule.next_run_at).toLocaleString()}</>
            )}
            {schedule.last_run_at && <> • last run {formatRelativeTime(schedule.last_run_at)}</>}
          </p>
        </div>
      ) : (
        <p className="text-sm text-muted-foreground">No backup schedule for this volume.</p>
      )}
      <div className="flex gap-1">
        <Button variant="outline" size="sm" onClick={startEditing}>
          <Settings className="mr-2 h-4 w-4" />
          {schedule ? "Edit schedule" : "Set up schedule"}
        </Button>
        {schedule && (
          <Button
            aria-label="Remove schedule"
            variant="ghost"
            size="sm"
            className="text-destructive hover:text-destructive"
            onClick={() => deleteMutation.mutate()}
            disabled={deleteMutation.isPending}
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        )}
      </div>
    </div>
  );
}

function VolumeBackupSection({
  volume,
  s3Configs,
}: {
  volume: Volume;
  s3Configs: S3StorageConfigResponse[];
}) {
  const queryClient = useQueryClient();
  const [destination, setDestination] = useState<string>("local");
  const [mode, setMode] = useState<VolumeSnapshotMode>("live");
  const catalogKey = ["backup-catalog", "volume", volume.id];

  const createMutation = useMutation({
    mutationFn: () =>
      backupsApi.createVolumeBackup(volume.id, {
        destination: destination === "local" ? "local" : "s3",
        s3_config_id: destination === "local" ? undefined : destination,
        snapshot_mode: mode,
      }),
    onSuccess: () => {
      toast.success("Backup started");
      queryClient.invalidateQueries({ queryKey: catalogKey });
    },
    onError: (e: Error) => toast.error(e.message || "Failed to start backup"),
  });

  const deleteMutation = useMutation({
    mutationFn: (entry: BackupCatalogEntry) =>
      backupsApi.deleteVolumeBackup(volume.id, entry.id, entry.source),
    onSuccess: () => {
      toast.success("Backup deleted");
      queryClient.invalidateQueries({ queryKey: catalogKey });
    },
    onError: (e: Error) => toast.error(e.message || "Failed to delete backup"),
  });

  return (
    <div className="space-y-4">
      <div className="space-y-2">
        <h4 className="flex items-center gap-2 text-sm font-medium">
          <Calendar className="h-4 w-4" />
          Schedule
        </h4>
        <VolumeSchedule volume={volume} s3Configs={s3Configs} />
      </div>
      <BackupCatalog
        kind="volume"
        resourceId={volume.id}
        resourceName={volume.name}
        restoreWarning="The app's containers are stopped while the volume is replaced and started again afterwards."
        toolbar={
          <>
            <Select value={destination} onValueChange={setDestination}>
              <SelectTrigger className="h-9 w-[170px]" aria-label="Backup destination">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="local">Local disk</SelectItem>
                {s3Configs.map((c) => (
                  <SelectItem key={c.id} value={c.id}>
                    S3 ({c.name})
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Select value={mode} onValueChange={(v) => setMode(v as VolumeSnapshotMode)}>
              <SelectTrigger className="h-9 w-[130px]" aria-label="Snapshot mode">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="live">Live copy</SelectItem>
                <SelectItem value="stop">Stop app</SelectItem>
              </SelectContent>
            </Select>
            <Button
              size="sm"
              onClick={() => createMutation.mutate()}
              disabled={createMutation.isPending}
            >
              {createMutation.isPending ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <Plus className="mr-2 h-4 w-4" />
              )}
              Back up now
            </Button>
          </>
        }
        renderActions={(entry) =>
          entry.status === "running" || entry.status === "pending" ? null : (
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8 text-destructive hover:text-destructive"
              onClick={() => deleteMutation.mutate(entry)}
              disabled={deleteMutation.isPending}
              title="Delete backup"
              aria-label="Delete backup"
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          )
        }
      />
    </div>
  );
}

/**
 * Volume backups for an app: per-volume schedule, manual backups to local disk or S3,
 * history and restore.
 */
export function VolumeBackupsCard({ appId }: { appId: string }) {
  const { data: volumes = [] } = useQuery<Volume[]>({
    queryKey: ["volumes", appId],
    queryFn: () => api.getVolumes(appId),
  });
  const { data: s3Configs = [] } = useQuery<S3StorageConfigResponse[]>({
    queryKey: ["s3-configs"],
    queryFn: () => s3Api.listConfigs(),
  });
  const [selected, setSelected] = useState<string>("");

  useEffect(() => {
    if (volumes.length > 0 && !volumes.some((v) => v.id === selected)) {
      setSelected(volumes[0].id);
    }
  }, [volumes, selected]);

  if (volumes.length === 0) return null;
  const volume = volumes.find((v) => v.id === selected) ?? volumes[0];

  return (
    <Card>
      <CardHeader>
        <div className="flex flex-wrap items-start justify-between gap-2">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Archive className="h-5 w-5" />
              Volume Backups
            </CardTitle>
            <CardDescription>
              Scheduled and manual backups to local disk or S3. Restores stop the app, take a
              safety backup, replace the volume content and start the app again.
            </CardDescription>
          </div>
          {volumes.length > 1 && (
            <Select value={volume.id} onValueChange={setSelected}>
              <SelectTrigger className="w-[200px]" aria-label="Volume">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {volumes.map((v) => (
                  <SelectItem key={v.id} value={v.id}>
                    {v.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
        </div>
      </CardHeader>
      <CardContent>
        <VolumeBackupSection key={volume.id} volume={volume} s3Configs={s3Configs} />
      </CardContent>
    </Card>
  );
}
