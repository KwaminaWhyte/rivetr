import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Form, useNavigation, Link } from "react-router";
import type { Route } from "./+types/_index";
import { Plus, Play, Square, Trash2, Layers, ExternalLink } from "lucide-react";
import { toast } from "sonner";
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
  DialogTrigger,
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
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import type { Service, ServiceStatus } from "@/types/api";

export function meta() {
  return [
    { title: "Services - Rivetr" },
    { name: "description", content: "Manage Docker Compose services" },
  ];
}

export async function loader({ request }: Route.LoaderArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const services = await api.getServices(token).catch(() => []);

  return { services, token };
}

export async function action({ request }: Route.ActionArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  if (intent === "create") {
    const name = formData.get("name");
    const composeContent = formData.get("compose_content");

    if (typeof name !== "string" || !name.trim()) {
      return { error: "Service name is required" };
    }
    if (typeof composeContent !== "string" || !composeContent.trim()) {
      return { error: "Docker Compose content is required" };
    }

    try {
      const service = await api.createService(token, {
        name: name.trim(),
        compose_content: composeContent.trim(),
      });
      return { success: true, service };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to create service" };
    }
  }

  if (intent === "delete") {
    const serviceId = formData.get("service_id");
    if (typeof serviceId !== "string") {
      return { error: "Service ID is required" };
    }
    try {
      await api.deleteService(token, serviceId);
      return { success: true, deleted: true };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to delete service" };
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

const DEFAULT_COMPOSE = `version: "3.8"
services:
  app:
    image: nginx:alpine
    ports:
      - "80"
`;

export default function ServicesPage({ loaderData, actionData }: Route.ComponentProps) {
  const queryClient = useQueryClient();
  const navigation = useNavigation();
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false);

  const { data: services = [] } = useQuery<Service[]>({
    queryKey: ["services"],
    queryFn: () => api.getServices(loaderData.token),
    initialData: loaderData.services,
  });

  const startMutation = useMutation({
    mutationFn: (id: string) => api.startService(id, loaderData.token),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["services"] });
      toast.success("Service started");
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to start service");
    },
  });

  const stopMutation = useMutation({
    mutationFn: (id: string) => api.stopService(id, loaderData.token),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["services"] });
      toast.success("Service stopped");
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to stop service");
    },
  });

  const isSubmitting = navigation.state === "submitting";

  // Close dialog on successful creation
  if (actionData?.success && isCreateDialogOpen) {
    setIsCreateDialogOpen(false);
    queryClient.invalidateQueries({ queryKey: ["services"] });
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Services</h1>
          <p className="text-muted-foreground">
            Deploy and manage Docker Compose services
          </p>
        </div>

        <div className="flex gap-2">
          <Button variant="outline" asChild>
            <Link to="/templates">
              <Layers className="mr-2 h-4 w-4" />
              Browse Templates
            </Link>
          </Button>
          <Dialog open={isCreateDialogOpen} onOpenChange={setIsCreateDialogOpen}>
            <DialogTrigger asChild>
              <Button>
                <Plus className="mr-2 h-4 w-4" />
                New Service
              </Button>
            </DialogTrigger>
            <DialogContent className="max-w-2xl">
              <Form method="post">
                <input type="hidden" name="intent" value="create" />
                <DialogHeader>
                  <DialogTitle>Create Docker Compose Service</DialogTitle>
                  <DialogDescription>
                    Deploy a multi-container application using Docker Compose.
                  </DialogDescription>
                </DialogHeader>
                {actionData?.error && (
                  <div className="p-3 rounded-md bg-destructive/10 text-destructive text-sm">
                    {actionData.error}
                  </div>
                )}
                <div className="space-y-4 py-4">
                  <div className="space-y-2">
                    <Label htmlFor="service-name">Service Name</Label>
                    <Input
                      id="service-name"
                      name="name"
                      placeholder="my-service"
                      required
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="compose-content">Docker Compose Content</Label>
                    <Textarea
                      id="compose-content"
                      name="compose_content"
                      placeholder="Paste your docker-compose.yml content..."
                      className="font-mono text-sm"
                      rows={12}
                      defaultValue={DEFAULT_COMPOSE}
                      required
                    />
                  </div>
                </div>
                <DialogFooter>
                  <Button
                    type="button"
                    variant="outline"
                    onClick={() => setIsCreateDialogOpen(false)}
                  >
                    Cancel
                  </Button>
                  <Button type="submit" disabled={isSubmitting}>
                    {isSubmitting ? "Creating..." : "Create Service"}
                  </Button>
                </DialogFooter>
              </Form>
            </DialogContent>
          </Dialog>
        </div>
      </div>

      {/* Services Grid */}
      {services.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center">
            <Layers className="mx-auto h-12 w-12 text-muted-foreground mb-4" />
            <p className="text-muted-foreground mb-4">
              No services yet. Create a service or deploy from a template.
            </p>
            <div className="flex justify-center gap-2">
              <Button variant="outline" asChild>
                <Link to="/templates">Browse Templates</Link>
              </Button>
              <Button onClick={() => setIsCreateDialogOpen(true)}>
                <Plus className="mr-2 h-4 w-4" />
                Create Service
              </Button>
            </div>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
          {services.map((service) => (
            <Card key={service.id} className="relative">
              <CardHeader className="pb-3">
                <div className="flex items-start justify-between">
                  <div className="space-y-1">
                    <CardTitle className="text-lg">
                      <Link
                        to={`/services/${service.id}`}
                        className="hover:underline"
                      >
                        {service.name}
                      </Link>
                    </CardTitle>
                    <CardDescription className="text-xs">
                      Created {new Date(service.created_at).toLocaleDateString()}
                    </CardDescription>
                  </div>
                  <Badge variant={getStatusColor(service.status)}>
                    {getStatusLabel(service.status)}
                  </Badge>
                </div>
              </CardHeader>
              <CardContent>
                {service.error_message && (
                  <div className="mb-3 p-2 rounded bg-destructive/10 text-destructive text-xs">
                    {service.error_message}
                  </div>
                )}
                <div className="flex items-center justify-between">
                  <div className="flex gap-2">
                    {service.status === "running" ? (
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => stopMutation.mutate(service.id)}
                        disabled={stopMutation.isPending}
                      >
                        <Square className="h-3 w-3 mr-1" />
                        Stop
                      </Button>
                    ) : (
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => startMutation.mutate(service.id)}
                        disabled={startMutation.isPending}
                      >
                        <Play className="h-3 w-3 mr-1" />
                        Start
                      </Button>
                    )}
                    <Button size="sm" variant="ghost" asChild>
                      <Link to={`/services/${service.id}`}>
                        <ExternalLink className="h-3 w-3" />
                      </Link>
                    </Button>
                  </div>
                  <AlertDialog>
                    <AlertDialogTrigger asChild>
                      <Button size="sm" variant="ghost" className="text-destructive">
                        <Trash2 className="h-3 w-3" />
                      </Button>
                    </AlertDialogTrigger>
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
                          <input type="hidden" name="service_id" value={service.id} />
                          <AlertDialogAction type="submit" className="bg-destructive text-destructive-foreground hover:bg-destructive/90">
                            Delete
                          </AlertDialogAction>
                        </Form>
                      </AlertDialogFooter>
                    </AlertDialogContent>
                  </AlertDialog>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
