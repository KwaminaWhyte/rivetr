import { useState } from "react";
import { useParams } from "react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
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
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { logDrainsApi } from "@/lib/api/log-drains";
import type {
  LogDrain,
  LogDrainProvider,
  CreateLogDrainRequest,
  UpdateLogDrainRequest,
} from "@/types/api";
import {
  Plus,
  Pencil,
  Trash2,
  MoreHorizontal,
  TestTube2,
  AlertTriangle,
  CheckCircle2,
  Loader2,
  Unplug,
} from "lucide-react";

// Provider display info
const PROVIDERS: {
  value: LogDrainProvider;
  label: string;
  description: string;
}[] = [
  { value: "axiom", label: "Axiom", description: "Modern observability platform" },
  { value: "newrelic", label: "New Relic", description: "Full-stack observability" },
  { value: "datadog", label: "Datadog", description: "Cloud monitoring and analytics" },
  { value: "logtail", label: "Logtail (Better Stack)", description: "Log management by Better Stack" },
  { value: "http", label: "Custom HTTP", description: "Send logs to any HTTP endpoint" },
];

function getProviderLabel(provider: string): string {
  const p = PROVIDERS.find((p) => p.value === provider);
  return p?.label || provider;
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return "Never";
  try {
    const date = new Date(dateStr);
    return date.toLocaleString();
  } catch {
    return dateStr;
  }
}

// Dynamic config form fields per provider
function ProviderConfigFields({
  provider,
  config,
  onChange,
}: {
  provider: LogDrainProvider;
  config: Record<string, unknown>;
  onChange: (config: Record<string, unknown>) => void;
}) {
  const updateField = (key: string, value: string) => {
    onChange({ ...config, [key]: value });
  };

  switch (provider) {
    case "axiom":
      return (
        <div className="space-y-3">
          <div className="space-y-1.5">
            <Label htmlFor="dataset">Dataset</Label>
            <Input
              id="dataset"
              placeholder="my-dataset"
              value={(config.dataset as string) || ""}
              onChange={(e) => updateField("dataset", e.target.value)}
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="api_token">API Token</Label>
            <Input
              id="api_token"
              type="password"
              placeholder="xaat-..."
              value={(config.api_token as string) || ""}
              onChange={(e) => updateField("api_token", e.target.value)}
            />
          </div>
        </div>
      );

    case "newrelic":
      return (
        <div className="space-y-3">
          <div className="space-y-1.5">
            <Label htmlFor="api_key">API Key</Label>
            <Input
              id="api_key"
              type="password"
              placeholder="NRAK-..."
              value={(config.api_key as string) || ""}
              onChange={(e) => updateField("api_key", e.target.value)}
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="region">Region</Label>
            <Select
              value={(config.region as string) || "us"}
              onValueChange={(val) => updateField("region", val)}
            >
              <SelectTrigger id="region">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="us">US</SelectItem>
                <SelectItem value="eu">EU</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>
      );

    case "datadog":
      return (
        <div className="space-y-3">
          <div className="space-y-1.5">
            <Label htmlFor="api_key">API Key</Label>
            <Input
              id="api_key"
              type="password"
              placeholder="Your Datadog API key"
              value={(config.api_key as string) || ""}
              onChange={(e) => updateField("api_key", e.target.value)}
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="site">Site</Label>
            <Select
              value={(config.site as string) || "datadoghq.com"}
              onValueChange={(val) => updateField("site", val)}
            >
              <SelectTrigger id="site">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="datadoghq.com">datadoghq.com (US1)</SelectItem>
                <SelectItem value="us3.datadoghq.com">us3.datadoghq.com (US3)</SelectItem>
                <SelectItem value="us5.datadoghq.com">us5.datadoghq.com (US5)</SelectItem>
                <SelectItem value="datadoghq.eu">datadoghq.eu (EU)</SelectItem>
                <SelectItem value="ap1.datadoghq.com">ap1.datadoghq.com (AP1)</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>
      );

    case "logtail":
      return (
        <div className="space-y-3">
          <div className="space-y-1.5">
            <Label htmlFor="source_token">Source Token</Label>
            <Input
              id="source_token"
              type="password"
              placeholder="Your Logtail source token"
              value={(config.source_token as string) || ""}
              onChange={(e) => updateField("source_token", e.target.value)}
            />
          </div>
        </div>
      );

    case "http":
      return (
        <div className="space-y-3">
          <div className="space-y-1.5">
            <Label htmlFor="url">URL</Label>
            <Input
              id="url"
              placeholder="https://your-endpoint.com/logs"
              value={(config.url as string) || ""}
              onChange={(e) => updateField("url", e.target.value)}
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="auth_header_name">Auth Header Name (optional)</Label>
            <Input
              id="auth_header_name"
              placeholder="Authorization"
              value={(config.auth_header_name as string) || ""}
              onChange={(e) => updateField("auth_header_name", e.target.value)}
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="auth_header_value">Auth Header Value (optional)</Label>
            <Input
              id="auth_header_value"
              type="password"
              placeholder="Bearer your-token"
              value={(config.auth_header_value as string) || ""}
              onChange={(e) => updateField("auth_header_value", e.target.value)}
            />
          </div>
        </div>
      );

    default:
      return null;
  }
}

