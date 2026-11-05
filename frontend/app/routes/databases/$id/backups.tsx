import { useState } from "react";
import { useOutletContext } from "react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  Archive,
  CloudUpload,
  Calendar,
  Download,
  Loader2,
  Plus,
  Settings,
  Trash2,
  AlertCircle,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { api } from "@/lib/api";
import { s3Api } from "@/lib/api/s3";
import { backupsApi } from "@/lib/api/backups";
import { BackupCatalog } from "@/components/backup-catalog";
import type { BackupCatalogEntry } from "@/types/backups";
import type { S3StorageConfigResponse } from "@/types/api";
import type {
  ManagedDatabase,
  DatabaseBackupSchedule,
  ScheduleType,
} from "@/types/api";

export function meta() {
  return [
    { title: "Database Backups - Rivetr" },
    { name: "description", content: "Manage database backup schedules and restore points" },
  ];
}

// Format relative time
function formatRelativeTime(dateStr: string): string {
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffMins < 1) return "just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  return date.toLocaleDateString();
}

// Schedule description
function getScheduleDescription(schedule: DatabaseBackupSchedule): string {
  const hour = schedule.schedule_hour.toString().padStart(2, "0");
  const days = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

  switch (schedule.schedule_type) {
    case "hourly":
      return "Every hour";
    case "daily":
      return `Daily at ${hour}:00 UTC`;
    case "weekly":
      const day = schedule.schedule_day !== undefined ? days[schedule.schedule_day] : "Sunday";
      return `Every ${day} at ${hour}:00 UTC`;
    default:
      return schedule.schedule_type;
  }
}

