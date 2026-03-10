import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { api } from "@/lib/api";
import { apiRequest } from "@/lib/api/core";
import type { BackupInfo, RestoreResult } from "@/types/api";
import {
  Download,
  Trash2,
  Upload,
  HardDrive,
  Shield,
  Clock,
  AlertTriangle,
  CheckCircle,
  Loader2,
  CloudUpload,
} from "lucide-react";
import { useState, useRef } from "react";
import { toast } from "sonner";

export function meta() {
  return [
    { title: "Backup & Restore - Rivetr" },
    {
      name: "description",
      content: "Backup and restore your Rivetr instance",
    },
  ];
}

function formatBytes(bytes: number): string {
  const KB = 1024;
  const MB = KB * 1024;
  const GB = MB * 1024;

  if (bytes >= GB) {
    return `${(bytes / GB).toFixed(2)} GB`;
  } else if (bytes >= MB) {
    return `${(bytes / MB).toFixed(2)} MB`;
  } else if (bytes >= KB) {
    return `${(bytes / KB).toFixed(2)} KB`;
  }
  return `${bytes} B`;
}

function formatDate(dateStr: string): string {
  if (!dateStr || dateStr === "unknown") return "Unknown";
  const date = new Date(dateStr);
  return date.toLocaleString();
}

