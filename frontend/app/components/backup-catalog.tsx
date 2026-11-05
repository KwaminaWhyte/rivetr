import { useState, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  CheckCircle,
  Clock,
  Cloud,
  HardDrive,
  Loader2,
  RefreshCw,
  RotateCcw,
  Search,
  XCircle,
} from "lucide-react";

import { ConfirmDialog } from "@/components/confirm-dialog";
import { QueryError } from "@/components/query-error";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { backupsApi, type BackupResourceKind } from "@/lib/api/backups";
import type { BackupCatalogEntry, RestoreJob } from "@/types/backups";

export function formatRelativeTime(dateStr: string): string {
  const date = new Date(dateStr);
  const diffMins = Math.floor((Date.now() - date.getTime()) / 60000);
  if (diffMins < 1) return "just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  const diffHours = Math.floor(diffMins / 60);
  if (diffHours < 24) return `${diffHours}h ago`;
  const diffDays = Math.floor(diffHours / 24);
  if (diffDays < 7) return `${diffDays}d ago`;
  return date.toLocaleDateString();
}

function formatDuration(ms: number | null): string {
  if (ms === null || ms === undefined) return "-";
  const seconds = Math.round(ms / 1000);
  if (seconds < 1) return `${ms} ms`;
  if (seconds < 60) return `${seconds}s`;
  return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
}

