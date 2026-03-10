import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
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
import { s3Api } from "@/lib/api/s3";
import type {
  S3StorageConfigResponse,
  S3BackupResponse,
  CreateS3StorageConfigRequest,
  S3BackupType,
  S3TestConnectionResult,
} from "@/types/api";
import {
  Cloud,
  Plus,
  Trash2,
  Edit,
  CheckCircle,
  XCircle,
  Loader2,
  Upload,
  Download,
  RefreshCw,
  HardDrive,
  Database,
  FolderArchive,
  Wifi,
} from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

export function meta() {
  return [
    { title: "S3 Storage - Rivetr" },
    {
      name: "description",
      content: "Manage S3 storage configurations and remote backups",
    },
  ];
}

function formatDate(dateStr: string): string {
  if (!dateStr || dateStr === "unknown") return "Unknown";
  const date = new Date(dateStr);
  return date.toLocaleString();
}

function BackupTypeIcon({ type }: { type: S3BackupType }) {
  switch (type) {
    case "instance":
      return <HardDrive className="h-4 w-4" />;
    case "database":
      return <Database className="h-4 w-4" />;
    case "volume":
      return <FolderArchive className="h-4 w-4" />;
  }
}

function StatusBadge({ status }: { status: string }) {
  switch (status) {
    case "completed":
      return (
        <Badge variant="default" className="bg-green-600">
          Completed
        </Badge>
      );
    case "uploading":
      return <Badge variant="secondary">Uploading</Badge>;
    case "pending":
      return <Badge variant="outline">Pending</Badge>;
    case "failed":
      return <Badge variant="destructive">Failed</Badge>;
    default:
      return <Badge variant="outline">{status}</Badge>;
  }
}

// ---------------------------------------------------------------------------
// S3 Config Form Dialog
// ---------------------------------------------------------------------------

function ConfigFormDialog({
  existingConfig,
  onClose,
}: {
  existingConfig?: S3StorageConfigResponse;
  onClose: () => void;
}) {
  const queryClient = useQueryClient();
  const [isOpen, setIsOpen] = useState(!!existingConfig);
  const [name, setName] = useState(existingConfig?.name ?? "");
  const [endpoint, setEndpoint] = useState(existingConfig?.endpoint ?? "");
  const [bucket, setBucket] = useState(existingConfig?.bucket ?? "");
  const [region, setRegion] = useState(existingConfig?.region ?? "us-east-1");
  const [accessKey, setAccessKey] = useState("");
  const [secretKey, setSecretKey] = useState("");
  const [pathPrefix, setPathPrefix] = useState(
    existingConfig?.path_prefix ?? ""
  );
  const [isDefault, setIsDefault] = useState(
    existingConfig?.is_default ?? false
  );

  const createMutation = useMutation({
    mutationFn: (config: CreateS3StorageConfigRequest) =>
      s3Api.createConfig(config),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["s3-configs"] });
      toast.success("S3 configuration created");
      handleClose();
    },
    onError: (error) => {
      toast.error(`Failed to create configuration: ${error.message}`);
    },
  });

  const updateMutation = useMutation({
    mutationFn: (config: Record<string, unknown>) =>
      s3Api.updateConfig(existingConfig!.id, config),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["s3-configs"] });
      toast.success("S3 configuration updated");
      handleClose();
    },
    onError: (error) => {
      toast.error(`Failed to update configuration: ${error.message}`);
    },
  });

  const handleClose = () => {
    setIsOpen(false);
    onClose();
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    if (!name || !bucket || !region) {
      toast.error("Name, bucket, and region are required");
      return;
    }

    if (existingConfig) {
      const update: Record<string, unknown> = {
        name,
        bucket,
        region,
        path_prefix: pathPrefix || undefined,
        is_default: isDefault,
        endpoint: endpoint || undefined,
      };
      if (accessKey) update.access_key = accessKey;
      if (secretKey) update.secret_key = secretKey;
      updateMutation.mutate(update);
    } else {
      if (!accessKey || !secretKey) {
        toast.error("Access key and secret key are required for new configs");
        return;
      }
      createMutation.mutate({
        name,
        endpoint: endpoint || undefined,
        bucket,
        region,
        access_key: accessKey,
        secret_key: secretKey,
        path_prefix: pathPrefix || undefined,
        is_default: isDefault,
      });
    }
  };

  const isLoading = createMutation.isPending || updateMutation.isPending;

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && handleClose()}>
      {!existingConfig && (
        <DialogTrigger asChild>
          <Button onClick={() => setIsOpen(true)}>
            <Plus className="h-4 w-4 mr-2" />
            Add S3 Config
          </Button>
        </DialogTrigger>
      )}
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>
            {existingConfig
              ? "Edit S3 Configuration"
              : "Add S3 Configuration"}
          </DialogTitle>
          <DialogDescription>
            Configure an S3-compatible storage endpoint for remote backups.
            Supports AWS S3, MinIO, Cloudflare R2, and more.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="config-name">Name</Label>
            <Input
              id="config-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="My S3 Backup"
              required
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="config-endpoint">
              Endpoint URL (optional, for MinIO/R2)
            </Label>
            <Input
              id="config-endpoint"
              value={endpoint}
              onChange={(e) => setEndpoint(e.target.value)}
              placeholder="https://minio.example.com"
            />
            <p className="text-xs text-muted-foreground">
              Leave empty for AWS S3. Set for MinIO, Cloudflare R2, or other
              S3-compatible providers.
            </p>
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="config-bucket">Bucket</Label>
              <Input
                id="config-bucket"
                value={bucket}
                onChange={(e) => setBucket(e.target.value)}
                placeholder="my-backups"
                required
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="config-region">Region</Label>
              <Input
                id="config-region"
                value={region}
                onChange={(e) => setRegion(e.target.value)}
                placeholder="us-east-1"
                required
              />
            </div>
          </div>
          <div className="space-y-2">
            <Label htmlFor="config-access-key">Access Key</Label>
            <Input
              id="config-access-key"
              type="password"
              value={accessKey}
              onChange={(e) => setAccessKey(e.target.value)}
              placeholder={
                existingConfig
                  ? "Leave empty to keep current"
                  : "AKIA..."
              }
              required={!existingConfig}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="config-secret-key">Secret Key</Label>
            <Input
              id="config-secret-key"
              type="password"
              value={secretKey}
              onChange={(e) => setSecretKey(e.target.value)}
              placeholder={
                existingConfig
                  ? "Leave empty to keep current"
                  : "Secret key..."
              }
              required={!existingConfig}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="config-prefix">Path Prefix (optional)</Label>
            <Input
              id="config-prefix"
              value={pathPrefix}
              onChange={(e) => setPathPrefix(e.target.value)}
              placeholder="rivetr/backups"
            />
            <p className="text-xs text-muted-foreground">
              All backup keys will be prefixed with this path.
            </p>
          </div>
          <div className="flex items-center gap-2">
            <input
              type="checkbox"
              id="config-default"
              checked={isDefault}
              onChange={(e) => setIsDefault(e.target.checked)}
              className="rounded border-gray-300"
            />
            <Label htmlFor="config-default">
              Set as default storage configuration
            </Label>
          </div>
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={handleClose}
              disabled={isLoading}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={isLoading}>
              {isLoading ? (
                <>
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  Saving...
                </>
              ) : existingConfig ? (
                "Update"
              ) : (
                "Create"
              )}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

