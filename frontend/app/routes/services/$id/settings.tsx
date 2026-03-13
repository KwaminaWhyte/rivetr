import { useState, useEffect, useRef } from "react";
import { useOutletContext, useNavigate } from "react-router";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export function meta() {
  return [
    { title: "Service Settings - Rivetr" },
    { name: "description", content: "Configure service settings and environment variables" },
  ];
}
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Input } from "@/components/ui/input";
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
import { Switch } from "@/components/ui/switch";
import { toast } from "sonner";
import { api } from "@/lib/api";
import type { Service } from "@/types/api";
import { Trash2, AlertTriangle, Code, Globe, Pencil, X, Save, AlertCircle, Database, Upload, Download, Network } from "lucide-react";

interface OutletContext {
  service: Service;
}

/** Parse image names from a docker-compose YAML string */
function parseComposeImages(composeContent: string): string[] {
  const images: string[] = [];
  const lines = composeContent.split("\n");
  for (const line of lines) {
    const match = line.match(/^\s+image:\s+(.+)$/);
    if (match) {
      images.push(match[1].trim().toLowerCase());
    }
  }
  return images;
}

/** Parse container_name entries from a docker-compose YAML string */
function parseContainerNames(composeContent: string): string[] {
  const names: string[] = [];
  const lines = composeContent.split("\n");
  for (const line of lines) {
    const match = line.match(/^\s+container_name:\s+(.+)$/);
    if (match) {
      names.push(match[1].trim());
    }
  }
  return names;
}

const DB_KEYWORDS = ["postgres", "mysql", "mariadb", "mongo", "redis"];

/** Returns true when the compose file contains at least one database service */
function hasDatabaseService(composeContent: string): boolean {
  const images = parseComposeImages(composeContent);
  return images.some((img) => DB_KEYWORDS.some((kw) => img.includes(kw)));
}

