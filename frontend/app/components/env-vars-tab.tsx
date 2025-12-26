import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
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
import { Eye, EyeOff, Pencil, Trash2, Plus, Lock } from "lucide-react";
import { api } from "@/lib/api";
import type { EnvVar, CreateEnvVarRequest, UpdateEnvVarRequest } from "@/types/api";

interface EnvVarsTabProps {
  appId: string;
  token: string;
}

export function EnvVarsTab({ appId, token }: EnvVarsTabProps) {
  const queryClient = useQueryClient();
  const [showAddDialog, setShowAddDialog] = useState(false);
  const [showEditDialog, setShowEditDialog] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [selectedEnvVar, setSelectedEnvVar] = useState<EnvVar | null>(null);
  const [revealedKeys, setRevealedKeys] = useState<Set<string>>(new Set());

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
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle>Environment Variables</CardTitle>
        <Button onClick={handleAdd} size="sm">
          <Plus className="h-4 w-4 mr-1" />
          Add Variable
        </Button>
      </CardHeader>
      <CardContent>
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
    </Card>
  );
}
