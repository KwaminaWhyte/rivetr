import { useState, useMemo } from "react";
import { Form, Link, redirect, useNavigation } from "react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import type { Route } from "./+types/$id";
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
import { EnvironmentBadge } from "@/components/environment-badge";
import { Badge } from "@/components/ui/badge";
import type { App, ProjectWithApps, UpdateProjectRequest } from "@/types/api";

// Status badge component
function StatusBadge({ status }: { status: string }) {
  const variants: Record<string, { className: string; label: string }> = {
    running: { className: "bg-green-500 text-white", label: "Running" },
    stopped: { className: "bg-gray-500 text-white", label: "Stopped" },
    not_deployed: { className: "bg-gray-400 text-white", label: "Not Deployed" },
    failed: { className: "bg-red-500 text-white", label: "Failed" },
    building: { className: "bg-blue-500 text-white", label: "Building" },
    pending: { className: "bg-yellow-500 text-white", label: "Pending" },
  };
  const variant = variants[status] || variants.stopped;
  return <Badge className={variant.className}>{variant.label}</Badge>;
}

export async function loader({ request, params }: Route.LoaderArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const [project, allApps] = await Promise.all([
    api.getProject(token, params.id!),
    api.getApps(token).catch(() => []),
  ]);

  // Get app statuses for apps in this project
  const appStatuses: Record<string, string> = {};
  await Promise.all(
    project.apps.map(async (app) => {
      try {
        const status = await api.getAppStatus(token, app.id);
        appStatuses[app.id] = status.status;
      } catch {
        appStatuses[app.id] = "stopped";
      }
    })
  );

  return { project, allApps, appStatuses };
}

export async function action({ request, params }: Route.ActionArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  if (intent === "delete") {
    await api.deleteProject(token, params.id!);
    return redirect("/projects");
  }

  if (intent === "update") {
    const name = formData.get("name");
    const description = formData.get("description");

    if (typeof name !== "string" || !name.trim()) {
      return { error: "Project name is required" };
    }

    try {
      await api.updateProject(token, params.id!, {
        name: name.trim(),
        description: typeof description === "string" ? description : undefined,
      });
      return { success: true };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to update project" };
    }
  }

  if (intent === "assign-app") {
    const appId = formData.get("appId");
    if (typeof appId !== "string") {
      return { error: "App ID is required" };
    }
    try {
      await api.assignAppToProject(token, appId, params.id!);
      return { success: true };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to add app" };
    }
  }

  if (intent === "remove-app") {
    const appId = formData.get("appId");
    if (typeof appId !== "string") {
      return { error: "App ID is required" };
    }
    try {
      await api.assignAppToProject(token, appId, null);
      return { success: true };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to remove app" };
    }
  }

  return { error: "Unknown action" };
}

export default function ProjectDetailPage({ loaderData, actionData }: Route.ComponentProps) {
  const queryClient = useQueryClient();
  const navigation = useNavigation();
  const [isEditDialogOpen, setIsEditDialogOpen] = useState(false);
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
  const [isAddAppDialogOpen, setIsAddAppDialogOpen] = useState(false);
  const [editData, setEditData] = useState<UpdateProjectRequest>({});

  // Use React Query with SSR initial data
  const { data: project } = useQuery<ProjectWithApps>({
    queryKey: ["project", loaderData.project.id],
    queryFn: () => api.getProject(loaderData.project.id),
    initialData: loaderData.project,
  });

  const { data: allApps = [] } = useQuery<App[]>({
    queryKey: ["apps"],
    queryFn: () => api.getApps(),
    initialData: loaderData.allApps,
  });

  // Apps available to add (not in this project)
  const availableApps = useMemo(() => {
    if (!project) return [];
    const projectAppIds = new Set(project.apps.map((a) => a.id));
    return allApps.filter((app) => !app.project_id && !projectAppIds.has(app.id));
  }, [allApps, project]);

  const isSubmitting = navigation.state === "submitting";

  const openEditDialog = () => {
    if (project) {
      setEditData({
        name: project.name,
        description: project.description || "",
      });
      setIsEditDialogOpen(true);
    }
  };

  // Close dialogs on success
  if (actionData?.success) {
    if (isEditDialogOpen) setIsEditDialogOpen(false);
    if (isAddAppDialogOpen) setIsAddAppDialogOpen(false);
    queryClient.invalidateQueries({ queryKey: ["project", project?.id] });
    queryClient.invalidateQueries({ queryKey: ["projects"] });
    queryClient.invalidateQueries({ queryKey: ["apps"] });
  }

  if (!project) {
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
              <Link to={`/projects/${project.id}/apps/new`}>
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
                  <Link to={`/projects/${project.id}/apps/new`}>
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
                  <TableHead>Status</TableHead>
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
                      <StatusBadge status={loaderData.appStatuses?.[app.id] || "stopped"} />
                    </TableCell>
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
                        <Form method="post">
                          <input type="hidden" name="intent" value="remove-app" />
                          <input type="hidden" name="appId" value={app.id} />
                          <Button
                            type="submit"
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8 text-muted-foreground hover:text-destructive"
                            disabled={isSubmitting}
                            title="Remove from project"
                          >
                            <X className="h-4 w-4" />
                          </Button>
                        </Form>
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
          <Form method="post">
            <input type="hidden" name="intent" value="update" />
            <DialogHeader>
              <DialogTitle>Edit Project</DialogTitle>
              <DialogDescription>
                Update your project details.
              </DialogDescription>
            </DialogHeader>
            {actionData?.error && (
              <div className="p-3 rounded-md bg-destructive/10 text-destructive text-sm">
                {actionData.error}
              </div>
            )}
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <Label htmlFor="edit-name">Name</Label>
                <Input
                  id="edit-name"
                  name="name"
                  defaultValue={editData.name || ""}
                  required
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="edit-description">Description</Label>
                <Textarea
                  id="edit-description"
                  name="description"
                  defaultValue={editData.description || ""}
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
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? "Saving..." : "Save Changes"}
              </Button>
            </DialogFooter>
          </Form>
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
            <Form method="post">
              <input type="hidden" name="intent" value="delete" />
              <AlertDialogAction
                type="submit"
                className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              >
                {isSubmitting ? "Deleting..." : "Delete"}
              </AlertDialogAction>
            </Form>
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
                  <Link to={`/projects/${project.id}/apps/new`}>Create New App</Link>
                </Button>
              </div>
            ) : (
              <div className="space-y-2 max-h-80 overflow-y-auto">
                {availableApps.map((app) => (
                  <Form method="post" key={app.id}>
                    <input type="hidden" name="intent" value="assign-app" />
                    <input type="hidden" name="appId" value={app.id} />
                    <button
                      type="submit"
                      className="flex items-center justify-between w-full p-3 rounded-lg border hover:bg-muted/50 cursor-pointer text-left"
                      disabled={isSubmitting}
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
                        <Plus className="h-4 w-4" />
                      </div>
                    </button>
                  </Form>
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
