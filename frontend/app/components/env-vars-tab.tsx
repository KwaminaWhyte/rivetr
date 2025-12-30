import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
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
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Skeleton } from "@/components/ui/skeleton";
import { Badge } from "@/components/ui/badge";
import { Eye, EyeOff, Pencil, Trash2, Plus, Lock, Code, List, Loader2, AlertTriangle } from "lucide-react";
import { api } from "@/lib/api";
import type { EnvVar, CreateEnvVarRequest, UpdateEnvVarRequest } from "@/types/api";

// Parse .env file content into key-value pairs
function parseEnvContent(content: string): { key: string; value: string }[] {
  const lines = content.split(/\r?\n/);
  const result: { key: string; value: string }[] = [];

  for (const line of lines) {
    // Skip empty lines and comments
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;

    // Find the first = sign
    const eqIndex = trimmed.indexOf("=");
    if (eqIndex === -1) continue;

    const key = trimmed.substring(0, eqIndex).trim();
    let value = trimmed.substring(eqIndex + 1);

    // Handle quoted values
    if ((value.startsWith('"') && value.endsWith('"')) ||
        (value.startsWith("'") && value.endsWith("'"))) {
      value = value.slice(1, -1);
    }

    // Validate key format (letters, numbers, underscores)
    if (/^[A-Za-z_][A-Za-z0-9_]*$/.test(key)) {
      result.push({ key: key.toUpperCase(), value });
    }
  }

  return result;
}

// Convert env vars to .env file format
function envVarsToString(envVars: EnvVar[], revealed: Map<string, string>): string {
  return envVars
    .map((env) => {
      const value = revealed.get(env.key) ?? env.value;
      // Quote values that contain special characters
      const needsQuotes = value.includes(" ") || value.includes("#") || value.includes("\n");
      const quotedValue = needsQuotes ? `"${value.replace(/"/g, '\\"')}"` : value;
      return `${env.key}=${quotedValue}`;
    })
    .join("\n");
}

interface EnvVarsTabProps {
  appId: string;
  token: string;
}

