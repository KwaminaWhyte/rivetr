import { useState } from "react";
import { Link, useParams } from "react-router";
import { useQuery, useQueryClient, useMutation } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  ArrowLeft,
  Copy,
  Plus,
  Trash2,
  Edit2,
  Eye,
  EyeOff,
  Key,
  Shield,
} from "lucide-react";
import { api } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
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
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Checkbox } from "@/components/ui/checkbox";
import type {
  ProjectEnvironment,
  EnvironmentEnvVar,
  Project,
  CloneEnvironmentResponse,
} from "@/types/api";

export function meta() {
  return [
    { title: "Environments - Rivetr" },
    {
      name: "description",
      content: "Manage project environments and environment variables",
    },
  ];
}

function EnvironmentEnvVarsPanel({ environment }: { environment: ProjectEnvironment }) {
  const queryClient = useQueryClient();
  const [showSecrets, setShowSecrets] = useState(false);
  const [isAddDialogOpen, setIsAddDialogOpen] = useState(false);
  const [newKey, setNewKey] = useState("");
  const [newValue, setNewValue] = useState("");
  const [newIsSecret, setNewIsSecret] = useState(false);
  const [editingVar, setEditingVar] = useState<EnvironmentEnvVar | null>(null);
  const [editValue, setEditValue] = useState("");
  const [editIsSecret, setEditIsSecret] = useState(false);
  const [deleteVarId, setDeleteVarId] = useState<string | null>(null);

  const { data: envVars = [], isLoading } = useQuery<EnvironmentEnvVar[]>({
    queryKey: ["environment-env-vars", environment.id, showSecrets],
    queryFn: () => api.getEnvironmentEnvVars(environment.id, showSecrets),
  });

  const createMutation = useMutation({
    mutationFn: () =>
      api.createEnvironmentEnvVar(environment.id, {
        key: newKey.trim(),
        value: newValue,
        is_secret: newIsSecret,
      }),
    onSuccess: () => {
      toast.success("Environment variable created");
      setIsAddDialogOpen(false);
      setNewKey("");
      setNewValue("");
      setNewIsSecret(false);
      queryClient.invalidateQueries({
        queryKey: ["environment-env-vars", environment.id],
      });
    },
    onError: (err: Error) => toast.error(err.message),
  });

  const updateMutation = useMutation({
    mutationFn: () =>
      api.updateEnvironmentEnvVar(environment.id, editingVar!.id, {
        value: editValue,
        is_secret: editIsSecret,
      }),
    onSuccess: () => {
      toast.success("Environment variable updated");
      setEditingVar(null);
      queryClient.invalidateQueries({
        queryKey: ["environment-env-vars", environment.id],
      });
    },
    onError: (err: Error) => toast.error(err.message),
  });

  const deleteMutation = useMutation({
    mutationFn: (varId: string) =>
      api.deleteEnvironmentEnvVar(environment.id, varId),
    onSuccess: () => {
      toast.success("Environment variable deleted");
      setDeleteVarId(null);
      queryClient.invalidateQueries({
        queryKey: ["environment-env-vars", environment.id],
      });
    },
    onError: (err: Error) => toast.error(err.message),
  });

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Key className="h-4 w-4 text-muted-foreground" />
          <span className="text-sm font-medium">
            Environment Variables ({envVars.length})
          </span>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setShowSecrets(!showSecrets)}
          >
            {showSecrets ? (
              <EyeOff className="mr-1 h-3 w-3" />
            ) : (
              <Eye className="mr-1 h-3 w-3" />
            )}
            {showSecrets ? "Hide" : "Reveal"}
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setIsAddDialogOpen(true)}
          >
            <Plus className="mr-1 h-3 w-3" />
            Add Variable
          </Button>
        </div>
      </div>

      {isLoading ? (
        <div className="text-sm text-muted-foreground">Loading...</div>
      ) : envVars.length === 0 ? (
        <div className="text-sm text-muted-foreground py-4 text-center">
          No environment variables set for this environment.
        </div>
      ) : (
        <div className="space-y-2">
          {envVars.map((v) => (
            <div
              key={v.id}
              className="flex items-center justify-between gap-4 rounded-md border p-3"
            >
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <code className="text-sm font-mono font-medium">{v.key}</code>
                  {v.is_secret && (
                    <Badge variant="secondary" className="text-xs">
                      <Shield className="mr-1 h-3 w-3" />
                      Secret
                    </Badge>
                  )}
                </div>
                <div className="mt-1 text-sm text-muted-foreground font-mono truncate">
                  {v.value}
                </div>
              </div>
              <div className="flex items-center gap-1">
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7"
                  onClick={() => {
                    setEditingVar(v);
                    setEditValue(v.is_secret && !showSecrets ? "" : v.value);
                    setEditIsSecret(v.is_secret);
                  }}
                >
                  <Edit2 className="h-3 w-3" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7 text-destructive"
                  onClick={() => setDeleteVarId(v.id)}
                >
                  <Trash2 className="h-3 w-3" />
                </Button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Add Variable Dialog */}
      <Dialog open={isAddDialogOpen} onOpenChange={setIsAddDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Environment Variable</DialogTitle>
            <DialogDescription>
              Add a variable to the "{environment.name}" environment. These
              variables are inherited by apps assigned to this environment.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="env-key">Key</Label>
              <Input
                id="env-key"
                placeholder="DATABASE_URL"
                value={newKey}
                onChange={(e) => setNewKey(e.target.value.toUpperCase())}
                className="font-mono"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="env-value">Value</Label>
              <Textarea
                id="env-value"
                placeholder="postgres://user:pass@host:5432/db"
                value={newValue}
                onChange={(e) => setNewValue(e.target.value)}
                className="font-mono"
                rows={3}
              />
            </div>
            <div className="flex items-center space-x-2">
              <Checkbox
                id="env-secret"
                checked={newIsSecret}
                onCheckedChange={(checked) => setNewIsSecret(checked === true)}
              />
              <Label htmlFor="env-secret" className="text-sm">
                Mark as secret (value will be masked)
              </Label>
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setIsAddDialogOpen(false)}
            >
              Cancel
            </Button>
            <Button
              onClick={() => createMutation.mutate()}
              disabled={!newKey.trim() || createMutation.isPending}
            >
              {createMutation.isPending ? "Adding..." : "Add Variable"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit Variable Dialog */}
      <Dialog
        open={!!editingVar}
        onOpenChange={(open) => !open && setEditingVar(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit {editingVar?.key}</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label>Value</Label>
              <Textarea
                value={editValue}
                onChange={(e) => setEditValue(e.target.value)}
                className="font-mono"
                rows={3}
              />
            </div>
            <div className="flex items-center space-x-2">
              <Checkbox
                id="edit-env-secret"
                checked={editIsSecret}
                onCheckedChange={(checked) =>
                  setEditIsSecret(checked === true)
                }
              />
              <Label htmlFor="edit-env-secret" className="text-sm">
                Mark as secret
              </Label>
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setEditingVar(null)}
            >
              Cancel
            </Button>
            <Button
              onClick={() => updateMutation.mutate()}
              disabled={updateMutation.isPending}
            >
              {updateMutation.isPending ? "Saving..." : "Save"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <AlertDialog
        open={!!deleteVarId}
        onOpenChange={(open) => !open && setDeleteVarId(null)}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete environment variable?</AlertDialogTitle>
            <AlertDialogDescription>
              This will remove the variable from the "{environment.name}" environment.
              Apps using this variable will no longer receive it on next deployment.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => deleteVarId && deleteMutation.mutate(deleteVarId)}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}

export default function ProjectEnvironmentsPage() {
  const { id } = useParams();
  const queryClient = useQueryClient();
  const [activeTab, setActiveTab] = useState<string>("");
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false);
  const [isEditDialogOpen, setIsEditDialogOpen] = useState(false);
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
  const [isCloneDialogOpen, setIsCloneDialogOpen] = useState(false);
  const [selectedEnv, setSelectedEnv] = useState<ProjectEnvironment | null>(null);
  const [newEnvName, setNewEnvName] = useState("");
  const [newEnvDescription, setNewEnvDescription] = useState("");
  const [editName, setEditName] = useState("");
  const [editDescription, setEditDescription] = useState("");
  const [cloneName, setCloneName] = useState("");

  const { data: project } = useQuery<Project>({
    queryKey: ["project", id],
    queryFn: () => api.getProject(id!),
    enabled: !!id,
  });

  const { data: environments = [], isLoading } = useQuery<ProjectEnvironment[]>({
    queryKey: ["environments", id],
    queryFn: () => api.getEnvironments(id!),
    enabled: !!id,
  });

  // Set active tab to first environment when loaded
  if (environments.length > 0 && !activeTab) {
    const defaultEnv = environments.find((e) => e.is_default) || environments[0];
    setActiveTab(defaultEnv.id);
  }

  const createMutation = useMutation({
    mutationFn: () =>
      api.createEnvironment(id!, {
        name: newEnvName.trim(),
        description: newEnvDescription.trim() || undefined,
      }),
    onSuccess: () => {
      toast.success("Environment created");
      setIsCreateDialogOpen(false);
      setNewEnvName("");
      setNewEnvDescription("");
      queryClient.invalidateQueries({ queryKey: ["environments", id] });
    },
    onError: (err: Error) => toast.error(err.message),
  });

  const updateMutation = useMutation({
    mutationFn: () =>
      api.updateEnvironment(selectedEnv!.id, {
        name: editName.trim() || undefined,
        description: editDescription.trim() || undefined,
      }),
    onSuccess: () => {
      toast.success("Environment updated");
      setIsEditDialogOpen(false);
      setSelectedEnv(null);
      queryClient.invalidateQueries({ queryKey: ["environments", id] });
    },
    onError: (err: Error) => toast.error(err.message),
  });

  const deleteMutation = useMutation({
    mutationFn: () => api.deleteEnvironment(selectedEnv!.id),
    onSuccess: () => {
      toast.success("Environment deleted");
      setIsDeleteDialogOpen(false);
      setSelectedEnv(null);
      setActiveTab("");
      queryClient.invalidateQueries({ queryKey: ["environments", id] });
    },
    onError: (err: Error) => toast.error(err.message),
  });

  const cloneMutation = useMutation({
    mutationFn: () =>
      api.cloneEnvironment(id!, selectedEnv!.id, {
        name: cloneName.trim(),
      }),
    onSuccess: (result: CloneEnvironmentResponse) => {
      const parts = [
        result.cloned_apps > 0 ? `${result.cloned_apps} app${result.cloned_apps !== 1 ? "s" : ""}` : null,
        result.cloned_databases > 0 ? `${result.cloned_databases} database${result.cloned_databases !== 1 ? "s" : ""}` : null,
        result.cloned_services > 0 ? `${result.cloned_services} service${result.cloned_services !== 1 ? "s" : ""}` : null,
      ].filter(Boolean);
      const summary = parts.length > 0 ? ` (${parts.join(", ")} cloned)` : "";
      toast.success(`Environment "${result.name}" created${summary}`);
      setIsCloneDialogOpen(false);
      setSelectedEnv(null);
      setCloneName("");
      queryClient.invalidateQueries({ queryKey: ["environments", id] });
      // Navigate to the new environment tab
      setActiveTab(result.id);
    },
    onError: (err: Error) => toast.error(err.message),
  });

  const activeEnvironment = environments.find((e) => e.id === activeTab);

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary" />
      </div>
    );
  }

  return (
    <div className="container max-w-5xl mx-auto py-6 space-y-6">
      {/* Header */}
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="icon" asChild>
          <Link to={`/projects/${id}`}>
            <ArrowLeft className="h-5 w-5" />
          </Link>
        </Button>
        <div>
          <h1 className="text-2xl font-bold">
            Environments
          </h1>
          {project && (
            <p className="text-muted-foreground text-sm">
              {project.name}
            </p>
          )}
        </div>
        <div className="ml-auto">
          <Button onClick={() => setIsCreateDialogOpen(true)}>
            <Plus className="mr-2 h-4 w-4" />
            New Environment
          </Button>
        </div>
      </div>

      {environments.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center">
            <p className="text-muted-foreground mb-4">
              No environments found. Environments are auto-created when a project is created.
            </p>
            <Button onClick={() => setIsCreateDialogOpen(true)}>
              <Plus className="mr-2 h-4 w-4" />
              Create Environment
            </Button>
          </CardContent>
        </Card>
      ) : (
        <Tabs value={activeTab} onValueChange={setActiveTab}>
          <TabsList>
            {environments.map((env) => (
              <TabsTrigger key={env.id} value={env.id}>
                {env.name}
                {env.is_default && (
                  <Badge variant="secondary" className="ml-2 text-[10px] px-1 py-0">
                    default
                  </Badge>
                )}
              </TabsTrigger>
            ))}
          </TabsList>

          {environments.map((env) => (
            <TabsContent key={env.id} value={env.id}>
              <Card>
                <CardHeader className="flex flex-row items-center justify-between">
                  <div>
                    <CardTitle>{env.name}</CardTitle>
                    {env.description && (
                      <p className="text-sm text-muted-foreground mt-1">
                        {env.description}
                      </p>
                    )}
                  </div>
                  <div className="flex gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        setSelectedEnv(env);
                        setCloneName(`${env.name}-copy`);
                        setIsCloneDialogOpen(true);
                      }}
                    >
                      <Copy className="mr-1 h-3 w-3" />
                      Clone
                    </Button>
                    {!env.is_default && (
                      <>
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => {
                            setSelectedEnv(env);
                            setEditName(env.name);
                            setEditDescription(env.description || "");
                            setIsEditDialogOpen(true);
                          }}
                        >
                          <Edit2 className="mr-1 h-3 w-3" />
                          Edit
                        </Button>
                        <Button
                          variant="outline"
                          size="sm"
                          className="text-destructive border-destructive/50 hover:bg-destructive/10"
                          onClick={() => {
                            setSelectedEnv(env);
                            setIsDeleteDialogOpen(true);
                          }}
                        >
                          <Trash2 className="mr-1 h-3 w-3" />
                          Delete
                        </Button>
                      </>
                    )}
                  </div>
                </CardHeader>
                <CardContent>
                  <EnvironmentEnvVarsPanel environment={env} />
                </CardContent>
              </Card>
            </TabsContent>
          ))}
        </Tabs>
      )}

      {/* Create Environment Dialog */}
      <Dialog open={isCreateDialogOpen} onOpenChange={setIsCreateDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create Environment</DialogTitle>
            <DialogDescription>
              Add a new environment to this project. Each environment can have
              its own set of environment variables.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="env-name">Name</Label>
              <Input
                id="env-name"
                placeholder="e.g., qa, testing, preview"
                value={newEnvName}
                onChange={(e) => setNewEnvName(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="env-desc">Description (optional)</Label>
              <Input
                id="env-desc"
                placeholder="Description for this environment"
                value={newEnvDescription}
                onChange={(e) => setNewEnvDescription(e.target.value)}
              />
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setIsCreateDialogOpen(false)}
            >
              Cancel
            </Button>
            <Button
              onClick={() => createMutation.mutate()}
              disabled={!newEnvName.trim() || createMutation.isPending}
            >
              {createMutation.isPending ? "Creating..." : "Create"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit Environment Dialog */}
      <Dialog open={isEditDialogOpen} onOpenChange={setIsEditDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit Environment</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label>Name</Label>
              <Input
                value={editName}
                onChange={(e) => setEditName(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label>Description</Label>
              <Input
                value={editDescription}
                onChange={(e) => setEditDescription(e.target.value)}
              />
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setIsEditDialogOpen(false)}
            >
              Cancel
            </Button>
            <Button
              onClick={() => updateMutation.mutate()}
              disabled={updateMutation.isPending}
            >
              {updateMutation.isPending ? "Saving..." : "Save"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Environment Confirmation */}
      <AlertDialog
        open={isDeleteDialogOpen}
        onOpenChange={setIsDeleteDialogOpen}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete environment?</AlertDialogTitle>
            <AlertDialogDescription>
              This will delete the "{selectedEnv?.name}" environment and all its
              environment variables. Apps assigned to this environment will become
              unassigned. This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => deleteMutation.mutate()}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {deleteMutation.isPending ? "Deleting..." : "Delete"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Clone Environment Dialog */}
      <Dialog
        open={isCloneDialogOpen}
        onOpenChange={(open) => {
          setIsCloneDialogOpen(open);
          if (!open) {
            setSelectedEnv(null);
            setCloneName("");
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Clone Environment</DialogTitle>
            <DialogDescription>
              Create a new environment as a copy of "{selectedEnv?.name}". All
              apps, environment variables, volumes, databases, and services will
              be duplicated with fresh IDs. Domains are cleared and containers
              are not started — the new environment starts clean.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="clone-env-name">New environment name</Label>
              <Input
                id="clone-env-name"
                placeholder="e.g., staging, qa, testing"
                value={cloneName}
                onChange={(e) => setCloneName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && cloneName.trim() && !cloneMutation.isPending) {
                    cloneMutation.mutate();
                  }
                }}
              />
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setIsCloneDialogOpen(false);
                setSelectedEnv(null);
                setCloneName("");
              }}
            >
              Cancel
            </Button>
            <Button
              onClick={() => cloneMutation.mutate()}
              disabled={!cloneName.trim() || cloneMutation.isPending}
            >
              <Copy className="mr-2 h-4 w-4" />
              {cloneMutation.isPending ? "Cloning..." : "Clone Environment"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
