/**
 * SharedEnvVarsTable — reusable component for managing team-level or
 * project-level shared environment variables.
 */
import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
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
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Eye, EyeOff, Lock, Pencil, Plus, Trash2 } from "lucide-react";
import { sharedEnvVarsApi } from "@/lib/api/shared-env-vars";
import type {
  TeamEnvVar,
  ProjectEnvVar,
  CreateTeamEnvVarRequest,
  CreateProjectEnvVarRequest,
  UpdateTeamEnvVarRequest,
  UpdateProjectEnvVarRequest,
} from "@/types/api";

type SharedVar = TeamEnvVar | ProjectEnvVar;

interface SharedEnvVarsTableProps {
  /** "team" or "project" */
  scope: "team" | "project";
  /** The team or project ID */
  scopeId: string;
  title?: string;
  description?: string;
}

export function SharedEnvVarsTable({
  scope,
  scopeId,
  title,
  description,
}: SharedEnvVarsTableProps) {
  const queryClient = useQueryClient();
  const queryKey = ["shared-env-vars", scope, scopeId];

  const [showAddDialog, setShowAddDialog] = useState(false);
  const [showEditDialog, setShowEditDialog] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [selectedVar, setSelectedVar] = useState<SharedVar | null>(null);
  const [revealedIds, setRevealedIds] = useState<Set<string>>(new Set());

  // Form state
  const [formKey, setFormKey] = useState("");
  const [formValue, setFormValue] = useState("");
  const [formIsSecret, setFormIsSecret] = useState(false);
  const [formDescription, setFormDescription] = useState("");

  const {
    data: vars = [],
    isLoading,
    error,
  } = useQuery<SharedVar[]>({
    queryKey,
    queryFn: () =>
      scope === "team"
        ? sharedEnvVarsApi.getTeamEnvVars(scopeId)
        : sharedEnvVarsApi.getProjectEnvVars(scopeId),
  });

  const createMutation = useMutation<
    SharedVar,
    Error,
    CreateTeamEnvVarRequest | CreateProjectEnvVarRequest
  >({
    mutationFn: (data) =>
      (scope === "team"
        ? sharedEnvVarsApi.createTeamEnvVar(
            scopeId,
            data as CreateTeamEnvVarRequest
          )
        : sharedEnvVarsApi.createProjectEnvVar(
            scopeId,
            data as CreateProjectEnvVarRequest
          )) as Promise<SharedVar>,
    onSuccess: () => {
      toast.success("Variable created");
      queryClient.invalidateQueries({ queryKey });
      resetForm();
      setShowAddDialog(false);
    },
    onError: (err: Error) => {
      if (err.message.includes("409")) {
        toast.error("A variable with this key already exists");
      } else if (err.message.includes("400")) {
        toast.error(
          "Invalid key format. Use only letters, numbers, and underscores."
        );
      } else {
        toast.error(`Failed to create variable: ${err.message}`);
      }
    },
  });

  const updateMutation = useMutation<
    SharedVar,
    Error,
    { varId: string; data: UpdateTeamEnvVarRequest | UpdateProjectEnvVarRequest }
  >({
    mutationFn: ({ varId, data }) =>
      (scope === "team"
        ? sharedEnvVarsApi.updateTeamEnvVar(
            scopeId,
            varId,
            data as UpdateTeamEnvVarRequest
          )
        : sharedEnvVarsApi.updateProjectEnvVar(
            scopeId,
            varId,
            data as UpdateProjectEnvVarRequest
          )) as Promise<SharedVar>,
    onSuccess: () => {
      toast.success("Variable updated");
      queryClient.invalidateQueries({ queryKey });
      resetForm();
      setShowEditDialog(false);
    },
    onError: (err: Error) => {
      toast.error(`Failed to update variable: ${err.message}`);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (varId: string) =>
      scope === "team"
        ? sharedEnvVarsApi.deleteTeamEnvVar(scopeId, varId)
        : sharedEnvVarsApi.deleteProjectEnvVar(scopeId, varId),
    onSuccess: () => {
      toast.success("Variable deleted");
      queryClient.invalidateQueries({ queryKey });
      setShowDeleteDialog(false);
      setSelectedVar(null);
    },
    onError: (err: Error) => {
      toast.error(`Failed to delete variable: ${err.message}`);
    },
  });

  const resetForm = () => {
    setFormKey("");
    setFormValue("");
    setFormIsSecret(false);
    setFormDescription("");
    setSelectedVar(null);
  };

  const handleAdd = () => {
    resetForm();
    setShowAddDialog(true);
  };

  const handleEdit = (v: SharedVar) => {
    setSelectedVar(v);
    setFormKey(v.key);
    setFormValue(v.is_secret && !revealedIds.has(v.id) ? "" : v.value);
    setFormIsSecret(v.is_secret);
    setFormDescription(v.description ?? "");
    setShowEditDialog(true);
  };

  const handleDelete = (v: SharedVar) => {
    setSelectedVar(v);
    setShowDeleteDialog(true);
  };

  const handleToggleReveal = (v: SharedVar) => {
    if (revealedIds.has(v.id)) {
      setRevealedIds((prev) => {
        const next = new Set(prev);
        next.delete(v.id);
        return next;
      });
    } else {
      // Fetch with reveal=true
      const fetchFn =
        scope === "team"
          ? sharedEnvVarsApi.getTeamEnvVars(scopeId, true)
          : sharedEnvVarsApi.getProjectEnvVars(scopeId, true);

      fetchFn
        .then((all) => {
          const found = all.find((x) => x.id === v.id);
          if (found) {
            queryClient.setQueryData<SharedVar[]>(queryKey, (old) => {
              if (!old) return [found];
              return old.map((x) => (x.id === found.id ? found : x));
            });
            setRevealedIds((prev) => new Set(prev).add(v.id));
          }
        })
        .catch(() => {
          toast.error("Failed to reveal value");
        });
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
      description: formDescription || undefined,
    });
  };

  const handleSubmitEdit = () => {
    if (!selectedVar) return;

    const updates: UpdateTeamEnvVarRequest = {};
    if (formValue || !selectedVar.is_secret) {
      updates.value = formValue;
    }
    if (formIsSecret !== selectedVar.is_secret) {
      updates.is_secret = formIsSecret;
    }
    if (formDescription !== (selectedVar.description ?? "")) {
      updates.description = formDescription || undefined;
    }

    if (Object.keys(updates).length === 0) {
      toast.info("No changes to save");
      return;
    }

    updateMutation.mutate({ varId: selectedVar.id, data: updates });
  };

  const scopeLabel = scope === "team" ? "team" : "project";
  const cardTitle =
    title ?? (scope === "team" ? "Team Shared Variables" : "Project Shared Variables");
  const cardDesc =
    description ??
    `Variables shared with all apps in this ${scopeLabel}. App-level variables take precedence.`;

  if (isLoading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>{cardTitle}</CardTitle>
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
          <CardTitle>{cardTitle}</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-center text-red-500 py-4">
            Failed to load shared variables
          </p>
        </CardContent>
      </Card>
    );
  }

  return (
    <>
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>{cardTitle}</CardTitle>
              <CardDescription>{cardDesc}</CardDescription>
            </div>
            <Button onClick={handleAdd} size="sm">
              <Plus className="h-4 w-4 mr-1" />
              Add Variable
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {vars.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              No shared variables defined.
              <br />
              Click "Add Variable" to create one.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-[220px]">Key</TableHead>
                  <TableHead>Value</TableHead>
                  <TableHead className="w-[120px]">Description</TableHead>
                  <TableHead className="w-[90px]">Type</TableHead>
                  <TableHead className="w-[100px]">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {vars.map((v) => (
                  <TableRow key={v.id}>
                    <TableCell className="font-mono text-sm">{v.key}</TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <span className="font-mono text-sm truncate max-w-[250px]">
                          {revealedIds.has(v.id) ? v.value : "••••••••"}
                        </span>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleToggleReveal(v)}
                          className="h-6 w-6 p-0"
                          title={
                            revealedIds.has(v.id) ? "Hide value" : "Reveal value"
                          }
                        >
                          {revealedIds.has(v.id) ? (
                            <EyeOff className="h-3 w-3" />
                          ) : (
                            <Eye className="h-3 w-3" />
                          )}
                        </Button>
                      </div>
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground truncate max-w-[140px]">
                      {v.description ?? "—"}
                    </TableCell>
                    <TableCell>
                      {v.is_secret ? (
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
                          onClick={() => handleEdit(v)}
                          className="h-7 w-7 p-0"
                          title="Edit"
                        >
                          <Pencil className="h-3 w-3" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleDelete(v)}
                          className="h-7 w-7 p-0 text-red-500 hover:text-red-600"
                          title="Delete"
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
      </Card>

      {/* Add Dialog */}
      <Dialog open={showAddDialog} onOpenChange={setShowAddDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Shared Variable</DialogTitle>
            <DialogDescription>
              This variable will be inherited by all apps in this {scopeLabel}.
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
            <div className="space-y-2">
              <Label htmlFor="add-description">Description (optional)</Label>
              <Input
                id="add-description"
                placeholder="What is this variable used for?"
                value={formDescription}
                onChange={(e) => setFormDescription(e.target.value)}
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
            <DialogTitle>Edit Shared Variable</DialogTitle>
            <DialogDescription>
              Update the value of <strong>{selectedVar?.key}</strong>
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
                  selectedVar?.is_secret && !revealedIds.has(selectedVar?.id ?? "")
                    ? "Enter new value to replace existing..."
                    : "Enter value..."
                }
                value={formValue}
                onChange={(e) => setFormValue(e.target.value)}
                className="font-mono min-h-[80px]"
              />
              {selectedVar?.is_secret &&
                !revealedIds.has(selectedVar?.id ?? "") && (
                  <p className="text-xs text-muted-foreground">
                    Current value is hidden. Enter a new value to replace it,
                    or leave empty to keep current.
                  </p>
                )}
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-description">Description (optional)</Label>
              <Input
                id="edit-description"
                placeholder="What is this variable used for?"
                value={formDescription}
                onChange={(e) => setFormDescription(e.target.value)}
              />
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
            <AlertDialogTitle>Delete Shared Variable</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete{" "}
              <strong>{selectedVar?.key}</strong>? Apps currently inheriting this
              variable will lose it on next deployment.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() =>
                selectedVar && deleteMutation.mutate(selectedVar.id)
              }
              className="bg-red-500 hover:bg-red-600"
            >
              {deleteMutation.isPending ? "Deleting..." : "Delete"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