// ---------------------------------------------------------------------------
// Trigger Backup Dialog
// ---------------------------------------------------------------------------

function TriggerBackupDialog({
  configs,
}: {
  configs: S3StorageConfigResponse[];
}) {
  const queryClient = useQueryClient();
  const [isOpen, setIsOpen] = useState(false);
  const [configId, setConfigId] = useState(
    configs.find((c) => c.is_default)?.id ?? configs[0]?.id ?? ""
  );
  const [backupType, setBackupType] = useState<S3BackupType>("instance");

  const triggerMutation = useMutation({
    mutationFn: () =>
      s3Api.triggerBackup({
        storage_config_id: configId,
        backup_type: backupType,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["s3-backups"] });
      toast.success("Backup triggered. It will upload in the background.");
      setIsOpen(false);
    },
    onError: (error) => {
      toast.error(`Failed to trigger backup: ${error.message}`);
    },
  });

  if (configs.length === 0) return null;

  return (
    <Dialog open={isOpen} onOpenChange={setIsOpen}>
      <DialogTrigger asChild>
        <Button>
          <Upload className="h-4 w-4 mr-2" />
          Backup to S3
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Trigger S3 Backup</DialogTitle>
          <DialogDescription>
            Create a backup and upload it to your configured S3 storage.
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4">
          <div className="space-y-2">
            <Label>Storage Configuration</Label>
            <Select value={configId} onValueChange={setConfigId}>
              <SelectTrigger>
                <SelectValue placeholder="Select storage config" />
              </SelectTrigger>
              <SelectContent>
                {configs.map((config) => (
                  <SelectItem key={config.id} value={config.id}>
                    {config.name}{" "}
                    {config.is_default && (
                      <span className="text-muted-foreground">(default)</span>
                    )}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-2">
            <Label>Backup Type</Label>
            <Select
              value={backupType}
              onValueChange={(v) => setBackupType(v as S3BackupType)}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="instance">
                  Instance (DB + Config + Certs)
                </SelectItem>
                <SelectItem value="database" disabled>
                  Database (coming soon)
                </SelectItem>
                <SelectItem value="volume" disabled>
                  Volume (coming soon)
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>
        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => setIsOpen(false)}
            disabled={triggerMutation.isPending}
          >
            Cancel
          </Button>
          <Button
            onClick={() => triggerMutation.mutate()}
            disabled={triggerMutation.isPending || !configId}
          >
            {triggerMutation.isPending ? (
              <>
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                Starting...
              </>
            ) : (
              <>
                <Upload className="h-4 w-4 mr-2" />
                Start Backup
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ---------------------------------------------------------------------------
// Main Page
// ---------------------------------------------------------------------------

export default function S3StoragePage() {
  const queryClient = useQueryClient();
  const [editingConfig, setEditingConfig] =
    useState<S3StorageConfigResponse | null>(null);
  const [testingId, setTestingId] = useState<string | null>(null);

  // Fetch S3 configs
  const { data: configs, isLoading: configsLoading } = useQuery<
    S3StorageConfigResponse[]
  >({
    queryKey: ["s3-configs"],
    queryFn: () => s3Api.listConfigs(),
  });

  // Fetch S3 backups
  const { data: backups, isLoading: backupsLoading } = useQuery<
    S3BackupResponse[]
  >({
    queryKey: ["s3-backups"],
    queryFn: () => s3Api.listBackups(),
  });

  // Delete config mutation
  const deleteConfigMutation = useMutation({
    mutationFn: (id: string) => s3Api.deleteConfig(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["s3-configs"] });
      toast.success("S3 configuration deleted");
    },
    onError: (error) => {
      if (error.message.includes("409")) {
        toast.error(
          "Cannot delete: this configuration has backups. Delete them first."
        );
      } else {
        toast.error(`Failed to delete configuration: ${error.message}`);
      }
    },
  });

  // Test connection
  const handleTestConnection = async (id: string) => {
    setTestingId(id);
    try {
      const result: S3TestConnectionResult = await s3Api.testConfig(id);
      if (result.success) {
        toast.success("Connection successful");
      } else {
        toast.error(`Connection failed: ${result.message}`);
      }
    } catch (error) {
      toast.error(
        `Test failed: ${error instanceof Error ? error.message : "Unknown error"}`
      );
    } finally {
      setTestingId(null);
    }
  };

  // Restore from S3 backup
  const restoreMutation = useMutation({
    mutationFn: (id: string) => s3Api.restoreBackup(id),
    onSuccess: (data) => {
      toast.success(
        data.message || "Restore completed. Server restart recommended."
      );
    },
    onError: (error) => {
      toast.error(`Restore failed: ${error.message}`);
    },
  });

  // Delete S3 backup
  const deleteBackupMutation = useMutation({
    mutationFn: (id: string) => s3Api.deleteBackup(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["s3-backups"] });
      toast.success("S3 backup deleted");
    },
    onError: (error) => {
      toast.error(`Failed to delete backup: ${error.message}`);
    },
  });

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">S3 Storage</h1>
        <div className="flex items-center gap-2">
          <TriggerBackupDialog configs={configs ?? []} />
          <ConfigFormDialog
            key="new-config"
            onClose={() => {}}
          />
        </div>
      </div>

      {/* S3 Storage Configurations */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Cloud className="h-5 w-5" />
            Storage Configurations
          </CardTitle>
          <CardDescription>
            Configure S3-compatible storage endpoints for remote backups. Supports
            AWS S3, MinIO, Cloudflare R2, and more.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {configsLoading ? (
            <div className="animate-pulse space-y-3">
              <div className="h-4 bg-muted rounded w-2/3"></div>
              <div className="h-4 bg-muted rounded w-1/2"></div>
            </div>
          ) : !configs || configs.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              No S3 storage configurations yet. Add one to start backing up to
              remote storage.
            </p>
          ) : (
            <div className="space-y-3">
              {configs.map((config) => (
                <div
                  key={config.id}
                  className="flex items-center justify-between p-4 bg-muted/50 rounded-lg"
                >
                  <div className="space-y-1 min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="font-medium">{config.name}</span>
                      {config.is_default && (
                        <Badge variant="secondary">Default</Badge>
                      )}
                    </div>
                    <div className="flex items-center gap-3 text-xs text-muted-foreground">
                      <span>
                        {config.endpoint
                          ? new URL(config.endpoint).hostname
                          : "AWS S3"}
                      </span>
                      <span>{config.bucket}</span>
                      <span>{config.region}</span>
                      {config.path_prefix && (
                        <span>/{config.path_prefix}</span>
                      )}
                    </div>
                  </div>
                  <div className="flex items-center gap-2 ml-4">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleTestConnection(config.id)}
                      disabled={testingId === config.id}
                    >
                      {testingId === config.id ? (
                        <Loader2 className="h-4 w-4 animate-spin" />
                      ) : (
                        <Wifi className="h-4 w-4" />
                      )}
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setEditingConfig(config)}
                    >
                      <Edit className="h-4 w-4" />
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
                          <AlertDialogTitle>Delete Configuration</AlertDialogTitle>
                          <AlertDialogDescription>
                            Are you sure you want to delete &quot;{config.name}
                            &quot;? This will not delete any backups stored in S3.
                          </AlertDialogDescription>
                        </AlertDialogHeader>
                        <AlertDialogFooter>
                          <AlertDialogCancel>Cancel</AlertDialogCancel>
                          <AlertDialogAction
                            onClick={() =>
                              deleteConfigMutation.mutate(config.id)
                            }
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

      {/* Edit config dialog */}
      {editingConfig && (
        <ConfigFormDialog
          key={editingConfig.id}
          existingConfig={editingConfig}
          onClose={() => setEditingConfig(null)}
        />
      )}

      {/* S3 Backups */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <FolderArchive className="h-5 w-5" />
                S3 Backups
              </CardTitle>
              <CardDescription>
                Backups stored in your S3-compatible storage.
              </CardDescription>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() =>
                queryClient.invalidateQueries({ queryKey: ["s3-backups"] })
              }
            >
              <RefreshCw className="h-4 w-4" />
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {backupsLoading ? (
            <div className="animate-pulse space-y-3">
              <div className="h-4 bg-muted rounded w-2/3"></div>
              <div className="h-4 bg-muted rounded w-1/2"></div>
            </div>
          ) : !backups || backups.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              No S3 backups yet. Create a backup using the &quot;Backup to
              S3&quot; button above.
            </p>
          ) : (
            <div className="space-y-3">
              {backups.map((backup) => (
                <div
                  key={backup.id}
                  className="flex items-center justify-between p-4 bg-muted/50 rounded-lg"
                >
                  <div className="space-y-1 min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <BackupTypeIcon type={backup.backup_type} />
                      <span className="font-medium capitalize">
                        {backup.backup_type}
                      </span>
                      <StatusBadge status={backup.status} />
                    </div>
                    <div className="flex items-center gap-3 text-xs text-muted-foreground">
                      <span className="font-mono truncate max-w-xs">
                        {backup.s3_key}
                      </span>
                      {backup.size_human && <span>{backup.size_human}</span>}
                      <span>{formatDate(backup.created_at)}</span>
                    </div>
                    {backup.storage_config_name && (
                      <div className="text-xs text-muted-foreground">
                        Storage: {backup.storage_config_name}
                      </div>
                    )}
                    {backup.error_message && (
                      <div className="text-xs text-destructive flex items-center gap-1">
                        <XCircle className="h-3 w-3" />
                        {backup.error_message}
                      </div>
                    )}
                  </div>
                  <div className="flex items-center gap-2 ml-4">
                    {backup.status === "completed" && (
                      <AlertDialog>
                        <AlertDialogTrigger asChild>
                          <Button variant="outline" size="sm">
                            <Download className="h-4 w-4 mr-1" />
                            Restore
                          </Button>
                        </AlertDialogTrigger>
                        <AlertDialogContent>
                          <AlertDialogHeader>
                            <AlertDialogTitle>
                              Restore from S3 Backup?
                            </AlertDialogTitle>
                            <AlertDialogDescription>
                              This will download the backup from S3 and restore
                              it. Your current database and configuration will be
                              replaced. A server restart is required after
                              restoration.
                            </AlertDialogDescription>
                          </AlertDialogHeader>
                          <AlertDialogFooter>
                            <AlertDialogCancel>Cancel</AlertDialogCancel>
                            <AlertDialogAction
                              onClick={() =>
                                restoreMutation.mutate(backup.id)
                              }
                              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                            >
                              {restoreMutation.isPending ? (
                                <>
                                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                                  Restoring...
                                </>
                              ) : (
                                "Yes, Restore"
                              )}
                            </AlertDialogAction>
                          </AlertDialogFooter>
                        </AlertDialogContent>
                      </AlertDialog>
                    )}
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
                          <AlertDialogTitle>Delete S3 Backup</AlertDialogTitle>
                          <AlertDialogDescription>
                            This will delete the backup from both S3 and the
                            local record. This action cannot be undone.
                          </AlertDialogDescription>
                        </AlertDialogHeader>
                        <AlertDialogFooter>
                          <AlertDialogCancel>Cancel</AlertDialogCancel>
                          <AlertDialogAction
                            onClick={() =>
                              deleteBackupMutation.mutate(backup.id)
                            }
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
    </div>
  );
}
