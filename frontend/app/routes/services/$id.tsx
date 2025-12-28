import { useState, useEffect } from "react";
import { Form, Link, redirect, useNavigation } from "react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import type { Route } from "./+types/$id";
import { toast } from "sonner";
import {
  ArrowLeft,
  Edit2,
  MoreVertical,
  Play,
  Square,
  Trash2,
  Save,
  FileCode,
} from "lucide-react";
import { api } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
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
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import type { Service, ServiceStatus } from "@/types/api";

export function meta({ data }: Route.MetaArgs) {
  const serviceName = data?.service?.name || "Service";
  return [
    { title: `${serviceName} - Rivetr` },
    { name: "description", content: `Manage ${serviceName} Docker Compose service` },
  ];
}

export async function loader({ request, params }: Route.LoaderArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const service = await api.getService(token, params.id!);

  return { service, token };
}

export async function action({ request, params }: Route.ActionArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  if (intent === "delete") {
    await api.deleteService(token, params.id!);
    return redirect("/services");
  }

  if (intent === "update") {
    const name = formData.get("name");
    const composeContent = formData.get("compose_content");

    if (typeof name !== "string" || !name.trim()) {
      return { error: "Service name is required" };
    }

    try {
      await api.updateService(token, params.id!, {
        name: name.trim(),
        compose_content: typeof composeContent === "string" ? composeContent.trim() : undefined,
      });
      return { success: true, action: "update" };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to update service" };
    }
  }

  if (intent === "start") {
    try {
      await api.startService(token, params.id!);
      return { success: true, action: "start" };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to start service" };
    }
  }

  if (intent === "stop") {
    try {
      await api.stopService(token, params.id!);
      return { success: true, action: "stop" };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to stop service" };
    }
  }

  return { error: "Unknown action" };
}

function getStatusColor(status: ServiceStatus) {
  switch (status) {
    case "running":
      return "default";
    case "stopped":
      return "secondary";
    case "failed":
      return "destructive";
    case "pending":
      return "outline";
    default:
      return "secondary";
  }
}

function getStatusLabel(status: ServiceStatus) {
  switch (status) {
    case "running":
      return "Running";
    case "stopped":
      return "Stopped";
    case "failed":
      return "Failed";
    case "pending":
      return "Pending";
    default:
      return status;
  }
}

export default function ServiceDetailPage({ loaderData, actionData }: Route.ComponentProps) {
  const queryClient = useQueryClient();
  const navigation = useNavigation();
  const [isEditDialogOpen, setIsEditDialogOpen] = useState(false);
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
  const [editName, setEditName] = useState("");
  const [editCompose, setEditCompose] = useState("");

  const { data: service, refetch } = useQuery<Service>({
    queryKey: ["service", loaderData.service.id],
    queryFn: () => api.getService(loaderData.service.id, loaderData.token),
    initialData: loaderData.service,
    refetchInterval: 5000, // Poll for status updates
  });

  const isSubmitting = navigation.state === "submitting";

  const openEditDialog = () => {
    if (service) {
      setEditName(service.name);
      setEditCompose(service.compose_content);
      setIsEditDialogOpen(true);
    }
  };

  // Handle success actions
  useEffect(() => {
    if (actionData?.success) {
      if (actionData.action === "update") {
        toast.success("Service updated");
        setIsEditDialogOpen(false);
      } else if (actionData.action === "start") {
        toast.success("Service starting");
      } else if (actionData.action === "stop") {
        toast.success("Service stopped");
      }
      queryClient.invalidateQueries({ queryKey: ["service", service?.id] });
      queryClient.invalidateQueries({ queryKey: ["services"] });
      refetch();
    }

    if (actionData?.error) {
      toast.error(actionData.error);
    }
  }, [actionData, queryClient, service?.id, refetch]);

  if (!service) {
    return (
      <div className="space-y-6">
        <Button variant="ghost" asChild>
          <Link to="/services">
            <ArrowLeft className="mr-2 h-4 w-4" />
            Back to Services
          </Link>
        </Button>
        <Card>
          <CardContent className="py-8 text-center text-destructive">
            Failed to load service. It may have been deleted.
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
          <Link to="/services">
            <ArrowLeft className="h-4 w-4" />
          </Link>
        </Button>
        <div className="flex-1">
          <div className="flex items-center gap-3">
            <h1 className="text-3xl font-bold">{service.name}</h1>
            <Badge variant={getStatusColor(service.status)}>
              {getStatusLabel(service.status)}
            </Badge>
          </div>
          <p className="text-muted-foreground text-sm">
            Created {new Date(service.created_at).toLocaleDateString()}
            {service.updated_at !== service.created_at && (
              <> &middot; Updated {new Date(service.updated_at).toLocaleDateString()}</>
            )}
          </p>
        </div>

        {/* Action Buttons */}
        <div className="flex items-center gap-2">
          {service.status === "running" ? (
            <Form method="post">
              <input type="hidden" name="intent" value="stop" />
              <Button variant="outline" type="submit" disabled={isSubmitting}>
                <Square className="mr-2 h-4 w-4" />
                Stop
              </Button>
            </Form>
          ) : (
            <Form method="post">
              <input type="hidden" name="intent" value="start" />
              <Button type="submit" disabled={isSubmitting}>
                <Play className="mr-2 h-4 w-4" />
                Start
              </Button>
            </Form>
          )}
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
                Edit Service
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                className="text-destructive focus:text-destructive"
                onClick={() => setIsDeleteDialogOpen(true)}
              >
                <Trash2 className="mr-2 h-4 w-4" />
                Delete Service
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>

      {/* Error Message */}
      {service.error_message && (
        <Card className="border-destructive">
          <CardContent className="py-4">
            <p className="text-destructive text-sm">{service.error_message}</p>
          </CardContent>
        </Card>
      )}

      {/* Docker Compose Content */}
      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <FileCode className="h-5 w-5" />
            <CardTitle>Docker Compose Configuration</CardTitle>
          </div>
          <CardDescription>
            The docker-compose.yml content for this service
          </CardDescription>
        </CardHeader>
        <CardContent>
          <pre className="bg-muted p-4 rounded-lg overflow-x-auto text-sm font-mono whitespace-pre-wrap">
            {service.compose_content}
          </pre>
        </CardContent>
      </Card>

      {/* Edit Dialog */}
      <Dialog open={isEditDialogOpen} onOpenChange={setIsEditDialogOpen}>
        <DialogContent className="max-w-2xl">
          <Form method="post">
            <input type="hidden" name="intent" value="update" />
            <DialogHeader>
              <DialogTitle>Edit Service</DialogTitle>
              <DialogDescription>
                Update service name or Docker Compose configuration. Changing the
                compose content requires a restart to take effect.
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
                  value={editName}
                  onChange={(e) => setEditName(e.target.value)}
                  required
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="edit-compose">Docker Compose Content</Label>
                <Textarea
                  id="edit-compose"
                  name="compose_content"
                  value={editCompose}
                  onChange={(e) => setEditCompose(e.target.value)}
                  className="font-mono text-sm"
                  rows={12}
                  required
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
                <Save className="mr-2 h-4 w-4" />
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
            <AlertDialogTitle>Delete Service</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete "{service.name}"? This will stop all
              containers and remove all data. This action cannot be undone.
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
                {isSubmitting ? "Deleting..." : "Delete Service"}
              </AlertDialogAction>
            </Form>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