export function EnvVarsTab({ appId, token }: EnvVarsTabProps) {
  const queryClient = useQueryClient();
  const [viewMode, setViewMode] = useState<"normal" | "developer">("normal");
  const [showAddDialog, setShowAddDialog] = useState(false);
  const [showEditDialog, setShowEditDialog] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [selectedEnvVar, setSelectedEnvVar] = useState<EnvVar | null>(null);
  const [revealedKeys, setRevealedKeys] = useState<Set<string>>(new Set());

  // Developer view state
  const [devEnvContent, setDevEnvContent] = useState("");
  const [isSavingBulk, setIsSavingBulk] = useState(false);
  const [showBulkConfirm, setShowBulkConfirm] = useState(false);
  const [pendingBulkChanges, setPendingBulkChanges] = useState<{
    toAdd: { key: string; value: string }[];
    toUpdate: { key: string; value: string }[];
    toDelete: string[];
  } | null>(null);

  // Form state for add/edit
  const [formKey, setFormKey] = useState("");
  const [formValue, setFormValue] = useState("");
  const [formIsSecret, setFormIsSecret] = useState(false);

  // Fetch env vars (with secrets masked)
  const {
    data: envVars = [],
    isLoading,
    error,
  } = useQuery<EnvVar[]>({
    queryKey: ["env-vars", appId],
    queryFn: () => api.getEnvVars(appId, false, token),
  });

  // Create mutation
  const createMutation = useMutation({
    mutationFn: (data: CreateEnvVarRequest) => api.createEnvVar(appId, data, token),
    onSuccess: () => {
      toast.success("Environment variable created");
      queryClient.invalidateQueries({ queryKey: ["env-vars", appId] });
      resetForm();
      setShowAddDialog(false);
    },
    onError: (error: Error) => {
      if (error.message.includes("409") || error.message.includes("CONFLICT")) {
        toast.error("A variable with this key already exists");
      } else if (error.message.includes("400")) {
        toast.error("Invalid key format. Use only letters, numbers, and underscores.");
      } else {
        toast.error(`Failed to create: ${error.message}`);
      }
    },
  });

  // Update mutation
  const updateMutation = useMutation({
    mutationFn: ({ key, data }: { key: string; data: UpdateEnvVarRequest }) =>
      api.updateEnvVar(appId, key, data, token),
    onSuccess: () => {
      toast.success("Environment variable updated");
      queryClient.invalidateQueries({ queryKey: ["env-vars", appId] });
      // Clear revealed state for this key since the value changed
      if (selectedEnvVar) {
        setRevealedKeys((prev) => {
          const next = new Set(prev);
          next.delete(selectedEnvVar.key);
          return next;
        });
      }
      resetForm();
      setShowEditDialog(false);
    },
    onError: (error: Error) => {
      toast.error(`Failed to update: ${error.message}`);
    },
  });

  // Delete mutation
  const deleteMutation = useMutation({
    mutationFn: (key: string) => api.deleteEnvVar(appId, key, token),
    onSuccess: () => {
      toast.success("Environment variable deleted");
      queryClient.invalidateQueries({ queryKey: ["env-vars", appId] });
      setShowDeleteDialog(false);
      setSelectedEnvVar(null);
    },
    onError: (error: Error) => {
      toast.error(`Failed to delete: ${error.message}`);
    },
  });

  // Reveal secret value
  const revealMutation = useMutation({
    mutationFn: (key: string) => api.getEnvVar(appId, key, true, token),
    onSuccess: (data) => {
      // Update the revealed state
      setRevealedKeys((prev) => new Set(prev).add(data.key));
      // Update the query cache with the revealed value
      queryClient.setQueryData<EnvVar[]>(["env-vars", appId], (old) => {
        if (!old) return [data];
        return old.map((v) => (v.key === data.key ? { ...v, value: data.value } : v));
      });
    },
    onError: (error: Error) => {
      toast.error(`Failed to reveal: ${error.message}`);
    },
  });

  const resetForm = () => {
    setFormKey("");
    setFormValue("");
    setFormIsSecret(false);
    setSelectedEnvVar(null);
  };

  const handleAdd = () => {
    resetForm();
    setShowAddDialog(true);
  };

  const handleEdit = (envVar: EnvVar) => {
    setSelectedEnvVar(envVar);
    setFormKey(envVar.key);
    // If secret and not revealed, start with empty value
    setFormValue(envVar.is_secret && !revealedKeys.has(envVar.key) ? "" : envVar.value);
    setFormIsSecret(envVar.is_secret);
    setShowEditDialog(true);
  };

  const handleDelete = (envVar: EnvVar) => {
    setSelectedEnvVar(envVar);
    setShowDeleteDialog(true);
  };

  const handleToggleReveal = (envVar: EnvVar) => {
    if (revealedKeys.has(envVar.key)) {
      // Hide it - just remove from revealed set
      setRevealedKeys((prev) => {
        const next = new Set(prev);
        next.delete(envVar.key);
        return next;
      });
    } else {
      // Reveal it - fetch the actual value
      revealMutation.mutate(envVar.key);
    }
  };

  const handleSubmitAdd = () => {
    if (!formKey.trim()) {
      toast.error("Key is required");
      return;
    }
    createMutation.mutate({
      key: formKey.trim().toUpperCase(),
      value: formValue,
      is_secret: formIsSecret,
    });
  };

  const handleSubmitEdit = () => {
    if (!selectedEnvVar) return;

    const updates: UpdateEnvVarRequest = {};

    // Only include value if it was changed (not empty when secret)
    if (formValue || !selectedEnvVar.is_secret) {
      updates.value = formValue;
    }

    // Include is_secret if changed
    if (formIsSecret !== selectedEnvVar.is_secret) {
      updates.is_secret = formIsSecret;
    }

    // Check if anything to update
    if (Object.keys(updates).length === 0) {
      toast.info("No changes to save");
      return;
    }

    updateMutation.mutate({ key: selectedEnvVar.key, data: updates });
  };

  const isMultiline = (value: string) => value.includes("\n") || value.length > 50;

  // Prepare bulk changes for review
  const prepareBulkChanges = () => {
    const parsed = parseEnvContent(devEnvContent);
    const existingKeys = new Set(envVars.map((e) => e.key));
    const newKeys = new Set(parsed.map((e) => e.key));

    const toAdd: { key: string; value: string }[] = [];
    const toUpdate: { key: string; value: string }[] = [];
    const toDelete: string[] = [];

    // Check for new and updated vars
    for (const { key, value } of parsed) {
      if (existingKeys.has(key)) {
        const existing = envVars.find((e) => e.key === key);
        // Only mark as update if value actually changed
        if (existing && existing.value !== value) {
          toUpdate.push({ key, value });
        }
      } else {
        toAdd.push({ key, value });
      }
    }

    // Check for deleted vars
    for (const existing of envVars) {
      if (!newKeys.has(existing.key)) {
        toDelete.push(existing.key);
      }
    }

    return { toAdd, toUpdate, toDelete };
  };

  // Handle bulk save
  const handleBulkSave = async () => {
    if (!pendingBulkChanges) return;

    const { toAdd, toUpdate, toDelete } = pendingBulkChanges;
    setIsSavingBulk(true);

    try {
      // Delete removed vars
      for (const key of toDelete) {
        await api.deleteEnvVar(appId, key, token);
      }

      // Update existing vars
      for (const { key, value } of toUpdate) {
        await api.updateEnvVar(appId, key, { value }, token);
      }

      // Add new vars
      for (const { key, value } of toAdd) {
        await api.createEnvVar(appId, { key, value, is_secret: false }, token);
      }

      toast.success(
        `Environment variables updated: ${toAdd.length} added, ${toUpdate.length} updated, ${toDelete.length} deleted`
      );
      queryClient.invalidateQueries({ queryKey: ["env-vars", appId] });
      setShowBulkConfirm(false);
      setPendingBulkChanges(null);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to save changes");
    } finally {
      setIsSavingBulk(false);
    }
  };

  // Initialize dev content when switching to developer view
  const handleViewModeChange = (mode: string) => {
    const newMode = mode as "normal" | "developer";
    setViewMode(newMode);
    if (newMode === "developer" && envVars.length > 0) {
      // Fetch revealed values for all secrets before showing
      const revealedMap = new Map<string, string>();
      for (const env of envVars) {
        if (revealedKeys.has(env.key)) {
          revealedMap.set(env.key, env.value);
        }
      }
      setDevEnvContent(envVarsToString(envVars, revealedMap));
    }
  };

  // Show preview of changes
  const handlePreviewChanges = () => {
    const changes = prepareBulkChanges();
    if (changes.toAdd.length === 0 && changes.toUpdate.length === 0 && changes.toDelete.length === 0) {
      toast.info("No changes detected");
      return;
    }
    setPendingBulkChanges(changes);
    setShowBulkConfirm(true);
  };

  if (isLoading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Environment Variables</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            {[1, 2, 3].map((i) => (
              <Skeleton key={i} className="h-10 w-full" />
            ))}
          </div>
        </CardContent>
      </Card>
    );
  }

  if (error) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Environment Variables</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-center py-4 text-red-500">
            Failed to load environment variables
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>Environment Variables</CardTitle>
            <CardDescription>
              Configure environment variables for your application.
            </CardDescription>
          </div>
          <Tabs value={viewMode} onValueChange={handleViewModeChange}>
            <TabsList>
              <TabsTrigger value="normal" className="gap-1.5">
                <List className="h-4 w-4" />
                Normal
              </TabsTrigger>
              <TabsTrigger value="developer" className="gap-1.5">
                <Code className="h-4 w-4" />
                Developer
              </TabsTrigger>
            </TabsList>
          </Tabs>
        </div>
      </CardHeader>
      <CardContent>
        {viewMode === "normal" ? (
          // Normal View - Table with individual vars
          <>
            <div className="flex justify-end mb-4">
              <Button onClick={handleAdd} size="sm">
                <Plus className="h-4 w-4 mr-1" />
                Add Variable
              </Button>
            </div>
            {envVars.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                No environment variables defined.
                <br />
                Click "Add Variable" to create one.
              </div>
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-[200px]">Key</TableHead>
                    <TableHead>Value</TableHead>
                    <TableHead className="w-[100px]">Type</TableHead>
                    <TableHead className="w-[120px]">Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {envVars.map((envVar) => (
                    <TableRow key={envVar.id}>
                      <TableCell className="font-mono text-sm">{envVar.key}</TableCell>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <span
                            className={`font-mono text-sm ${
                              !revealedKeys.has(envVar.key)
                                ? "text-muted-foreground"
                                : ""
                            } ${revealedKeys.has(envVar.key) && isMultiline(envVar.value) ? "whitespace-pre-wrap" : "truncate max-w-[300px]"}`}
                          >
                            {revealedKeys.has(envVar.key) ? envVar.value : "••••••••"}
                          </span>
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => handleToggleReveal(envVar)}
                            disabled={revealMutation.isPending}
                            className="h-6 w-6 p-0"
                            title={revealedKeys.has(envVar.key) ? "Hide value" : "Reveal value"}
                          >
                            {revealedKeys.has(envVar.key) ? (
                              <EyeOff className="h-3 w-3" />
                            ) : (
                              <Eye className="h-3 w-3" />
                            )}
                          </Button>
                        </div>
                      </TableCell>
                      <TableCell>
                        {envVar.is_secret ? (
                          <Badge variant="secondary" className="gap-1">
                            <Lock className="h-3 w-3" />
                            Secret
                          </Badge>
                        ) : (
                          <Badge variant="outline">Plain</Badge>
                        )}
                      </TableCell>
                      <TableCell>
                        <div className="flex gap-1">
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => handleEdit(envVar)}
                            className="h-7 w-7 p-0"
                          >
                            <Pencil className="h-3 w-3" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => handleDelete(envVar)}
                            className="h-7 w-7 p-0 text-red-500 hover:text-red-600"
                          >
                            <Trash2 className="h-3 w-3" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}
          </>
        ) : (
          // Developer View - Textarea for .env format
          <div className="space-y-4">
            <div className="p-3 bg-muted/50 rounded-lg text-sm text-muted-foreground">
              <p>Paste your <code className="bg-muted px-1 rounded">.env</code> file contents below. Format:</p>
              <pre className="mt-2 text-xs">
{`DATABASE_URL=postgres://...
API_KEY="your-api-key"
# Comments are ignored
DEBUG=true`}
              </pre>
            </div>
            <Textarea
              value={devEnvContent}
              onChange={(e) => setDevEnvContent(e.target.value)}
              placeholder={`# Paste your .env file contents here\nDATABASE_URL=...\nAPI_KEY=...`}
              className="font-mono min-h-[300px] text-sm"
            />
            <div className="flex items-center justify-between">
              <p className="text-xs text-muted-foreground">
                {parseEnvContent(devEnvContent).length} variable(s) detected
              </p>
              <Button onClick={handlePreviewChanges} disabled={isSavingBulk}>
                {isSavingBulk ? (
                  <>
                    <Loader2 className="h-4 w-4 mr-1 animate-spin" />
                    Saving...
                  </>
                ) : (
                  "Review & Save"
                )}
              </Button>
            </div>
          </div>
        )}
      </CardContent>

      {/* Add Dialog */}
      <Dialog open={showAddDialog} onOpenChange={setShowAddDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Environment Variable</DialogTitle>
            <DialogDescription>
              Add a new environment variable. Keys will be converted to uppercase.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="add-key">Key</Label>
              <Input
                id="add-key"
                placeholder="DATABASE_URL"
                value={formKey}
                onChange={(e) => setFormKey(e.target.value.toUpperCase())}
                className="font-mono"
              />
              <p className="text-xs text-muted-foreground">
                Only letters, numbers, and underscores allowed.
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="add-value">Value</Label>
              <Textarea
                id="add-value"
                placeholder="Enter value..."
                value={formValue}
                onChange={(e) => setFormValue(e.target.value)}
                className="font-mono min-h-[80px]"
              />
            </div>
            <div className="flex items-center gap-2">
              <input
                type="checkbox"
                id="add-secret"
                checked={formIsSecret}
                onChange={(e) => setFormIsSecret(e.target.checked)}
                className="h-4 w-4 rounded border-gray-300"
              />
              <Label htmlFor="add-secret" className="text-sm font-normal cursor-pointer">
                Mark as secret (value will be masked in the UI)
              </Label>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowAddDialog(false)}>
              Cancel
            </Button>
            <Button onClick={handleSubmitAdd} disabled={createMutation.isPending}>
              {createMutation.isPending ? "Creating..." : "Create"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit Dialog */}
      <Dialog open={showEditDialog} onOpenChange={setShowEditDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit Environment Variable</DialogTitle>
            <DialogDescription>
              Update the value of {selectedEnvVar?.key}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label>Key</Label>
              <Input value={formKey} disabled className="font-mono bg-muted" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-value">Value</Label>
              <Textarea
                id="edit-value"
                placeholder={
                  selectedEnvVar?.is_secret && !revealedKeys.has(selectedEnvVar?.key || "")
                    ? "Enter new value to replace existing..."
                    : "Enter value..."
                }
                value={formValue}
                onChange={(e) => setFormValue(e.target.value)}
                className="font-mono min-h-[80px]"
              />
              {selectedEnvVar?.is_secret && !revealedKeys.has(selectedEnvVar?.key || "") && (
                <p className="text-xs text-muted-foreground">
                  Current value is hidden. Enter a new value to replace it, or leave empty to keep current.
                </p>
              )}
            </div>
            <div className="flex items-center gap-2">
              <input
                type="checkbox"
                id="edit-secret"
                checked={formIsSecret}
                onChange={(e) => setFormIsSecret(e.target.checked)}
                className="h-4 w-4 rounded border-gray-300"
              />
              <Label htmlFor="edit-secret" className="text-sm font-normal cursor-pointer">
                Mark as secret
              </Label>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowEditDialog(false)}>
              Cancel
            </Button>
            <Button onClick={handleSubmitEdit} disabled={updateMutation.isPending}>
              {updateMutation.isPending ? "Saving..." : "Save Changes"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <AlertDialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Environment Variable</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete <strong>{selectedEnvVar?.key}</strong>?
              This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => selectedEnvVar && deleteMutation.mutate(selectedEnvVar.key)}
              className="bg-red-500 hover:bg-red-600"
            >
              {deleteMutation.isPending ? "Deleting..." : "Delete"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Bulk Save Confirmation */}
      <Dialog open={showBulkConfirm} onOpenChange={setShowBulkConfirm}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>Review Changes</DialogTitle>
            <DialogDescription>
              The following changes will be applied to your environment variables.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4 space-y-4 max-h-[400px] overflow-y-auto">
            {pendingBulkChanges?.toAdd.length ? (
              <div>
                <h4 className="text-sm font-medium text-green-600 mb-2 flex items-center gap-1">
                  <Plus className="h-4 w-4" />
                  New Variables ({pendingBulkChanges.toAdd.length})
                </h4>
                <div className="space-y-1">
                  {pendingBulkChanges.toAdd.map(({ key, value }) => (
                    <div key={key} className="text-xs font-mono bg-green-50 dark:bg-green-950/30 p-2 rounded">
                      <span className="text-green-700 dark:text-green-400">{key}</span>=
                      <span className="text-muted-foreground truncate">{value.length > 50 ? value.slice(0, 50) + "..." : value}</span>
                    </div>
                  ))}
                </div>
              </div>
            ) : null}

            {pendingBulkChanges?.toUpdate.length ? (
              <div>
                <h4 className="text-sm font-medium text-blue-600 mb-2 flex items-center gap-1">
                  <Pencil className="h-4 w-4" />
                  Updated Variables ({pendingBulkChanges.toUpdate.length})
                </h4>
                <div className="space-y-1">
                  {pendingBulkChanges.toUpdate.map(({ key, value }) => (
                    <div key={key} className="text-xs font-mono bg-blue-50 dark:bg-blue-950/30 p-2 rounded">
                      <span className="text-blue-700 dark:text-blue-400">{key}</span>=
                      <span className="text-muted-foreground truncate">{value.length > 50 ? value.slice(0, 50) + "..." : value}</span>
                    </div>
                  ))}
                </div>
              </div>
            ) : null}

            {pendingBulkChanges?.toDelete.length ? (
              <div>
                <h4 className="text-sm font-medium text-red-600 mb-2 flex items-center gap-1">
                  <Trash2 className="h-4 w-4" />
                  Deleted Variables ({pendingBulkChanges.toDelete.length})
                </h4>
                <div className="space-y-1">
                  {pendingBulkChanges.toDelete.map((key) => (
                    <div key={key} className="text-xs font-mono bg-red-50 dark:bg-red-950/30 p-2 rounded">
                      <span className="text-red-700 dark:text-red-400">{key}</span>
                    </div>
                  ))}
                </div>
              </div>
            ) : null}

            {pendingBulkChanges?.toDelete.length ? (
              <div className="flex items-start gap-2 p-3 bg-yellow-50 dark:bg-yellow-950/30 rounded-lg">
                <AlertTriangle className="h-4 w-4 text-yellow-600 mt-0.5" />
                <p className="text-xs text-yellow-700 dark:text-yellow-400">
                  {pendingBulkChanges.toDelete.length} variable(s) will be deleted. This cannot be undone.
                </p>
              </div>
            ) : null}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowBulkConfirm(false)}>
              Cancel
            </Button>
            <Button onClick={handleBulkSave} disabled={isSavingBulk}>
              {isSavingBulk ? (
                <>
                  <Loader2 className="h-4 w-4 mr-1 animate-spin" />
                  Saving...
                </>
              ) : (
                "Apply Changes"
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </Card>
  );
}
