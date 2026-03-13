import { useState } from "react";
import { useOutletContext, useNavigate } from "react-router";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { ManagedDatabase, UpdateManagedDatabaseRequest } from "@/types/api";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Textarea } from "@/components/ui/textarea";
import { Switch } from "@/components/ui/switch";
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
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { toast } from "sonner";
import { api } from "@/lib/api";
import {
  AlertTriangle,
  Trash2,
  Settings2,
  Cpu,
  HardDrive,
  Globe,
  Lock,
  Shield,
  ShieldAlert,
  ShieldCheck,
  RefreshCw,
  Info,
  Image,
} from "lucide-react";

const SSL_MODES_POSTGRES = ["disable", "allow", "prefer", "require", "verify-ca", "verify-full"];
const SSL_MODES_MYSQL = ["disabled", "preferred", "required", "verify-ca", "verify-identity"];

export function meta() {
  return [
    { title: "Database Settings - Rivetr" },
    { name: "description", content: "Configure database settings and resource limits" },
  ];
}

interface OutletContext {
  database: ManagedDatabase;
}

export default function DatabaseSettingsTab() {
  const { database } = useOutletContext<OutletContext>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const [deleteConfirmName, setDeleteConfirmName] = useState("");
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [showPublicAccessDialog, setShowPublicAccessDialog] = useState(false);

  // Form state for network settings
  const [publicAccess, setPublicAccess] = useState(database.public_access);
  const [externalPort, setExternalPort] = useState<string>(
    database.external_port > 0 ? String(database.external_port) : ""
  );
  const [useCustomPort, setUseCustomPort] = useState(database.external_port > 0);

  // SSL/TLS state
  const [sslEnabled, setSslEnabled] = useState(database.ssl_enabled ?? false);
  const [sslMode, setSslMode] = useState(database.ssl_mode ?? "");

  // Custom image & init commands state
  const [customImage, setCustomImage] = useState(database.custom_image ?? "");
  // init_commands is stored as a JSON array; display one command per line for editing
  const parseInitCommands = (json: string | null): string => {
    if (!json) return "";
    try {
      const arr = JSON.parse(json);
      if (Array.isArray(arr)) return arr.join("\n");
    } catch {
      // Not valid JSON — show as-is
    }
    return json;
  };
  const [initCommandsText, setInitCommandsText] = useState(
    parseInitCommands(database.init_commands)
  );

  const deleteMutation = useMutation({
    mutationFn: () => api.deleteDatabase(database.id),
    onSuccess: () => {
      toast.success("Database deleted");
      navigate("/projects");
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to delete database");
    },
  });

  const updateMutation = useMutation({
    mutationFn: (data: UpdateManagedDatabaseRequest) =>
      api.updateDatabase(database.id, data),
    onSuccess: () => {
      toast.success("Database settings updated");
      queryClient.invalidateQueries({ queryKey: ["database", database.id] });
      setShowPublicAccessDialog(false);
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to update database");
    },
  });

  const sslMutation = useMutation({
    mutationFn: (data: UpdateManagedDatabaseRequest) =>
      api.updateDatabase(database.id, data),
    onSuccess: () => {
      toast.success("SSL/TLS settings updated");
      queryClient.invalidateQueries({ queryKey: ["database", database.id] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to update SSL settings");
    },
  });

  const imageMutation = useMutation({
    mutationFn: (data: UpdateManagedDatabaseRequest) =>
      api.updateDatabase(database.id, data),
    onSuccess: () => {
      toast.success("Image settings saved");
      queryClient.invalidateQueries({ queryKey: ["database", database.id] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to save image settings");
    },
  });

  const isSubmitting = deleteMutation.isPending || updateMutation.isPending || sslMutation.isPending || imageMutation.isPending;
  const canDelete = deleteConfirmName === database.name;
  const hasNetworkChanges =
    publicAccess !== database.public_access ||
    (useCustomPort ? parseInt(externalPort) || 0 : 0) !== database.external_port;

  const handleDelete = () => {
    deleteMutation.mutate();
  };

  const handleNetworkUpdate = () => {
    const updateData: UpdateManagedDatabaseRequest = {
      public_access: publicAccess,
      external_port: useCustomPort && publicAccess ? parseInt(externalPort) || 0 : 0,
    };
    updateMutation.mutate(updateData);
  };

  const validatePort = (value: string): boolean => {
    if (!value) return true; // Empty is valid (auto-assign)
    const port = parseInt(value);
    return !isNaN(port) && port >= 1024 && port <= 65535;
  };

  const isPortValid = !useCustomPort || !externalPort || validatePort(externalPort);

  // SSL helpers
  const supportsSSL = database.db_type === "postgres" || database.db_type === "mysql" || database.db_type === "mariadb";
  const sslModes = database.db_type === "postgres" ? SSL_MODES_POSTGRES : SSL_MODES_MYSQL;
  const hasSslChanges = sslEnabled !== (database.ssl_enabled ?? false) || sslMode !== (database.ssl_mode ?? "");

  const handleSslUpdate = () => {
    sslMutation.mutate({ ssl_enabled: sslEnabled, ssl_mode: sslMode || undefined });
  };

  // Helpers for image/init-commands card
  const serializeInitCommands = (text: string): string | undefined => {
    const lines = text
      .split("\n")
      .map((l) => l.trim())
      .filter((l) => l.length > 0);
    if (lines.length === 0) return undefined;
    return JSON.stringify(lines);
  };

  const hasImageChanges =
    customImage !== (database.custom_image ?? "") ||
    initCommandsText !== parseInitCommands(database.init_commands);

  const handleImageUpdate = () => {
    imageMutation.mutate({
      custom_image: customImage.trim() || undefined,
      init_commands: serializeInitCommands(initCommandsText),
    });
  };

  return (
    <div className="space-y-6">
      {/* Network Settings Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Settings2 className="h-5 w-5" />
            Network Access
          </CardTitle>
          <CardDescription>
            Configure how this database can be accessed
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Public Access Toggle */}
          <div className="flex items-center justify-between rounded-lg border p-4">
            <div className="space-y-0.5">
              <div className="flex items-center gap-2">
                {publicAccess ? (
                  <Globe className="h-4 w-4 text-amber-500" />
                ) : (
                  <Lock className="h-4 w-4 text-green-500" />
                )}
                <Label className="text-base">Public Access</Label>
              </div>
              <p className="text-sm text-muted-foreground">
                {publicAccess
                  ? "Database is accessible from external networks (internet)"
                  : "Database is only accessible within the Docker network"}
              </p>
            </div>
            <Switch
              checked={publicAccess}
              onCheckedChange={setPublicAccess}
              disabled={isSubmitting}
            />
          </div>

          {/* Security Warning for Public Access */}
          {publicAccess && (
            <div className="rounded-lg border border-amber-200 bg-amber-50 dark:border-amber-800 dark:bg-amber-950 p-4">
              <div className="flex items-start gap-3">
                <ShieldAlert className="h-5 w-5 text-amber-600 dark:text-amber-400 mt-0.5 flex-shrink-0" />
                <div className="space-y-2">
                  <h4 className="font-medium text-amber-800 dark:text-amber-200">
                    Security Warning
                  </h4>
                  <ul className="text-sm text-amber-700 dark:text-amber-300 space-y-1">
                    <li className="flex items-start gap-2">
                      <span className="mt-1.5">•</span>
                      <span>
                        Exposing your database to the internet increases security risks.
                        Ensure you have strong passwords.
                      </span>
                    </li>
                    <li className="flex items-start gap-2">
                      <span className="mt-1.5">•</span>
                      <span>
                        Consider using a VPN or SSH tunnel for remote access instead.
                      </span>
                    </li>
                    <li className="flex items-start gap-2">
                      <span className="mt-1.5">•</span>
                      <span>
                        Enable SSL/TLS if your database supports it (configure in the
                        database itself).
                      </span>
                    </li>
                    <li className="flex items-start gap-2">
                      <span className="mt-1.5">•</span>
                      <span>
                        Use firewall rules to restrict access to specific IP addresses
                        when possible.
                      </span>
                    </li>
                  </ul>
                </div>
              </div>
            </div>
          )}

          {/* Custom External Port */}
          {publicAccess && (
            <div className="space-y-4 rounded-lg border p-4">
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label className="text-base">Custom External Port</Label>
                  <p className="text-sm text-muted-foreground">
                    Specify a custom port or let the system auto-assign one
                  </p>
                </div>
                <Switch
                  checked={useCustomPort}
                  onCheckedChange={setUseCustomPort}
                  disabled={isSubmitting}
                />
              </div>

              {useCustomPort && (
                <div className="space-y-2">
                  <Label htmlFor="external_port">External Port</Label>
                  <Input
                    id="external_port"
                    type="number"
                    placeholder="e.g., 5433"
                    value={externalPort}
                    onChange={(e) => setExternalPort(e.target.value)}
                    className={`font-mono ${!isPortValid ? "border-destructive" : ""}`}
                    min={1024}
                    max={65535}
                    disabled={isSubmitting}
                  />
                  {!isPortValid ? (
                    <p className="text-xs text-destructive">
                      Port must be between 1024 and 65535
                    </p>
                  ) : (
                    <p className="text-xs text-muted-foreground">
                      Must be between 1024-65535. Make sure this port is not already in use.
                    </p>
                  )}
                </div>
              )}
            </div>
          )}

          {/* Private Access Benefits */}
          {!publicAccess && (
            <div className="rounded-lg border border-green-200 bg-green-50 dark:border-green-800 dark:bg-green-950 p-4">
              <div className="flex items-start gap-3">
                <Shield className="h-5 w-5 text-green-600 dark:text-green-400 mt-0.5 flex-shrink-0" />
                <div className="space-y-2">
                  <h4 className="font-medium text-green-800 dark:text-green-200">
                    Secure by Default
                  </h4>
                  <p className="text-sm text-green-700 dark:text-green-300">
                    Your database is only accessible within the Rivetr network. Other apps
                    deployed in Rivetr can connect using the internal hostname{" "}
                    <code className="bg-green-100 dark:bg-green-900 px-1 rounded">
                      rivetr-db-{database.name}
                    </code>
                  </p>
                </div>
              </div>
            </div>
          )}

          {/* Save Changes Button */}
          {hasNetworkChanges && (
            <div className="flex items-center justify-between pt-4 border-t">
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Info className="h-4 w-4" />
                <span>
                  {database.status === "running"
                    ? "Database will restart to apply changes"
                    : "Changes will apply on next start"}
                </span>
              </div>
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  onClick={() => {
                    setPublicAccess(database.public_access);
                    setExternalPort(
                      database.external_port > 0 ? String(database.external_port) : ""
                    );
                    setUseCustomPort(database.external_port > 0);
                  }}
                  disabled={isSubmitting}
                >
                  Cancel
                </Button>
                {publicAccess && !database.public_access ? (
                  <AlertDialog
                    open={showPublicAccessDialog}
                    onOpenChange={setShowPublicAccessDialog}
                  >
                    <AlertDialogTrigger asChild>
                      <Button disabled={isSubmitting || !isPortValid}>
                        {isSubmitting ? (
                          <>
                            <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                            Updating...
                          </>
                        ) : (
                          "Enable Public Access"
                        )}
                      </Button>
                    </AlertDialogTrigger>
                    <AlertDialogContent>
                      <AlertDialogHeader>
                        <AlertDialogTitle className="flex items-center gap-2 text-amber-600">
                          <ShieldAlert className="h-5 w-5" />
                          Enable Public Access?
                        </AlertDialogTitle>
                        <AlertDialogDescription className="space-y-3">
                          <p>
                            This will expose your database to the internet. Anyone with the
                            connection details can attempt to connect.
                          </p>
                          <p className="font-medium">Before proceeding, ensure:</p>
                          <ul className="list-disc list-inside space-y-1">
                            <li>Your database password is strong and unique</li>
                            <li>You understand the security implications</li>
                            <li>You have considered using a VPN instead</li>
                          </ul>
                        </AlertDialogDescription>
                      </AlertDialogHeader>
                      <AlertDialogFooter>
                        <AlertDialogCancel>Cancel</AlertDialogCancel>
                        <AlertDialogAction
                          onClick={handleNetworkUpdate}
                          className="bg-amber-600 hover:bg-amber-700"
                        >
                          Enable Public Access
                        </AlertDialogAction>
                      </AlertDialogFooter>
                    </AlertDialogContent>
                  </AlertDialog>
                ) : (
                  <Button
                    onClick={handleNetworkUpdate}
                    disabled={isSubmitting || !isPortValid}
                  >
                    {isSubmitting ? (
                      <>
                        <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                        Updating...
                      </>
                    ) : (
                      "Save Changes"
                    )}
                  </Button>
                )}
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      {/* SSL/TLS Card */}
      {supportsSSL && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <ShieldCheck className="h-5 w-5" />
              SSL / TLS
            </CardTitle>
            <CardDescription>
              Configure transport encryption for this database
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center justify-between rounded-lg border p-4">
              <div className="space-y-0.5">
                <Label className="text-base">Enable SSL/TLS</Label>
                <p className="text-sm text-muted-foreground">
                  Require encrypted connections to this database
                </p>
              </div>
              <Switch
                checked={sslEnabled}
                onCheckedChange={setSslEnabled}
                disabled={isSubmitting}
              />
            </div>

            {sslEnabled && (
              <div className="space-y-2">
                <Label>SSL Mode</Label>
                <Select
                  value={sslMode}
                  onValueChange={setSslMode}
                  disabled={isSubmitting}
                >
                  <SelectTrigger>
                    <SelectValue placeholder="Select SSL mode" />
                  </SelectTrigger>
                  <SelectContent>
                    {sslModes.map((mode) => (
                      <SelectItem key={mode} value={mode}>
                        {mode}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  {database.db_type === "postgres"
                    ? "PostgreSQL SSL modes: disable, allow, prefer, require, verify-ca, verify-full"
                    : "MySQL/MariaDB SSL modes: disabled, preferred, required, verify-ca, verify-identity"}
                </p>
              </div>
            )}

            {hasSslChanges && (
              <div className="flex items-center justify-between pt-4 border-t">
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <Info className="h-4 w-4" />
                  <span>Restart the database for changes to take effect</span>
                </div>
                <div className="flex gap-2">
                  <Button
                    variant="outline"
                    onClick={() => {
                      setSslEnabled(database.ssl_enabled ?? false);
                      setSslMode(database.ssl_mode ?? "");
                    }}
                    disabled={isSubmitting}
                  >
                    Cancel
                  </Button>
                  <Button onClick={handleSslUpdate} disabled={isSubmitting}>
                    {sslMutation.isPending ? (
                      <>
                        <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                        Saving...
                      </>
                    ) : (
                      "Save SSL Settings"
                    )}
                  </Button>
                </div>
              </div>
            )}
          </CardContent>
        </Card>
      )}

      {/* Custom Image & Init Commands Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Image className="h-5 w-5" />
            Custom Image & Init Commands
          </CardTitle>
          <CardDescription>
            Override the default Docker image and run SQL commands after first start
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="custom_image">Custom Docker Image</Label>
            <Input
              id="custom_image"
              placeholder={`Default: ${database.db_type === "postgres" ? "postgres" : database.db_type}:${database.version}`}
              value={customImage}
              onChange={(e) => setCustomImage(e.target.value)}
              className="font-mono"
              disabled={isSubmitting}
            />
            <p className="text-xs text-muted-foreground">
              Override the default image for this database type. E.g.{" "}
              <code className="bg-muted px-1 rounded">timescaledb/timescaledb-ha:pg16-latest</code>
              {" "}or{" "}
              <code className="bg-muted px-1 rounded">postgis/postgis:16-3.4</code>.
              Leave blank to use the default image.
            </p>
          </div>

          <div className="space-y-2">
            <Label htmlFor="init_commands">Init SQL Commands</Label>
            <Textarea
              id="init_commands"
              placeholder={"CREATE EXTENSION IF NOT EXISTS postgis;\nCREATE SCHEMA app;"}
              value={initCommandsText}
              onChange={(e) => setInitCommandsText(e.target.value)}
              className="font-mono text-sm min-h-[120px]"
              disabled={isSubmitting}
            />
            <p className="text-xs text-muted-foreground">
              One SQL command per line. These will be executed inside the container after
              the database first starts. For PostgreSQL, each line is passed to{" "}
              <code className="bg-muted px-1 rounded">psql -c</code>.
            </p>
          </div>

          {hasImageChanges && (
            <div className="flex items-center justify-between pt-4 border-t">
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Info className="h-4 w-4" />
                <span>
                  {database.status === "running"
                    ? "Recreate the container for the new image to take effect"
                    : "Changes will apply on next start"}
                </span>
              </div>
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  onClick={() => {
                    setCustomImage(database.custom_image ?? "");
                    setInitCommandsText(parseInitCommands(database.init_commands));
                  }}
                  disabled={isSubmitting}
                >
                  Cancel
                </Button>
                <Button onClick={handleImageUpdate} disabled={isSubmitting}>
                  {imageMutation.isPending ? (
                    <>
                      <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                      Saving...
                    </>
                  ) : (
                    "Save Image Settings"
                  )}
                </Button>
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Resource Limits Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Cpu className="h-5 w-5" />
            Resource Limits
          </CardTitle>
          <CardDescription>
            Configure CPU and memory limits for this database
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="memory_limit">Memory Limit</Label>
              <Input
                id="memory_limit"
                value={database.memory_limit || "512mb"}
                readOnly
                className="font-mono"
              />
              <p className="text-xs text-muted-foreground">
                Maximum memory the database can use (e.g., 512mb, 1g, 2g)
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="cpu_limit">CPU Limit</Label>
              <Input
                id="cpu_limit"
                value={database.cpu_limit || "0.5"}
                readOnly
                className="font-mono"
              />
              <p className="text-xs text-muted-foreground">
                Maximum CPU cores the database can use (e.g., 0.5, 1, 2)
              </p>
            </div>
          </div>
          <div className="rounded-md bg-muted p-3">
            <p className="text-sm text-muted-foreground">
              Resource limits are set at creation time. To change limits, delete and
              recreate the database with new values.
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Database Info Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <HardDrive className="h-5 w-5" />
            Database Information
          </CardTitle>
          <CardDescription>
            Read-only database configuration details
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label>Database ID</Label>
              <Input value={database.id} readOnly className="font-mono text-xs" />
            </div>
            <div className="space-y-2">
              <Label>Container ID</Label>
              <Input
                value={database.container_id || "Not running"}
                readOnly
                className="font-mono text-xs"
              />
            </div>
            <div className="space-y-2">
              <Label>Type</Label>
              <Input value={database.db_type.toUpperCase()} readOnly />
            </div>
            <div className="space-y-2">
              <Label>Version</Label>
              <Input value={database.version} readOnly />
            </div>
            <div className="space-y-2">
              <Label>Created</Label>
              <Input value={new Date(database.created_at).toLocaleString()} readOnly />
            </div>
            <div className="space-y-2">
              <Label>Last Updated</Label>
              <Input value={new Date(database.updated_at).toLocaleString()} readOnly />
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Danger Zone Card */}
      <Card className="border-destructive">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-destructive">
            <AlertTriangle className="h-5 w-5" />
            Danger Zone
          </CardTitle>
          <CardDescription>
            Irreversible actions that permanently affect your database
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="rounded-lg border border-destructive/50 p-4">
            <div className="flex items-start justify-between gap-4">
              <div className="space-y-1">
                <h4 className="font-medium text-destructive">Delete Database</h4>
                <p className="text-sm text-muted-foreground">
                  Permanently delete this database and all its data. This action cannot be undone.
                </p>
                <ul className="mt-2 text-sm text-muted-foreground list-disc list-inside">
                  <li>The database container will be stopped and removed</li>
                  <li>All data stored in the container will be lost</li>
                  <li>The database record will be permanently deleted</li>
                </ul>
              </div>
              <AlertDialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
                <AlertDialogTrigger asChild>
                  <Button variant="destructive" className="shrink-0">
                    <Trash2 className="h-4 w-4 mr-2" />
                    Delete Database
                  </Button>
                </AlertDialogTrigger>
                <AlertDialogContent>
                  <AlertDialogHeader>
                    <AlertDialogTitle className="flex items-center gap-2 text-destructive">
                      <AlertTriangle className="h-5 w-5" />
                      Delete Database
                    </AlertDialogTitle>
                    <AlertDialogDescription className="space-y-3">
                      <p>
                        This action is <strong>permanent and irreversible</strong>. Deleting this
                        database will:
                      </p>
                      <ul className="list-disc list-inside space-y-1">
                        <li>Stop and remove the database container</li>
                        <li>Delete all data stored in the container</li>
                        <li>Remove the database from your project</li>
                      </ul>
                      <p className="font-medium">
                        To confirm, type the database name: <code className="bg-muted rounded px-1">{database.name}</code>
                      </p>
                    </AlertDialogDescription>
                  </AlertDialogHeader>
                  <div className="py-4">
                    <Input
                      placeholder={`Type "${database.name}" to confirm`}
                      value={deleteConfirmName}
                      onChange={(e) => setDeleteConfirmName(e.target.value)}
                      className="font-mono"
                    />
                  </div>
                  <AlertDialogFooter>
                    <AlertDialogCancel onClick={() => setDeleteConfirmName("")}>
                      Cancel
                    </AlertDialogCancel>
                    <AlertDialogAction
                      onClick={handleDelete}
                      disabled={!canDelete || isSubmitting}
                      className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                    >
                      {isSubmitting ? "Deleting..." : "Delete Database"}
                    </AlertDialogAction>
                  </AlertDialogFooter>
                </AlertDialogContent>
              </AlertDialog>
            </div>
          </div>

          {/* Warning about data persistence */}
          {database.volume_path && (
            <div className="rounded-md bg-yellow-50 dark:bg-yellow-950 p-3">
              <p className="text-sm text-yellow-700 dark:text-yellow-300">
                <strong>Note:</strong> The data directory at{" "}
                <code className="bg-yellow-100 dark:bg-yellow-900 rounded px-1">
                  {database.volume_path}
                </code>{" "}
                will <strong>not</strong> be automatically deleted. You may want to manually remove
                it if you no longer need the data.
              </p>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
