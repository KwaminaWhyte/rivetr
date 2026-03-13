import { useOutletContext } from "react-router";
import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Switch } from "@/components/ui/switch";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Plus, Trash2, FileCode, Pencil } from "lucide-react";
import { api } from "@/lib/api";
import type { App, AppPatch } from "@/types/api";

export function meta() {
  return [
    { title: "Deployment Patches - Rivetr" },
    { name: "description", content: "Inject files into your application before each build" },
  ];
}

const OPERATION_LABELS: Record<string, string> = {
  create: "Create / Overwrite",
  append: "Append",
  delete: "Delete",
};

export default function AppSettingsPatches() {
  const { app } = useOutletContext<{ app: App }>();
  const queryClient = useQueryClient();
  const [newFilePath, setNewFilePath] = useState("");
  const [newContent, setNewContent] = useState("");
  const [newOperation, setNewOperation] = useState<"create" | "append" | "delete">("create");
  const [isAdding, setIsAdding] = useState(false);
  const [editingPatch, setEditingPatch] = useState<AppPatch | null>(null);

  const { data: patches = [], isLoading } = useQuery<AppPatch[]>({
    queryKey: ["app-patches", app.id],
    queryFn: () => api.listPatches(app.id),
  });

  const createMutation = useMutation({
    mutationFn: (data: { file_path: string; content: string; operation: "create" | "append" | "delete" }) =>
      api.createPatch(app.id, data),
    onSuccess: () => {
      toast.success("Patch created. It will be applied on the next deployment.");
      setNewFilePath("");
      setNewContent("");
      setNewOperation("create");
      setIsAdding(false);
      queryClient.invalidateQueries({ queryKey: ["app-patches", app.id] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to create patch");
    },
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: string; data: { file_path?: string; content?: string; operation?: "create" | "append" | "delete"; is_enabled?: boolean } }) =>
      api.updatePatch(app.id, id, data),
    onSuccess: () => {
      toast.success("Patch updated.");
      setEditingPatch(null);
      setNewFilePath("");
      setNewContent("");
      setNewOperation("create");
      queryClient.invalidateQueries({ queryKey: ["app-patches", app.id] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to update patch");
    },
  });

  const toggleMutation = useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) =>
      api.updatePatch(app.id, id, { is_enabled: enabled }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["app-patches", app.id] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to update patch");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => api.deletePatch(app.id, id),
    onSuccess: () => {
      toast.success("Patch deleted");
      queryClient.invalidateQueries({ queryKey: ["app-patches", app.id] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to delete patch");
    },
  });

  const handleCreate = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newFilePath.trim()) return;
    createMutation.mutate({ file_path: newFilePath.trim(), content: newContent, operation: newOperation });
  };

  const handleUpdate = (e: React.FormEvent) => {
    e.preventDefault();
    if (!editingPatch || !newFilePath.trim()) return;
    updateMutation.mutate({
      id: editingPatch.id,
      data: { file_path: newFilePath.trim(), content: newContent, operation: newOperation },
    });
  };

  const openEdit = (patch: AppPatch) => {
    setEditingPatch(patch);
    setNewFilePath(patch.file_path);
    setNewContent(patch.content);
    setNewOperation(patch.operation);
    setIsAdding(false);
  };

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <FileCode className="h-5 w-5" />
            Deployment Patches
          </CardTitle>
          <CardDescription>
            Inject files into your application&apos;s source directory before each build. Useful for overriding
            config files, adding secrets, or patching code without modifying the repository.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {isLoading ? (
            <p className="text-sm text-muted-foreground">Loading patches…</p>
          ) : patches.length === 0 && !isAdding ? (
            <p className="text-sm text-muted-foreground">
              No patches configured. Add a patch to inject a file before every build.
            </p>
          ) : null}

          {patches.map((patch) => (
            <div key={patch.id} className="flex items-start justify-between gap-4 p-4 border rounded-lg">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1">
                  <code className="text-sm font-mono truncate">{patch.file_path}</code>
                  <Badge variant={patch.is_enabled ? "default" : "secondary"}>
                    {patch.operation}
                  </Badge>
                </div>
                <pre className="text-xs text-muted-foreground bg-muted p-2 rounded max-h-24 overflow-auto">
                  {patch.content}
                </pre>
              </div>
              <div className="flex items-center gap-2 flex-shrink-0">
                <Switch
                  checked={patch.is_enabled}
                  onCheckedChange={(checked) => toggleMutation.mutate({ id: patch.id, enabled: checked })}
                  disabled={toggleMutation.isPending}
                />
                <AlertDialog>
                  <AlertDialogTrigger asChild>
                    <Button variant="ghost" size="icon" className="text-destructive hover:text-destructive">
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </AlertDialogTrigger>
                  <AlertDialogContent>
                    <AlertDialogHeader>
                      <AlertDialogTitle>Delete Patch</AlertDialogTitle>
                      <AlertDialogDescription>
                        This will permanently delete the patch for <code>{patch.file_path}</code>.
                      </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                      <AlertDialogCancel>Cancel</AlertDialogCancel>
                      <AlertDialogAction
                        onClick={() => deleteMutation.mutate(patch.id)}
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

          {isAdding ? (
            <form onSubmit={handleCreate} className="space-y-4 p-4 border rounded-lg">
              <div className="space-y-2">
                <Label htmlFor="file-path">File Path</Label>
                <Input
                  id="file-path"
                  placeholder="e.g. config/production.json or .env"
                  value={newFilePath}
                  onChange={(e) => setNewFilePath(e.target.value)}
                  required
                />
                <p className="text-xs text-muted-foreground">
                  Relative to the root of your repository.
                </p>
              </div>
              <div className="space-y-2">
                <Label htmlFor="patch-content">File Content</Label>
                <Textarea
                  id="patch-content"
                  placeholder="Enter the file contents…"
                  value={newContent}
                  onChange={(e) => setNewContent(e.target.value)}
                  className="font-mono text-sm min-h-[120px] resize-y"
                  required
                />
              </div>
              <div className="flex justify-end gap-2">
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => { setIsAdding(false); setNewFilePath(""); setNewContent(""); }}
                >
                  Cancel
                </Button>
                <Button type="submit" disabled={createMutation.isPending}>
                  {createMutation.isPending ? "Creating…" : "Create Patch"}
                </Button>
              </div>
            </form>
          ) : (
            <Button
              variant="outline"
              className="gap-2"
              onClick={() => setIsAdding(true)}
            >
              <Plus className="h-4 w-4" />
              Add Patch
            </Button>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