export default function BackupPage() {
  const queryClient = useQueryClient();
  const [isCreating, setIsCreating] = useState(false);
  const [isRestoring, setIsRestoring] = useState(false);
  const [restoreResult, setRestoreResult] = useState<RestoreResult | null>(null);
  const [uploadingToS3, setUploadingToS3] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  // Fetch existing backups
  const {
    data: backups,
    isLoading,
  } = useQuery<BackupInfo[]>({
    queryKey: ["instance-backups"],
    queryFn: () => api.listBackups(),
  });

  // Delete backup mutation
  const deleteMutation = useMutation({
    mutationFn: (name: string) => api.deleteBackup(name),
    onSuccess: (_data, name) => {
      queryClient.invalidateQueries({ queryKey: ["instance-backups"] });
      toast.success(`Backup "${name}" deleted`);
    },
    onError: (error) => {
      toast.error("Failed to delete backup");
      console.error(error);
    },
  });

  // Create backup and download
  const handleCreateBackup = async () => {
    setIsCreating(true);
    try {
      const blob = await api.createBackup();
      // Trigger browser download
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      const timestamp = new Date()
        .toISOString()
        .replace(/[:.]/g, "-")
        .slice(0, 19);
      link.download = `rivetr-backup-${timestamp}.tar.gz`;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      URL.revokeObjectURL(url);

      // Refresh the backup list (server also stores it)
      queryClient.invalidateQueries({ queryKey: ["instance-backups"] });
      toast.success("Backup created and downloaded");
    } catch (error) {
      toast.error("Failed to create backup");
      console.error(error);
    } finally {
      setIsCreating(false);
    }
  };

  // Download an existing backup
  const handleDownloadBackup = async (name: string) => {
    try {
      const blob = await api.downloadBackup(name);
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = name;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      URL.revokeObjectURL(url);
      toast.success("Backup downloaded");
    } catch (error) {
      toast.error("Failed to download backup");
      console.error(error);
    }
  };

  // Upload a backup to S3
  const handleUploadToS3 = async (name: string) => {
    setUploadingToS3(name);
    try {
      await apiRequest(`/system/backups/${encodeURIComponent(name)}/upload-to-s3`, {
        method: "POST",
      });
      toast.success(`Backup "${name}" uploaded to S3`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to upload backup to S3");
    } finally {
      setUploadingToS3(null);
    }
  };

  // Handle restore file upload
  const handleRestore = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    // Reset file input
    if (fileInputRef.current) {
      fileInputRef.current.value = "";
    }

    setIsRestoring(true);
    setRestoreResult(null);
    try {
      const result = await api.restoreBackup(file);
      setRestoreResult(result);
      toast.success("Backup restored successfully. Please restart the server.");
    } catch (error) {
      toast.error("Failed to restore backup");
      console.error(error);
    } finally {
      setIsRestoring(false);
    }
  };

  return (
    <div className="space-y-6">
      <h1 className="text-3xl font-bold">Backup & Restore</h1>

      {/* Create Backup Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <HardDrive className="h-5 w-5" />
            Create Backup
          </CardTitle>
          <CardDescription>
            Create a full backup of your Rivetr instance including the database,
            configuration file, and SSL certificates.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-3 text-sm">
            <div className="flex items-center gap-2">
              <Shield className="h-4 w-4 text-muted-foreground" />
              <span>SQLite database (all apps, deployments, users, teams)</span>
            </div>
            <div className="flex items-center gap-2">
              <Shield className="h-4 w-4 text-muted-foreground" />
              <span>Configuration file (rivetr.toml)</span>
            </div>
            <div className="flex items-center gap-2">
              <Shield className="h-4 w-4 text-muted-foreground" />
              <span>SSL/ACME certificates (if configured)</span>
            </div>
          </div>

          <Button onClick={handleCreateBackup} disabled={isCreating}>
            {isCreating ? (
              <>
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                Creating Backup...
              </>
            ) : (
              <>
                <Download className="h-4 w-4 mr-2" />
                Create & Download Backup
              </>
            )}
          </Button>
        </CardContent>
      </Card>

      {/* Existing Backups Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Clock className="h-5 w-5" />
            Existing Backups
          </CardTitle>
          <CardDescription>
            Backups stored on the server in the data/backups/ directory.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="animate-pulse space-y-3">
              <div className="h-4 bg-muted rounded w-2/3"></div>
              <div className="h-4 bg-muted rounded w-1/2"></div>
            </div>
          ) : !backups || backups.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              No backups found. Create your first backup above.
            </p>
          ) : (
            <div className="space-y-3">
              {backups.map((backup) => (
                <div
                  key={backup.name}
                  className="flex items-center justify-between p-3 bg-muted/50 rounded-lg"
                >
                  <div className="space-y-1 min-w-0 flex-1">
                    <div className="font-mono text-sm truncate">
                      {backup.name}
                    </div>
                    <div className="flex items-center gap-3 text-xs text-muted-foreground">
                      <span>{formatBytes(backup.size)}</span>
                      <span>{formatDate(backup.created_at)}</span>
                    </div>
                  </div>
                  <div className="flex items-center gap-2 ml-4">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleDownloadBackup(backup.name)}
                    >
                      <Download className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleUploadToS3(backup.name)}
                      disabled={uploadingToS3 === backup.name}
                      title="Upload to S3"
                    >
                      {uploadingToS3 === backup.name ? (
                        <Loader2 className="h-4 w-4 animate-spin" />
                      ) : (
                        <CloudUpload className="h-4 w-4" />
                      )}
                    </Button>
                    <AlertDialog>
                      <AlertDialogTrigger asChild>
                        <Button
                          variant="outline"
                          size="sm"
                          className="text-destructive hover:text-destructive"
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </AlertDialogTrigger>
                      <AlertDialogContent>
                        <AlertDialogHeader>
                          <AlertDialogTitle>Delete Backup</AlertDialogTitle>
                          <AlertDialogDescription>
                            Are you sure you want to delete &quot;{backup.name}
                            &quot;? This action cannot be undone.
                          </AlertDialogDescription>
                        </AlertDialogHeader>
                        <AlertDialogFooter>
                          <AlertDialogCancel>Cancel</AlertDialogCancel>
                          <AlertDialogAction
                            onClick={() => deleteMutation.mutate(backup.name)}
                            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                          >
                            Delete
                          </AlertDialogAction>
                        </AlertDialogFooter>
                      </AlertDialogContent>
                    </AlertDialog>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Restore Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Upload className="h-5 w-5" />
            Restore from Backup
          </CardTitle>
          <CardDescription>
            Upload a previously created backup file to restore your Rivetr
            instance.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="p-4 bg-destructive/10 rounded-lg space-y-2">
            <div className="flex items-center gap-2 text-destructive font-medium">
              <AlertTriangle className="h-4 w-4" />
              Warning
            </div>
            <p className="text-sm text-destructive/80">
              Restoring from a backup will replace the current database,
              configuration, and SSL certificates. This action cannot be undone.
              A server restart is required after restoration.
            </p>
          </div>

          <input
            ref={fileInputRef}
            type="file"
            accept=".tar.gz,.gz"
            onChange={handleRestore}
            className="hidden"
          />

          <AlertDialog>
            <AlertDialogTrigger asChild>
              <Button variant="outline" disabled={isRestoring}>
                {isRestoring ? (
                  <>
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                    Restoring...
                  </>
                ) : (
                  <>
                    <Upload className="h-4 w-4 mr-2" />
                    Upload & Restore Backup
                  </>
                )}
              </Button>
            </AlertDialogTrigger>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>
                  Restore from Backup?
                </AlertDialogTitle>
                <AlertDialogDescription>
                  This will replace all current data with the backup contents.
                  The server will need to be restarted after the restore
                  completes. Are you sure you want to continue?
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>Cancel</AlertDialogCancel>
                <AlertDialogAction
                  onClick={() => fileInputRef.current?.click()}
                  className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                >
                  Yes, Restore
                </AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>

          {/* Restore Result */}
          {restoreResult && (
            <div className="p-4 bg-muted rounded-lg space-y-3">
              <div className="flex items-center gap-2 font-medium">
                <CheckCircle className="h-4 w-4 text-green-500" />
                Restore Completed
              </div>
              <div className="grid gap-2 text-sm">
                <div className="flex items-center gap-2">
                  <Badge
                    variant={
                      restoreResult.database_restored ? "default" : "secondary"
                    }
                  >
                    {restoreResult.database_restored
                      ? "Restored"
                      : "Not included"}
                  </Badge>
                  <span>Database</span>
                </div>
                <div className="flex items-center gap-2">
                  <Badge
                    variant={
                      restoreResult.config_restored ? "default" : "secondary"
                    }
                  >
                    {restoreResult.config_restored
                      ? "Restored"
                      : "Not included"}
                  </Badge>
                  <span>Configuration</span>
                </div>
                <div className="flex items-center gap-2">
                  <Badge
                    variant={
                      restoreResult.certs_restored ? "default" : "secondary"
                    }
                  >
                    {restoreResult.certs_restored
                      ? "Restored"
                      : "Not included"}
                  </Badge>
                  <span>SSL Certificates</span>
                </div>
              </div>
              {restoreResult.warnings.length > 0 && (
                <div className="space-y-1">
                  {restoreResult.warnings.map((warning, i) => (
                    <div
                      key={i}
                      className="flex items-center gap-2 text-sm text-amber-600 dark:text-amber-400"
                    >
                      <AlertTriangle className="h-3 w-3 shrink-0" />
                      {warning}
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </CardContent>
      </Card>

      {/* CLI Info Card */}
      <Card>
        <CardHeader>
          <CardTitle>CLI Usage</CardTitle>
          <CardDescription>
            You can also manage backups from the command line
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="p-4 bg-muted/50 rounded-md">
            <pre className="text-xs bg-background p-3 rounded overflow-x-auto">
              {`# Create a backup (saves to data/backups/)
rivetr backup

# Create a backup at a specific path
rivetr backup --output /path/to/backup.tar.gz

# Restore from a backup file
rivetr restore /path/to/backup.tar.gz`}
            </pre>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