export function BackupStatusBadge({ status }: { status: string }) {
  switch (status) {
    case "completed":
      return (
        <Badge className="bg-green-500 text-white gap-1">
          <CheckCircle className="h-3 w-3" />
          Completed
        </Badge>
      );
    case "running":
    case "uploading":
      return (
        <Badge className="bg-blue-500 text-white gap-1">
          <Loader2 className="h-3 w-3 animate-spin" />
          {status === "uploading" ? "Uploading" : "Running"}
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

export function SourceBadge({ entry }: { entry: BackupCatalogEntry }) {
  if (entry.source === "s3") {
    return (
      <Badge variant="secondary" className="gap-1" title={entry.s3_key ?? undefined}>
        <Cloud className="h-3 w-3" />
        S3{entry.storage_config_name ? ` (${entry.storage_config_name})` : ""}
      </Badge>
    );
  }
  return (
    <Badge variant="outline" className="gap-1">
      <HardDrive className="h-3 w-3" />
      Local
    </Badge>
  );
}

function RestoreJobBanner({ job }: { job: RestoreJob }) {
  if (job.status === "running") {
    return (
      <div className="flex items-center gap-2 rounded-md border border-blue-200 bg-blue-50 p-3 text-sm text-blue-700 dark:border-blue-800 dark:bg-blue-900/20 dark:text-blue-300">
        <Loader2 className="h-4 w-4 flex-shrink-0 animate-spin" />
        <span>
          Restore from {job.source === "s3" ? "S3" : "a local backup"} in progress (started{" "}
          {formatRelativeTime(job.started_at)}). A safety backup of the current data is taken
          first.
        </span>
      </div>
    );
  }
  if (job.status === "failed") {
    return (
      <div className="flex items-start gap-2 rounded-md border border-red-200 bg-red-50 p-3 text-sm text-red-700 dark:border-red-800 dark:bg-red-900/20 dark:text-red-300">
        <XCircle className="mt-0.5 h-4 w-4 flex-shrink-0" />
        <span className="break-words">
          Last restore failed {job.completed_at ? formatRelativeTime(job.completed_at) : ""}:{" "}
          {job.error_message}
        </span>
      </div>
    );
  }
  return (
    <div className="flex items-start gap-2 rounded-md border border-green-200 bg-green-50 p-3 text-sm text-green-700 dark:border-green-800 dark:bg-green-900/20 dark:text-green-300">
      <CheckCircle className="mt-0.5 h-4 w-4 flex-shrink-0" />
      <span className="break-words">
        {job.message ?? "Restore completed"}
        {job.completed_at ? ` (${formatRelativeTime(job.completed_at)})` : ""}.
        {job.safety_backup_id && " The data it replaced is kept as a safety backup in the list."}
      </span>
    </div>
  );
}

export interface BackupCatalogProps {
  kind: BackupResourceKind;
  resourceId: string;
  /** Typed by the user to confirm a restore when two-step confirmation is on */
  resourceName: string;
  /** Extra per-row actions (download, upload, delete) */
  renderActions?: (entry: BackupCatalogEntry) => ReactNode;
  /** Shown above the table, e.g. a "Create backup" button */
  toolbar?: ReactNode;
  /** Extra warning in the restore dialog */
  restoreWarning?: string;
}

/**
 * Local and S3 backups of a database or volume in one table, with source badges and a
 * Restore button behind a two-step confirmation.
 */
export function BackupCatalog({
  kind,
  resourceId,
  resourceName,
  renderActions,
  toolbar,
  restoreWarning,
}: BackupCatalogProps) {
  const queryClient = useQueryClient();
  const [restoreTarget, setRestoreTarget] = useState<BackupCatalogEntry | null>(null);
  const catalogKey = ["backup-catalog", kind, resourceId];
  const restoresKey = ["backup-restores", kind, resourceId];

  const { data, isLoading, error, refetch } = useQuery({
    queryKey: catalogKey,
    queryFn: () => backupsApi.catalog(kind, resourceId),
    refetchInterval: 10000,
  });

  const { data: restores = [] } = useQuery({
    queryKey: restoresKey,
    queryFn: () => backupsApi.restores(kind, resourceId),
    refetchInterval: (query) =>
      query.state.data?.some((j) => j.status === "running") ? 3000 : 15000,
  });
  const latestRestore = restores[0];
  const restoreRunning = latestRestore?.status === "running";

  const scanMutation = useMutation({
    mutationFn: () => backupsApi.catalog(kind, resourceId, true),
    onSuccess: (res) => {
      queryClient.setQueryData(catalogKey, res);
      toast.success(
        res.imported > 0
          ? `Found ${res.imported} backup${res.imported === 1 ? "" : "s"} in S3`
          : "No new backups found in S3",
      );
    },
    onError: (e: Error) => toast.error(e.message || "S3 scan failed"),
  });

  const restoreMutation = useMutation({
    mutationFn: (entry: BackupCatalogEntry) =>
      backupsApi.restore(kind, resourceId, entry.id, entry.source),
    onSuccess: () => {
      toast.success("Restore started");
      setRestoreTarget(null);
      queryClient.invalidateQueries({ queryKey: restoresKey });
      queryClient.invalidateQueries({ queryKey: catalogKey });
    },
    onError: (e: Error) => toast.error(e.message || "Failed to start restore"),
  });

  const entries = data?.entries ?? [];

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-end gap-2">
        <Button
          variant="outline"
          size="sm"
          onClick={() => scanMutation.mutate()}
          disabled={scanMutation.isPending}
          title="List your S3 buckets for backups of this resource that are not shown yet"
        >
          {scanMutation.isPending ? (
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          ) : (
            <Search className="mr-2 h-4 w-4" />
          )}
          Scan S3
        </Button>
        <Button aria-label="Refresh" variant="outline" size="sm" onClick={() => refetch()}>
          <RefreshCw className="h-4 w-4" />
        </Button>
        {toolbar}
      </div>

      {latestRestore && <RestoreJobBanner job={latestRestore} />}

      {error ? (
        <QueryError what="backups" error={error} onRetry={() => refetch()} />
      ) : isLoading ? (
        <div className="flex items-center justify-center py-8">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      ) : entries.length === 0 ? (
        <div className="py-8 text-center text-muted-foreground">
          <HardDrive className="mx-auto mb-4 h-12 w-12 opacity-50" />
          <p className="font-medium">No backups yet</p>
          <p className="text-sm">Create a backup or set up a schedule.</p>
        </div>
      ) : (
        <div className="overflow-x-auto">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Created</TableHead>
                <TableHead>Source</TableHead>
                <TableHead>Trigger</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Size</TableHead>
                <TableHead>Duration</TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {entries.map((entry) => (
                <TableRow key={`${entry.source}-${entry.id}`}>
                  <TableCell>
                    <span className="font-medium">{formatRelativeTime(entry.created_at)}</span>
                    <p className="text-xs text-muted-foreground">
                      {new Date(entry.created_at).toLocaleString()}
                    </p>
                  </TableCell>
                  <TableCell>
                    <SourceBadge entry={entry} />
                  </TableCell>
                  <TableCell>
                    <Badge variant="outline" className="capitalize">
                      {(entry.trigger ?? "imported").replace("_", " ")}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <BackupStatusBadge status={entry.status} />
                    {entry.error_message && (
                      <p
                        className="mt-1 max-w-xs truncate text-xs text-destructive"
                        title={entry.error_message}
                      >
                        {entry.error_message}
                      </p>
                    )}
                  </TableCell>
                  <TableCell>{entry.size_human ?? "-"}</TableCell>
                  <TableCell>{formatDuration(entry.duration_ms)}</TableCell>
                  <TableCell className="text-right">
                    <div className="flex items-center justify-end gap-1">
                      {entry.status === "completed" && (
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-8"
                          onClick={() => setRestoreTarget(entry)}
                          disabled={restoreRunning}
                          title="Restore this backup"
                        >
                          <RotateCcw className="mr-1 h-4 w-4" />
                          Restore
                        </Button>
                      )}
                      {renderActions?.(entry)}
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}

      <ConfirmDialog
        open={!!restoreTarget}
        onOpenChange={(open) => !open && setRestoreTarget(null)}
        title={`Restore ${kind} "${resourceName}"?`}
        description={
          <span className="space-y-2">
            <span className="block">
              The current data is replaced with the backup from{" "}
              {restoreTarget ? new Date(restoreTarget.created_at).toLocaleString() : ""}
              {restoreTarget?.source === "s3"
                ? `, downloaded from S3${restoreTarget.storage_config_name ? ` (${restoreTarget.storage_config_name})` : ""} and checked against its size and checksum`
                : ""}
              . Anything written since then is lost.
            </span>
            <span className="block">
              A safety backup of the current data is taken first and appears in this list.
            </span>
            {restoreWarning && <span className="block font-medium">{restoreWarning}</span>}
          </span>
        }
        resourceName={resourceName}
        mode="two_step"
        confirmLabel="Restore"
        destructive
        isPending={restoreMutation.isPending}
        onConfirm={() => {
          if (restoreTarget) restoreMutation.mutate(restoreTarget);
        }}
      />
    </div>
  );
}
