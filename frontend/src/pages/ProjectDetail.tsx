import { useState, useMemo } from "react";
import { useParams, useNavigate, Link } from "react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  ArrowLeft,
  Edit2,
  ExternalLink,
  MoreVertical,
  Plus,
  Trash2,
  X,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
// Note: Badge removed - use EnvironmentBadge instead
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
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
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
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { EnvironmentBadge } from "@/components/EnvironmentBadge";
import { api } from "@/lib/api";
import type { App, ProjectWithApps, UpdateProjectRequest } from "@/types/api";

export function ProjectDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const [isEditDialogOpen, setIsEditDialogOpen] = useState(false);
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
  const [isAddAppDialogOpen, setIsAddAppDialogOpen] = useState(false);
  const [editData, setEditData] = useState<UpdateProjectRequest>({});

  // Fetch project with apps
  const {
    data: project,
    isLoading,
    error,
  } = useQuery<ProjectWithApps>({
    queryKey: ["project", id],
    queryFn: () => api.getProject(id!),
    enabled: !!id,
  });

  // Fetch all apps for the "add app" dialog
  const { data: allApps = [] } = useQuery<App[]>({
    queryKey: ["apps"],
    queryFn: () => api.getApps(),
  });

  // Apps available to add (not in this project)
  const availableApps = useMemo(() => {
    if (!project) return [];
    const projectAppIds = new Set(project.apps.map((a) => a.id));
    return allApps.filter((app) => !app.project_id && !projectAppIds.has(app.id));
  }, [allApps, project]);

  // Update project mutation
  const updateMutation = useMutation({
    mutationFn: (data: UpdateProjectRequest) => api.updateProject(id!, data),
    onSuccess: (updated) => {
      toast.success(`Project "${updated.name}" updated`);
      queryClient.invalidateQueries({ queryKey: ["project", id] });
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      setIsEditDialogOpen(false);
    },
    onError: (err: Error) => {
      toast.error(`Failed to update project: ${err.message}`);
    },
  });

  // Delete project mutation
  const deleteMutation = useMutation({
    mutationFn: () => api.deleteProject(id!),
    onSuccess: () => {
      toast.success("Project deleted");
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      navigate("/projects");
    },
    onError: (err: Error) => {
      toast.error(`Failed to delete project: ${err.message}`);
    },
  });

  // Assign app to project mutation
  const assignAppMutation = useMutation({
    mutationFn: (appId: string) => api.assignAppToProject(appId, id!),
    onSuccess: () => {
      toast.success("App added to project");
      queryClient.invalidateQueries({ queryKey: ["project", id] });
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      queryClient.invalidateQueries({ queryKey: ["apps"] });
      setIsAddAppDialogOpen(false);
    },
    onError: (err: Error) => {
      toast.error(`Failed to add app: ${err.message}`);
    },
  });

  // Remove app from project mutation
  const removeAppMutation = useMutation({
    mutationFn: (appId: string) => api.assignAppToProject(appId, null),
    onSuccess: () => {
      toast.success("App removed from project");
      queryClient.invalidateQueries({ queryKey: ["project", id] });
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      queryClient.invalidateQueries({ queryKey: ["apps"] });
    },
    onError: (err: Error) => {
      toast.error(`Failed to remove app: ${err.message}`);
    },
  });

  const handleEditSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    updateMutation.mutate(editData);
  };

  const openEditDialog = () => {
    if (project) {
      setEditData({
        name: project.name,
        description: project.description || "",
      });
      setIsEditDialogOpen(true);
    }
  };

  if (isLoading) {
    return (
      <div className="space-y-6">
        <div className="flex items-center gap-4">
          <Skeleton className="h-10 w-10" />
          <div>
            <Skeleton className="h-8 w-48 mb-2" />
            <Skeleton className="h-4 w-64" />
          </div>
        </div>
        <Card>
          <CardContent className="p-6">
            <Skeleton className="h-64 w-full" />
          </CardContent>
        </Card>
      </div>
    );
  }

  if (error || !project) {
    return (
      <div className="space-y-6">
        <Button variant="ghost" asChild>
          <Link to="/projects">
            <ArrowLeft className="mr-2 h-4 w-4" />
            Back to Projects
          </Link>
        </Button>
        <Card>
          <CardContent className="py-8 text-center text-destructive">
            Failed to load project. It may have been deleted.
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="icon" asChild>
          <Link to="/projects">
            <ArrowLeft className="h-4 w-4" />
          </Link>
        </Button>
        <div className="flex-1">
          <h1 className="text-3xl font-bold">{project.name}</h1>
          {project.description && (
            <p className="text-muted-foreground">{project.description}</p>
          )}
        </div>
        <Button variant="outline" onClick={openEditDialog}>
          <Edit2 className="mr-2 h-4 w-4" />
          Edit
        </Button>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="icon">
              <MoreVertical className="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onClick={openEditDialog}>
              <Edit2 className="mr-2 h-4 w-4" />
              Edit Project
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem
              className="text-destructive focus:text-destructive"
              onClick={() => setIsDeleteDialogOpen(true)}
            >
              <Trash2 className="mr-2 h-4 w-4" />
              Delete Project
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      {/* Apps Table */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>Applications</CardTitle>
          <div className="flex gap-2">
            <Button variant="outline" onClick={() => setIsAddAppDialogOpen(true)}>
              <Plus className="mr-2 h-4 w-4" />
              Add Existing App
            </Button>
            <Button asChild>
              <Link to={`/projects/${id}/apps/new`}>
                <Plus className="mr-2 h-4 w-4" />
                Create New App
              </Link>
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {project.apps.length === 0 ? (
            <div className="py-8 text-center">
              <p className="text-muted-foreground mb-4">
                No applications in this project yet.
              </p>
              <div className="flex gap-2 justify-center">
                <Button variant="outline" onClick={() => setIsAddAppDialogOpen(true)}>
                  <Plus className="mr-2 h-4 w-4" />
                  Add Existing
                </Button>
                <Button asChild>
                  <Link to={`/projects/${id}/apps/new`}>
                    <Plus className="mr-2 h-4 w-4" />
                    Create New App
                  </Link>
                </Button>
              </div>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Environment</TableHead>
                  <TableHead>Repository</TableHead>
                  <TableHead>Domain</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {project.apps.map((app) => (
                  <TableRow key={app.id}>
                    <TableCell className="font-medium">{app.name}</TableCell>
                    <TableCell>
                      <EnvironmentBadge environment={app.environment} />
                    </TableCell>
                    <TableCell className="text-muted-foreground max-w-xs truncate">
                      {app.git_url}
                    </TableCell>
                    <TableCell>{app.domain || "-"}</TableCell>
                    <TableCell className="text-right">
                      <div className="flex items-center justify-end gap-2">
                        <Button variant="ghost" size="sm" asChild>
                          <Link to={`/apps/${app.id}`}>
                            <ExternalLink className="mr-1 h-3 w-3" />
                            View
                          </Link>
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8 text-muted-foreground hover:text-destructive"
                          onClick={() => removeAppMutation.mutate(app.id)}
                          disabled={removeAppMutation.isPending}
                          title="Remove from project"
                        >
                          <X className="h-4 w-4" />
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

      {/* Edit Dialog */}
      <Dialog open={isEditDialogOpen} onOpenChange={setIsEditDialogOpen}>
        <DialogContent>
          <form onSubmit={handleEditSubmit}>
            <DialogHeader>
              <DialogTitle>Edit Project</DialogTitle>
              <DialogDescription>
                Update your project details.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <Label htmlFor="edit-name">Name</Label>
                <Input
                  id="edit-name"
                  value={editData.name || ""}
                  onChange={(e) =>
                    setEditData((prev) => ({ ...prev, name: e.target.value }))
                  }
                  required
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="edit-description">Description</Label>
                <Textarea
                  id="edit-description"
                  value={editData.description || ""}
                  onChange={(e) =>
                    setEditData((prev) => ({
                      ...prev,
                      description: e.target.value,
                    }))
                  }
                  rows={3}
                />
              </div>
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => setIsEditDialogOpen(false)}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={updateMutation.isPending}>
                {updateMutation.isPending ? "Saving..." : "Save Changes"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <AlertDialog open={isDeleteDialogOpen} onOpenChange={setIsDeleteDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Project</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete "{project.name}"? This will not
              delete the apps, but they will be unassigned from this project.
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

      {/* Add App Dialog */}
      <Dialog open={isAddAppDialogOpen} onOpenChange={setIsAddAppDialogOpen}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Add App to Project</DialogTitle>
            <DialogDescription>
              Select an unassigned app to add to this project.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4">
            {availableApps.length === 0 ? (
              <div className="text-center py-8">
                <p className="text-muted-foreground mb-4">
                  No unassigned apps available.
                </p>
                <Button asChild>
                  <Link to="/apps/new">Create New App</Link>
                </Button>
              </div>
            ) : (
              <div className="space-y-2 max-h-80 overflow-y-auto">
                {availableApps.map((app) => (
                  <div
                    key={app.id}
                    className="flex items-center justify-between p-3 rounded-lg border hover:bg-muted/50 cursor-pointer"
                    onClick={() => assignAppMutation.mutate(app.id)}
                  >
                    <div className="flex items-center gap-3">
                      <div>
                        <p className="font-medium">{app.name}</p>
                        <p className="text-sm text-muted-foreground truncate max-w-md">
                          {app.git_url}
                        </p>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <EnvironmentBadge environment={app.environment} />
                      <Button
                        variant="ghost"
                        size="sm"
                        disabled={assignAppMutation.isPending}
                      >
                        <Plus className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setIsAddAppDialogOpen(false)}
            >
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