export default function ServiceSettingsTab() {
  const { service } = useOutletContext<OutletContext>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [deleteConfirmName, setDeleteConfirmName] = useState("");
  const [isEditing, setIsEditing] = useState(false);
  const [composeContent, setComposeContent] = useState(service.compose_content);
  const [domain, setDomain] = useState(service.domain ?? "");
  const [port, setPort] = useState(service.port ?? 80);
  const [isolatedNetwork, setIsolatedNetwork] = useState(service.isolated_network ?? true);

  // Database import state
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [importFile, setImportFile] = useState<File | null>(null);
  const [importContainer, setImportContainer] = useState("");
  const [importDatabase, setImportDatabase] = useState("app");
  const showImportSection = hasDatabaseService(service.compose_content);

  // Mutations
  const updateComposeMutation = useMutation({
    mutationFn: (composeContent: string) =>
      api.updateService(service.id, { compose_content: composeContent }),
    onSuccess: () => {
      toast.success("Compose configuration updated. Restart the service to apply changes.");
      setIsEditing(false);
      queryClient.invalidateQueries({ queryKey: ["service", service.id] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to update compose configuration");
    },
  });

  const updateDomainMutation = useMutation({
    mutationFn: (data: { domain: string; port: number }) =>
      api.updateService(service.id, { domain: data.domain, port: data.port }),
    onSuccess: () => {
      toast.success("Domain configuration saved. Restart the service to apply changes.");
      queryClient.invalidateQueries({ queryKey: ["service", service.id] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to update domain configuration");
    },
  });

  const updateNetworkMutation = useMutation({
    mutationFn: (isolated: boolean) =>
      api.updateService(service.id, { isolated_network: isolated }),
    onSuccess: () => {
      toast.success("Network isolation setting saved. Restart the service to apply changes.");
      queryClient.invalidateQueries({ queryKey: ["service", service.id] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to update network settings");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: () => api.deleteService(service.id),
    onSuccess: () => {
      toast.success("Service deleted");
      if (service.project_id) {
        navigate(`/projects/${service.project_id}`);
      } else {
        navigate("/projects");
      }
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to delete service");
    },
  });

  const importDbMutation = useMutation({
    mutationFn: () => {
      if (!importFile) throw new Error("Please select a file to import");
      return api.importServiceDb(service.id, importFile, importContainer, importDatabase);
    },
    onSuccess: () => {
      toast.success("Database dump imported successfully");
      setImportFile(null);
      if (fileInputRef.current) fileInputRef.current.value = "";
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to import database dump");
    },
  });

  const isSubmitting = updateComposeMutation.isPending || updateDomainMutation.isPending || deleteMutation.isPending;

  // Reset compose content when service changes (e.g., after save)
  useEffect(() => {
    if (!isEditing) {
      setComposeContent(service.compose_content);
    }
  }, [service.compose_content, isEditing]);

  // Sync domain/port state when service data changes
  useEffect(() => {
    setDomain(service.domain ?? "");
    setPort(service.port ?? 80);
    setIsolatedNetwork(service.isolated_network ?? true);
  }, [service.domain, service.port, service.isolated_network]);

  const handleCancelEdit = () => {
    setComposeContent(service.compose_content);
    setIsEditing(false);
  };

  const handleSaveCompose = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    updateComposeMutation.mutate(composeContent);
  };

  const handleSaveDomain = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    updateDomainMutation.mutate({ domain, port });
  };

  const handleDelete = () => {
    deleteMutation.mutate();
  };

  return (
    <div className="space-y-6">
      {/* Domain Configuration */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Globe className="h-5 w-5" />
            Domain & Proxy Port
          </CardTitle>
          <CardDescription>
            Configure the subdomain and the host port the proxy forwards to. If your service shows a 502 error, verify the port matches the host-side port in your compose <code className="text-xs bg-muted px-1 py-0.5 rounded">ports:</code> mapping.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSaveDomain} className="space-y-4">
            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="domain">Domain</Label>
                <Input
                  id="domain"
                  type="text"
                  placeholder="myservice.rivetr.site"
                  value={domain}
                  onChange={(e) => setDomain(e.target.value)}
                />
                <p className="text-xs text-muted-foreground">
                  Leave empty to disable proxy routing.
                </p>
              </div>
              <div className="space-y-2">
                <Label htmlFor="port">Port</Label>
                <Input
                  id="port"
                  type="number"
                  min={1}
                  max={65535}
                  placeholder="80"
                  value={port}
                  onChange={(e) => setPort(Number(e.target.value))}
                />
                <p className="text-xs text-muted-foreground">
                  The host port the proxy forwards traffic to (the left side of <code className="font-mono">HOST:CONTAINER</code> in your compose ports mapping). If you see a 502 error, check this matches the port in your compose file.
                </p>
              </div>
            </div>
            <div className="flex justify-end">
              <Button
                type="submit"
                disabled={isSubmitting}
                className="gap-2"
              >
                <Save className="h-4 w-4" />
                {updateDomainMutation.isPending ? "Saving..." : "Save Domain"}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>

      {/* Networking */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Network className="h-5 w-5" />
            Networking
          </CardTitle>
          <CardDescription>
            Control how this service&apos;s containers are networked relative to other services.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <Label htmlFor="isolated-network" className="text-sm font-medium">
                Isolated Network
              </Label>
              <p className="text-sm text-muted-foreground">
                Run this service&apos;s containers in a dedicated Docker network, isolated from other services.
                When enabled, a network named <code className="text-xs bg-muted px-1 py-0.5 rounded">rivetr-svc-{service.id.slice(0, 8)}</code> is created automatically.
              </p>
            </div>
            <Switch
              id="isolated-network"
              checked={isolatedNetwork}
              onCheckedChange={(checked) => {
                setIsolatedNetwork(checked);
                updateNetworkMutation.mutate(checked);
              }}
              disabled={updateNetworkMutation.isPending}
            />
          </div>
        </CardContent>
      </Card>

      {/* Docker Compose Configuration Editor */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Code className="h-5 w-5" />
                Docker Compose Configuration
              </CardTitle>
              <CardDescription className="mt-1.5">
                Edit the Docker Compose YAML for this service. Changes will take effect after restarting the service.
              </CardDescription>
            </div>
            {!isEditing && (
              <Button
                variant="outline"
                className="gap-2"
                onClick={() => setIsEditing(true)}
              >
                <Pencil className="h-4 w-4" />
                Edit
              </Button>
            )}
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          {isEditing && (
            <div className="flex items-start gap-3 p-4 rounded-lg bg-amber-50 border border-amber-200 text-amber-800 dark:bg-amber-950/30 dark:border-amber-800 dark:text-amber-200">
              <AlertCircle className="h-5 w-5 flex-shrink-0 mt-0.5" />
              <p className="text-sm">
                Changes to the compose configuration require a service restart to take effect.
                Make sure your YAML syntax is valid before saving.
              </p>
            </div>
          )}

          <form onSubmit={handleSaveCompose}>
            <div className="space-y-4">
              <div className="relative">
                <Textarea
                  name="compose_content"
                  value={composeContent}
                  onChange={(e) => setComposeContent(e.target.value)}
                  readOnly={!isEditing}
                  className="font-mono text-sm min-h-[400px] resize-y bg-muted/50"
                  style={{
                    fontFamily: "'JetBrains Mono', 'Fira Code', 'Consolas', monospace",
                    lineHeight: "1.5",
                    tabSize: 2,
                  }}
                  placeholder="version: '3.8'&#10;services:&#10;  app:&#10;    image: nginx:latest&#10;    ports:&#10;      - '8080:80'"
                />
              </div>

              {isEditing && (
                <div className="flex justify-end gap-2">
                  <Button
                    type="button"
                    variant="outline"
                    onClick={handleCancelEdit}
                    disabled={isSubmitting}
                    className="gap-2"
                  >
                    <X className="h-4 w-4" />
                    Cancel
                  </Button>
                  <Button
                    type="submit"
                    disabled={isSubmitting}
                    className="gap-2"
                  >
                    <Save className="h-4 w-4" />
                    {updateComposeMutation.isPending ? "Saving..." : "Save Changes"}
                  </Button>
                </div>
              )}
            </div>
          </form>
        </CardContent>
      </Card>

      {/* Database Import/Export — only shown for services with database containers */}
      {showImportSection && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Database className="h-5 w-5" />
              Database Dump
            </CardTitle>
            <CardDescription>
              Export or import a database dump from a running database container. The service must be running.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-start gap-3 p-4 rounded-lg bg-amber-50 border border-amber-200 text-amber-800 dark:bg-amber-950/30 dark:border-amber-800 dark:text-amber-200">
              <AlertCircle className="h-5 w-5 flex-shrink-0 mt-0.5" />
              <p className="text-sm">
                Importing a dump will execute SQL against the running container. Ensure the service
                is running and the target database exists before importing.
              </p>
            </div>

            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="import-container">Container name</Label>
                <Input
                  id="import-container"
                  placeholder={parseContainerNames(service.compose_content)[0] ?? "e.g. rivetr-myservice-db"}
                  value={importContainer}
                  onChange={(e) => setImportContainer(e.target.value)}
                />
                <p className="text-xs text-muted-foreground">
                  Leave empty to auto-detect the first running database container.
                </p>
              </div>
              <div className="space-y-2">
                <Label htmlFor="import-database">Database name</Label>
                <Input
                  id="import-database"
                  placeholder="app"
                  value={importDatabase}
                  onChange={(e) => setImportDatabase(e.target.value)}
                />
                <p className="text-xs text-muted-foreground">
                  The target database to restore into. Defaults to <code className="font-mono">app</code>.
                </p>
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="import-file">Dump file</Label>
              <Input
                id="import-file"
                ref={fileInputRef}
                type="file"
                accept=".sql,.gz,.sql.gz"
                className="cursor-pointer"
                onChange={(e) => setImportFile(e.target.files?.[0] ?? null)}
              />
              <p className="text-xs text-muted-foreground">
                Accepts <code className="font-mono">.sql</code> (plain SQL) or{" "}
                <code className="font-mono">.sql.gz</code> / <code className="font-mono">.gz</code>{" "}
                (compressed / pg_restore archive). Max 100&nbsp;MB.
              </p>
            </div>

            {importFile && (
              <p className="text-sm text-muted-foreground">
                Selected: <strong>{importFile.name}</strong> ({(importFile.size / 1024 / 1024).toFixed(2)}&nbsp;MB)
              </p>
            )}

            <div className="flex justify-end gap-2">
              <Button
                type="button"
                variant="outline"
                disabled={service.status !== "running"}
                onClick={() => {
                  const params = new URLSearchParams({ database: importDatabase });
                  if (importContainer) params.set("container_name", importContainer);
                  window.location.href = `/api/services/${service.id}/export-db?${params}`;
                }}
                className="gap-2"
              >
                <Download className="h-4 w-4" />
                Export Dump
              </Button>
              <Button
                type="button"
                disabled={!importFile || importDbMutation.isPending || service.status !== "running"}
                onClick={() => importDbMutation.mutate()}
                className="gap-2"
              >
                <Upload className="h-4 w-4" />
                {importDbMutation.isPending ? "Importing…" : "Import Dump"}
              </Button>
            </div>

            {service.status !== "running" && (
              <p className="text-sm text-destructive">
                The service must be running before you can import or export a dump.
              </p>
            )}
          </CardContent>
        </Card>
      )}

      {/* Danger Zone */}
      <Card className="border-destructive">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-destructive">
            <AlertTriangle className="h-5 w-5" />
            Danger Zone
          </CardTitle>
          <CardDescription>
            Irreversible and destructive actions
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between p-4 border border-destructive/50 rounded-lg">
            <div>
              <h4 className="font-medium">Delete Service</h4>
              <p className="text-sm text-muted-foreground">
                Permanently delete this service and all its data. This action cannot be undone.
              </p>
            </div>
            <AlertDialog>
              <AlertDialogTrigger asChild>
                <Button variant="destructive" className="gap-2">
                  <Trash2 className="h-4 w-4" />
                  Delete Service
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>Delete Service</AlertDialogTitle>
                  <AlertDialogDescription>
                    This action cannot be undone. This will permanently delete the service
                    <strong className="text-foreground"> {service.name}</strong> and remove all
                    associated containers and volumes.
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <div className="py-4">
                  <Label htmlFor="confirm-name" className="text-sm">
                    Type <strong>{service.name}</strong> to confirm:
                  </Label>
                  <Input
                    id="confirm-name"
                    value={deleteConfirmName}
                    onChange={(e) => setDeleteConfirmName(e.target.value)}
                    placeholder={service.name}
                    className="mt-2"
                  />
                </div>
                <AlertDialogFooter>
                  <AlertDialogCancel onClick={() => setDeleteConfirmName("")}>
                    Cancel
                  </AlertDialogCancel>
                  <AlertDialogAction
                    onClick={handleDelete}
                    disabled={deleteConfirmName !== service.name || isSubmitting}
                    className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                  >
                    {deleteMutation.isPending ? "Deleting..." : "Delete Service"}
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