export default function DatabaseBackupsPage() {
  const { database } = useOutletContext<{ database: ManagedDatabase }>();
  const queryClient = useQueryClient();
  const [showScheduleForm, setShowScheduleForm] = useState(false);

  // Schedule form state
  const [scheduleEnabled, setScheduleEnabled] = useState(true);
  const [scheduleType, setScheduleType] = useState<ScheduleType>("daily");
  const [scheduleHour, setScheduleHour] = useState(2);
  const [scheduleDay, setScheduleDay] = useState(0);
  const [retentionCount, setRetentionCount] = useState(5);
  const [scheduleS3ConfigId, setScheduleS3ConfigId] = useState<string>("__local__");
  const [uploadTarget, setUploadTarget] = useState<BackupCatalogEntry | null>(null);
  const [uploadConfigId, setUploadConfigId] = useState<string>("");
  const [deleteTarget, setDeleteTarget] = useState<BackupCatalogEntry | null>(null);

  const catalogKey = ["backup-catalog", "database", database.id];

  // S3 storage configs for offsite copies
  const { data: s3Configs = [] } = useQuery<S3StorageConfigResponse[]>({
    queryKey: ["s3-configs"],
    queryFn: () => s3Api.listConfigs(),
  });

  // Fetch schedule
  const { data: schedule, isLoading: scheduleLoading } = useQuery<DatabaseBackupSchedule | null>({
    queryKey: ["database-backup-schedule", database.id],
    queryFn: () => api.getDatabaseBackupSchedule(database.id),
  });

  // Create backup mutation
  const createBackupMutation = useMutation({
    mutationFn: () => api.createDatabaseBackup(database.id),
    onSuccess: () => {
      toast.success("Backup created");
      queryClient.invalidateQueries({ queryKey: catalogKey });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to start backup");
    },
  });

  // Delete backup mutation (local file or S3 object)
  const deleteBackupMutation = useMutation({
    mutationFn: (entry: BackupCatalogEntry) =>
      entry.source === "s3"
        ? s3Api.deleteBackup(entry.id).then(() => undefined)
        : api.deleteDatabaseBackup(database.id, entry.id).then(() => undefined),
    onSuccess: () => {
      toast.success("Backup deleted");
      queryClient.invalidateQueries({ queryKey: catalogKey });
      setDeleteTarget(null);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to delete backup");
    },
  });

  // Update schedule mutation
  const updateScheduleMutation = useMutation({
    mutationFn: () =>
      api.upsertDatabaseBackupSchedule(
        database.id,
        {
          enabled: scheduleEnabled,
          schedule_type: scheduleType,
          schedule_hour: scheduleHour,
          schedule_day: scheduleType === "weekly" ? scheduleDay : undefined,
          retention_count: retentionCount,
          s3_config_id: scheduleS3ConfigId === "__local__" ? "" : scheduleS3ConfigId,
        }
      ),
    onSuccess: () => {
      toast.success("Backup schedule updated");
      queryClient.invalidateQueries({ queryKey: ["database-backup-schedule", database.id] });
      setShowScheduleForm(false);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to update schedule");
    },
  });

  // Delete schedule mutation
  const deleteScheduleMutation = useMutation({
    mutationFn: () => api.deleteDatabaseBackupSchedule(database.id),
    onSuccess: () => {
      toast.success("Backup schedule removed");
      queryClient.invalidateQueries({ queryKey: ["database-backup-schedule", database.id] });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to remove schedule");
    },
  });

  // Download backup mutation
  const downloadBackupMutation = useMutation({
    mutationFn: (backupId: string) => api.downloadDatabaseBackup(database.id, backupId),
    onSuccess: () => {
      toast.success("Backup download started");
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to download backup");
    },
  });

  // Copy a local backup to S3
  const uploadMutation = useMutation({
    mutationFn: ({ backupId, configId }: { backupId: string; configId: string }) =>
      backupsApi.uploadDatabaseBackup(database.id, backupId, configId),
    onSuccess: () => {
      toast.success("Backup uploaded to S3");
      setUploadTarget(null);
      queryClient.invalidateQueries({ queryKey: catalogKey });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Upload failed");
    },
  });

  // Initialize form when editing schedule
  const handleEditSchedule = () => {
    if (schedule) {
      setScheduleEnabled(schedule.enabled);
      setScheduleType(schedule.schedule_type);
      setScheduleHour(schedule.schedule_hour);
      setScheduleDay(schedule.schedule_day ?? 0);
      setRetentionCount(schedule.retention_count);
      setScheduleS3ConfigId(schedule.s3_config_id ?? "__local__");
    }
    setShowScheduleForm(true);
  };

  const isDbRunning = database.status === "running";

  return (
    <div className="space-y-6">
      {/* Schedule Card */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Calendar className="h-5 w-5" />
                Backup Schedule
              </CardTitle>
              <CardDescription>
                Configure automatic backups for this database
              </CardDescription>
            </div>
            {!showScheduleForm && (
              <Button
                variant="outline"
                size="sm"
                onClick={handleEditSchedule}
              >
                <Settings className="h-4 w-4 mr-2" />
                {schedule ? "Edit Schedule" : "Set Up Schedule"}
              </Button>
            )}
          </div>
        </CardHeader>
        <CardContent>
          {scheduleLoading ? (
            <div className="flex items-center justify-center py-4">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : showScheduleForm ? (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <Label htmlFor="schedule-enabled">Enable automatic backups</Label>
                <Switch
                  id="schedule-enabled"
                  checked={scheduleEnabled}
                  onCheckedChange={setScheduleEnabled}
                />
              </div>

              <div className="grid gap-4 sm:grid-cols-2">
                <div className="space-y-2">
                  <Label>Frequency</Label>
                  <Select
                    value={scheduleType}
                    onValueChange={(v) => setScheduleType(v as ScheduleType)}
                  >
                    <SelectTrigger aria-label="Backup frequency">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="hourly">Hourly</SelectItem>
                      <SelectItem value="daily">Daily</SelectItem>
                      <SelectItem value="weekly">Weekly</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                {scheduleType !== "hourly" && (
                  <div className="space-y-2">
                    <Label>Time (UTC)</Label>
                    <Select
                      value={scheduleHour.toString()}
                      onValueChange={(v) => setScheduleHour(parseInt(v))}
                    >
                      <SelectTrigger aria-label="Backup time (UTC)">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {Array.from({ length: 24 }, (_, i) => (
                          <SelectItem key={i} value={i.toString()}>
                            {i.toString().padStart(2, "0")}:00
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                )}

                {scheduleType === "weekly" && (
                  <div className="space-y-2">
                    <Label>Day of Week</Label>
                    <Select
                      value={scheduleDay.toString()}
                      onValueChange={(v) => setScheduleDay(parseInt(v))}
                    >
                      <SelectTrigger aria-label="Backup day of week">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="0">Sunday</SelectItem>
                        <SelectItem value="1">Monday</SelectItem>
                        <SelectItem value="2">Tuesday</SelectItem>
                        <SelectItem value="3">Wednesday</SelectItem>
                        <SelectItem value="4">Thursday</SelectItem>
                        <SelectItem value="5">Friday</SelectItem>
                        <SelectItem value="6">Saturday</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                )}

                <div className="space-y-2">
                  <Label>Destination</Label>
                  <Select value={scheduleS3ConfigId} onValueChange={setScheduleS3ConfigId}>
                    <SelectTrigger aria-label="Backup destination">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="__local__">Local only</SelectItem>
                      {s3Configs.map((c) => (
                        <SelectItem key={c.id} value={c.id}>
                          Local + S3 ({c.name})
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>

                <div className="space-y-2">
                  <Label>Retention (keep last N backups)</Label>
                  <Select
                    value={retentionCount.toString()}
                    onValueChange={(v) => setRetentionCount(parseInt(v))}
                  >
                    <SelectTrigger aria-label="Backup retention count">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="3">3 backups</SelectItem>
                      <SelectItem value="5">5 backups</SelectItem>
                      <SelectItem value="7">7 backups</SelectItem>
                      <SelectItem value="10">10 backups</SelectItem>
                      <SelectItem value="14">14 backups</SelectItem>
                      <SelectItem value="30">30 backups</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>

              <div className="flex justify-end gap-2 pt-4">
                <Button
                  variant="outline"
                  onClick={() => setShowScheduleForm(false)}
                >
                  Cancel
                </Button>
                <Button
                  onClick={() => updateScheduleMutation.mutate()}
                  disabled={updateScheduleMutation.isPending}
                >
                  {updateScheduleMutation.isPending && (
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  )}
                  Save Schedule
                </Button>
              </div>
            </div>
          ) : schedule ? (
            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <div className="flex items-center gap-2">
                  <Badge variant={schedule.enabled ? "default" : "secondary"}>
                    {schedule.enabled ? "Active" : "Disabled"}
                  </Badge>
                  <span className="text-sm font-medium">
                    {getScheduleDescription(schedule)}
                  </span>
                </div>
                <p className="text-sm text-muted-foreground">
                  Keeping last {schedule.retention_count} backups
                  {schedule.s3_config_id && (
                    <>
                      {" "}• copied to S3 (
                      {s3Configs.find((c) => c.id === schedule.s3_config_id)?.name ?? "unknown config"})
                    </>
                  )}
                  {schedule.next_run_at && schedule.enabled && (
                    <> • Next run: {formatRelativeTime(schedule.next_run_at)}</>
                  )}
                </p>
              </div>
              <Button aria-label="Delete"
                variant="ghost"
                size="sm"
                className="text-destructive hover:text-destructive"
                onClick={() => deleteScheduleMutation.mutate()}
                disabled={deleteScheduleMutation.isPending}
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            </div>
          ) : (
            <div className="text-center py-4 text-muted-foreground">
              <Calendar className="h-8 w-8 mx-auto mb-2 opacity-50" />
              <p>No backup schedule configured</p>
              <p className="text-sm">Set up automatic backups to protect your data</p>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Backups List Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Archive className="h-5 w-5" />
            Backups
          </CardTitle>
          <CardDescription>
            Local and S3 backups of this database. Restoring takes a safety backup of the
            current data first.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {!isDbRunning && (
            <div className="flex items-center gap-2 p-3 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-md text-yellow-700 dark:text-yellow-300 text-sm">
              <AlertCircle className="h-4 w-4 flex-shrink-0" />
              <span>Database must be running to create or restore backups</span>
            </div>
          )}
          <BackupCatalog
            kind="database"
            resourceId={database.id}
            resourceName={database.name}
            toolbar={
              <Button
                size="sm"
                onClick={() => createBackupMutation.mutate()}
                disabled={!isDbRunning || createBackupMutation.isPending}
              >
                {createBackupMutation.isPending ? (
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                ) : (
                  <Plus className="h-4 w-4 mr-2" />
                )}
                Create Backup
              </Button>
            }
            renderActions={(entry) => (
              <>
                {entry.source === "local" && entry.status === "completed" && (
                  <>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-8 w-8"
                      onClick={() => downloadBackupMutation.mutate(entry.id)}
                      disabled={downloadBackupMutation.isPending}
                      title="Download backup"
                      aria-label="Download backup"
                    >
                      <Download className="h-4 w-4" />
                    </Button>
                    {s3Configs.length > 0 && (
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-8 w-8"
                        onClick={() => {
                          setUploadConfigId(s3Configs[0]?.id ?? "");
                          setUploadTarget(entry);
                        }}
                        title="Copy to S3"
                        aria-label="Copy to S3"
                      >
                        <CloudUpload className="h-4 w-4" />
                      </Button>
                    )}
                  </>
                )}
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8 text-destructive hover:text-destructive"
                  onClick={() => setDeleteTarget(entry)}
                  disabled={entry.status === "running" || entry.status === "uploading"}
                  title="Delete backup"
                  aria-label="Delete backup"
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </>
            )}
          />
        </CardContent>
      </Card>

      {/* Copy to S3 */}
      <AlertDialog open={!!uploadTarget} onOpenChange={(open) => !open && setUploadTarget(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Copy backup to S3</AlertDialogTitle>
            <AlertDialogDescription>
              The file is uploaded with its SHA-256 checksum, which is checked again on restore.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <Select value={uploadConfigId} onValueChange={setUploadConfigId}>
            <SelectTrigger aria-label="S3 storage">
              <SelectValue placeholder="Select S3 storage" />
            </SelectTrigger>
            <SelectContent>
              {s3Configs.map((c) => (
                <SelectItem key={c.id} value={c.id}>
                  {c.name} ({c.bucket})
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              disabled={!uploadConfigId || uploadMutation.isPending}
              onClick={(e) => {
                e.preventDefault();
                if (uploadTarget && uploadConfigId) {
                  uploadMutation.mutate({ backupId: uploadTarget.id, configId: uploadConfigId });
                }
              }}
            >
              {uploadMutation.isPending && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
              Upload
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Delete Confirmation Dialog */}
      <AlertDialog open={!!deleteTarget} onOpenChange={() => setDeleteTarget(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Backup</AlertDialogTitle>
            <AlertDialogDescription>
              {deleteTarget?.source === "s3"
                ? "The backup object is deleted from S3. This action cannot be undone."
                : "Are you sure you want to delete this backup? This action cannot be undone."}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              onClick={() => deleteTarget && deleteBackupMutation.mutate(deleteTarget)}
              disabled={deleteBackupMutation.isPending}
            >
              {deleteBackupMutation.isPending && (
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              )}
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
