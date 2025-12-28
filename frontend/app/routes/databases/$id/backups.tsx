import { useState } from "react";
import { useOutletContext } from "react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  Archive,
  Calendar,
  Clock,
  Download,
  HardDrive,
  Loader2,
  Play,
  Plus,
  RefreshCw,
  Settings,
  Trash2,
  CheckCircle,
  XCircle,
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
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { api } from "@/lib/api";
import type {
  ManagedDatabase,
  DatabaseBackup,
  DatabaseBackupSchedule,
  ScheduleType,
} from "@/types/api";

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

// Format duration
function formatDuration(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;
  return `${mins}m ${secs}s`;
}

// Status badge for backups
function BackupStatusBadge({ status }: { status: string }) {
  switch (status) {
    case "completed":
      return (
        <Badge className="bg-green-500 text-white gap-1">
          <CheckCircle className="h-3 w-3" />
          Completed
        </Badge>
      );
    case "running":
      return (
        <Badge className="bg-blue-500 text-white gap-1">
          <Loader2 className="h-3 w-3 animate-spin" />
          Running
        </Badge>
      );
    case "failed":
      return (
        <Badge variant="destructive" className="gap-1">
          <XCircle className="h-3 w-3" />
          Failed
        </Badge>
      );
    case "pending":
      return (
        <Badge variant="outline" className="gap-1">
          <Clock className="h-3 w-3" />
          Pending
        </Badge>
      );
    default:
      return <Badge variant="outline">{status}</Badge>;
  }
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
  const { database, token } = useOutletContext<{ database: ManagedDatabase; token: string }>();
  const queryClient = useQueryClient();
  const [deleteBackupId, setDeleteBackupId] = useState<string | null>(null);
  const [showScheduleForm, setShowScheduleForm] = useState(false);

  // Schedule form state
  const [scheduleEnabled, setScheduleEnabled] = useState(true);
  const [scheduleType, setScheduleType] = useState<ScheduleType>("daily");
  const [scheduleHour, setScheduleHour] = useState(2);
  const [scheduleDay, setScheduleDay] = useState(0);
  const [retentionCount, setRetentionCount] = useState(5);

  // Fetch backups
  const { data: backups, isLoading: backupsLoading } = useQuery<DatabaseBackup[]>({
    queryKey: ["database-backups", database.id],
    queryFn: () => api.getDatabaseBackups(database.id, 50, token),
    refetchInterval: 10000, // Refresh every 10 seconds
  });

  // Fetch schedule
  const { data: schedule, isLoading: scheduleLoading } = useQuery<DatabaseBackupSchedule | null>({
    queryKey: ["database-backup-schedule", database.id],
    queryFn: () => api.getDatabaseBackupSchedule(database.id, token),
  });

  // Create backup mutation
  const createBackupMutation = useMutation({
    mutationFn: () => api.createDatabaseBackup(database.id, token),
    onSuccess: () => {
      toast.success("Backup started");
      queryClient.invalidateQueries({ queryKey: ["database-backups", database.id] });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to start backup");
    },
  });

  // Delete backup mutation
  const deleteBackupMutation = useMutation({
    mutationFn: (backupId: string) => api.deleteDatabaseBackup(database.id, backupId, token),
    onSuccess: () => {
      toast.success("Backup deleted");
      queryClient.invalidateQueries({ queryKey: ["database-backups", database.id] });
      setDeleteBackupId(null);
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
        },
        token
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
    mutationFn: () => api.deleteDatabaseBackupSchedule(database.id, token),
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
    mutationFn: (backupId: string) => api.downloadDatabaseBackup(database.id, backupId, token),
    onSuccess: () => {
      toast.success("Backup download started");
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to download backup");
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
                    <SelectTrigger>
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
                      <SelectTrigger>
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
                      <SelectTrigger>
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
                  <Label>Retention (keep last N backups)</Label>
                  <Select
                    value={retentionCount.toString()}
                    onValueChange={(v) => setRetentionCount(parseInt(v))}
                  >
                    <SelectTrigger>
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
                  {schedule.next_run_at && schedule.enabled && (
                    <> • Next run: {formatRelativeTime(schedule.next_run_at)}</>
                  )}
                </p>
              </div>
              <Button
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
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Archive className="h-5 w-5" />
                Backups
              </CardTitle>
              <CardDescription>
                View and manage database backups
              </CardDescription>
            </div>
            <div className="flex gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => queryClient.invalidateQueries({ queryKey: ["database-backups", database.id] })}
              >
                <RefreshCw className="h-4 w-4" />
              </Button>
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
            </div>
          </div>
        </CardHeader>
        <CardContent>
          {!isDbRunning && (
            <div className="flex items-center gap-2 p-3 mb-4 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-md text-yellow-700 dark:text-yellow-300 text-sm">
              <AlertCircle className="h-4 w-4 flex-shrink-0" />
              <span>Database must be running to create backups</span>
            </div>
          )}

          {backupsLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : !backups || backups.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <HardDrive className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p className="font-medium">No backups yet</p>
              <p className="text-sm">
                Create a manual backup or set up automatic backups
              </p>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Created</TableHead>
                  <TableHead>Type</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Size</TableHead>
                  <TableHead>Duration</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {backups.map((backup) => (
                  <TableRow key={backup.id}>
                    <TableCell>
                      <div>
                        <span className="font-medium">
                          {formatRelativeTime(backup.created_at)}
                        </span>
                        <p className="text-xs text-muted-foreground">
                          {new Date(backup.created_at).toLocaleString()}
                        </p>
                      </div>
                    </TableCell>
                    <TableCell>
                      <Badge variant="outline" className="capitalize">
                        {backup.backup_type}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <BackupStatusBadge status={backup.status} />
                      {backup.error_message && (
                        <p className="text-xs text-destructive mt-1 max-w-xs truncate">
                          {backup.error_message}
                        </p>
                      )}
                    </TableCell>
                    <TableCell>
                      {backup.file_size_human || "-"}
                    </TableCell>
                    <TableCell>
                      {backup.duration_seconds !== undefined
                        ? formatDuration(backup.duration_seconds)
                        : "-"}
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="flex items-center justify-end gap-1">
                        {backup.status === "completed" && (
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8"
                            onClick={() => downloadBackupMutation.mutate(backup.id)}
                            disabled={downloadBackupMutation.isPending}
                            title="Download backup"
                          >
                            {downloadBackupMutation.isPending ? (
                              <Loader2 className="h-4 w-4 animate-spin" />
                            ) : (
                              <Download className="h-4 w-4" />
                            )}
                          </Button>
                        )}
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8 text-destructive hover:text-destructive"
                          onClick={() => setDeleteBackupId(backup.id)}
                          disabled={backup.status === "running"}
                          title="Delete backup"
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Delete Confirmation Dialog */}
      <AlertDialog open={!!deleteBackupId} onOpenChange={() => setDeleteBackupId(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Backup</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete this backup? This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              onClick={() => deleteBackupId && deleteBackupMutation.mutate(deleteBackupId)}
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