export default function LogDrainsPage() {
  const { id: appId } = useParams();
  const queryClient = useQueryClient();

  // State for dialogs
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [showEditDialog, setShowEditDialog] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [selectedDrain, setSelectedDrain] = useState<LogDrain | null>(null);

  // Form state
  const [formName, setFormName] = useState("");
  const [formProvider, setFormProvider] = useState<LogDrainProvider>("axiom");
  const [formConfig, setFormConfig] = useState<Record<string, unknown>>({});
  const [formEnabled, setFormEnabled] = useState(true);

  // Fetch log drains
  const {
    data: drains = [],
    isLoading,
  } = useQuery<LogDrain[]>({
    queryKey: ["logDrains", appId],
    queryFn: () => logDrainsApi.getLogDrains(appId!),
    enabled: !!appId,
  });

  // Create mutation
  const createMutation = useMutation({
    mutationFn: (data: CreateLogDrainRequest) =>
      logDrainsApi.createLogDrain(appId!, data),
    onSuccess: () => {
      toast.success("Log drain created");
      queryClient.invalidateQueries({ queryKey: ["logDrains", appId] });
      resetForm();
      setShowCreateDialog(false);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to create log drain");
    },
  });

  // Update mutation
  const updateMutation = useMutation({
    mutationFn: ({
      drainId,
      data,
    }: {
      drainId: string;
      data: UpdateLogDrainRequest;
    }) => logDrainsApi.updateLogDrain(appId!, drainId, data),
    onSuccess: () => {
      toast.success("Log drain updated");
      queryClient.invalidateQueries({ queryKey: ["logDrains", appId] });
      setShowEditDialog(false);
      setSelectedDrain(null);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to update log drain");
    },
  });

  // Delete mutation
  const deleteMutation = useMutation({
    mutationFn: (drainId: string) =>
      logDrainsApi.deleteLogDrain(appId!, drainId),
    onSuccess: () => {
      toast.success("Log drain deleted");
      queryClient.invalidateQueries({ queryKey: ["logDrains", appId] });
      setShowDeleteDialog(false);
      setSelectedDrain(null);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to delete log drain");
    },
  });

  // Toggle enabled mutation
  const toggleMutation = useMutation({
    mutationFn: ({ drainId, enabled }: { drainId: string; enabled: boolean }) =>
      logDrainsApi.updateLogDrain(appId!, drainId, { enabled }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["logDrains", appId] });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to toggle log drain");
    },
  });

  // Test mutation
  const testMutation = useMutation({
    mutationFn: (drainId: string) =>
      logDrainsApi.testLogDrain(appId!, drainId),
    onSuccess: (data) => {
      if (data.success) {
        toast.success(data.message);
      } else {
        toast.error(data.message);
      }
      queryClient.invalidateQueries({ queryKey: ["logDrains", appId] });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Test failed");
    },
  });

  const resetForm = () => {
    setFormName("");
    setFormProvider("axiom");
    setFormConfig({});
    setFormEnabled(true);
  };

  const handleCreate = () => {
    createMutation.mutate({
      name: formName,
      provider: formProvider,
      config: formConfig,
      enabled: formEnabled,
    });
  };

  const handleEdit = (drain: LogDrain) => {
    setSelectedDrain(drain);
    setFormName(drain.name);
    setFormProvider(drain.provider as LogDrainProvider);
    // Config from API has masked values -- start with empty so user re-enters secrets
    setFormConfig(drain.config as Record<string, unknown>);
    setFormEnabled(drain.enabled);
    setShowEditDialog(true);
  };

  const handleUpdate = () => {
    if (!selectedDrain) return;
    updateMutation.mutate({
      drainId: selectedDrain.id,
      data: {
        name: formName,
        config: formConfig,
        enabled: formEnabled,
      },
    });
  };

  const handleDelete = (drain: LogDrain) => {
    setSelectedDrain(drain);
    setShowDeleteDialog(true);
  };

  if (isLoading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Log Drains</CardTitle>
          <CardDescription>Forward container logs to external services</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center gap-2 text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" />
            Loading...
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Log Drains</CardTitle>
              <CardDescription>
                Forward container logs to external observability platforms
              </CardDescription>
            </div>
            <Button
              onClick={() => {
                resetForm();
                setShowCreateDialog(true);
              }}
              className="gap-2"
            >
              <Plus className="h-4 w-4" />
              Add Log Drain
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {drains.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              <Unplug className="h-12 w-12 mx-auto mb-4 opacity-40" />
              <p className="text-lg font-medium">No log drains configured</p>
              <p className="text-sm mt-1">
                Set up a log drain to forward your container logs to Axiom, Datadog, New Relic, or other services.
              </p>
              <Button
                variant="outline"
                className="mt-4 gap-2"
                onClick={() => {
                  resetForm();
                  setShowCreateDialog(true);
                }}
              >
                <Plus className="h-4 w-4" />
                Add your first log drain
              </Button>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Provider</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Last Sent</TableHead>
                  <TableHead>Errors</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {drains.map((drain) => (
                  <TableRow key={drain.id}>
                    <TableCell className="font-medium">{drain.name}</TableCell>
                    <TableCell>
                      <Badge variant="outline">{getProviderLabel(drain.provider)}</Badge>
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <Switch
                          checked={drain.enabled}
                          onCheckedChange={(checked) =>
                            toggleMutation.mutate({
                              drainId: drain.id,
                              enabled: checked,
                            })
                          }
                          aria-label={
                            drain.enabled ? "Disable log drain" : "Enable log drain"
                          }
                        />
                        <span className="text-sm text-muted-foreground">
                          {drain.enabled ? "Enabled" : "Disabled"}
                        </span>
                      </div>
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {formatDate(drain.last_sent_at)}
                    </TableCell>
                    <TableCell>
                      {drain.error_count > 0 ? (
                        <div className="flex items-center gap-1.5">
                          <AlertTriangle className="h-4 w-4 text-yellow-500" />
                          <span className="text-sm text-yellow-600">{drain.error_count}</span>
                          {drain.last_error && (
                            <span
                              className="text-xs text-muted-foreground truncate max-w-[200px]"
                              title={drain.last_error}
                            >
                              {drain.last_error}
                            </span>
                          )}
                        </div>
                      ) : (
                        <div className="flex items-center gap-1.5">
                          <CheckCircle2 className="h-4 w-4 text-green-500" />
                          <span className="text-sm text-muted-foreground">None</span>
                        </div>
                      )}
                    </TableCell>
                    <TableCell className="text-right">
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" size="icon">
                            <MoreHorizontal className="h-4 w-4" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem
                            onClick={() =>
                              testMutation.mutate(drain.id)
                            }
                            disabled={testMutation.isPending}
                          >
                            <TestTube2 className="h-4 w-4 mr-2" />
                            {testMutation.isPending ? "Testing..." : "Test"}
                          </DropdownMenuItem>
                          <DropdownMenuItem onClick={() => handleEdit(drain)}>
                            <Pencil className="h-4 w-4 mr-2" />
                            Edit
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            onClick={() => handleDelete(drain)}
                            className="text-destructive"
                          >
                            <Trash2 className="h-4 w-4 mr-2" />
                            Delete
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Create Dialog */}
      <Dialog
        open={showCreateDialog}
        onOpenChange={(open) => {
          setShowCreateDialog(open);
          if (!open) resetForm();
        }}
      >
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>Add Log Drain</DialogTitle>
            <DialogDescription>
              Configure a new log drain to forward container logs to an external service.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4">
            <div className="space-y-1.5">
              <Label htmlFor="create-name">Name</Label>
              <Input
                id="create-name"
                placeholder="Production Logs"
                value={formName}
                onChange={(e) => setFormName(e.target.value)}
              />
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="create-provider">Provider</Label>
              <Select
                value={formProvider}
                onValueChange={(val) => {
                  setFormProvider(val as LogDrainProvider);
                  setFormConfig({});
                }}
              >
                <SelectTrigger id="create-provider">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {PROVIDERS.map((p) => (
                    <SelectItem key={p.value} value={p.value}>
                      <span className="flex flex-col">
                        <span>{p.label}</span>
                      </span>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                {PROVIDERS.find((p) => p.value === formProvider)?.description}
              </p>
            </div>

            <ProviderConfigFields
              provider={formProvider}
              config={formConfig}
              onChange={setFormConfig}
            />

            <div className="flex items-center gap-2">
              <Switch
                id="create-enabled"
                checked={formEnabled}
                onCheckedChange={setFormEnabled}
              />
              <Label htmlFor="create-enabled">Enabled</Label>
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setShowCreateDialog(false);
                resetForm();
              }}
            >
              Cancel
            </Button>
            <Button
              onClick={handleCreate}
              disabled={createMutation.isPending || !formName.trim()}
            >
              {createMutation.isPending ? (
                <>
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  Creating...
                </>
              ) : (
                "Create"
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit Dialog */}
      <Dialog
        open={showEditDialog}
        onOpenChange={(open) => {
          setShowEditDialog(open);
          if (!open) {
            setSelectedDrain(null);
            resetForm();
          }
        }}
      >
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>Edit Log Drain</DialogTitle>
            <DialogDescription>
              Update the log drain configuration. Leave secret fields empty to keep current values.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4">
            <div className="space-y-1.5">
              <Label htmlFor="edit-name">Name</Label>
              <Input
                id="edit-name"
                value={formName}
                onChange={(e) => setFormName(e.target.value)}
              />
            </div>

            <div className="space-y-1.5">
              <Label>Provider</Label>
              <div className="flex items-center gap-2">
                <Badge variant="outline">
                  {selectedDrain ? getProviderLabel(selectedDrain.provider) : ""}
                </Badge>
                <span className="text-xs text-muted-foreground">
                  Provider cannot be changed after creation
                </span>
              </div>
            </div>

            {selectedDrain && (
              <ProviderConfigFields
                provider={selectedDrain.provider as LogDrainProvider}
                config={formConfig}
                onChange={setFormConfig}
              />
            )}

            <div className="flex items-center gap-2">
              <Switch
                id="edit-enabled"
                checked={formEnabled}
                onCheckedChange={setFormEnabled}
              />
              <Label htmlFor="edit-enabled">Enabled</Label>
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setShowEditDialog(false);
                setSelectedDrain(null);
                resetForm();
              }}
            >
              Cancel
            </Button>
            <Button
              onClick={handleUpdate}
              disabled={updateMutation.isPending || !formName.trim()}
            >
              {updateMutation.isPending ? (
                <>
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  Saving...
                </>
              ) : (
                "Save Changes"
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <Dialog
        open={showDeleteDialog}
        onOpenChange={(open) => {
          setShowDeleteDialog(open);
          if (!open) setSelectedDrain(null);
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Log Drain</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete the log drain "{selectedDrain?.name}"?
              This action cannot be undone. Logs will no longer be forwarded to this destination.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setShowDeleteDialog(false);
                setSelectedDrain(null);
              }}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={() =>
                selectedDrain && deleteMutation.mutate(selectedDrain.id)
              }
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending ? (
                <>
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  Deleting...
                </>
              ) : (
                "Delete"
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
