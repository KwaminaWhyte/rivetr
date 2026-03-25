import { useState, useEffect } from "react";
import { Link, useNavigate, useParams } from "react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
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
  FileText,
  Network,
  Copy,
  Check,
} from "lucide-react";
import { api } from "@/lib/api";
import { useBreadcrumb } from "@/lib/breadcrumb-context";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ServiceLogs } from "@/components/service-logs";
import { ResourceMonitor } from "@/components/resource-monitor";
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
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import type { Service, ServiceStatus, Project } from "@/types/api";

export function meta() {
  return [
    { title: "Service - Rivetr" },
    { name: "description", content: "Manage Docker Compose service" },
  ];
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

export default function ServiceDetailPage() {
  const { id } = useParams();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { setItems } = useBreadcrumb();
  const serviceId = id!;

  const [isEditDialogOpen, setIsEditDialogOpen] = useState(false);
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
  const [editName, setEditName] = useState("");
  const [editCompose, setEditCompose] = useState("");

  // Public access form state
  const [publicAccess, setPublicAccess] = useState(false);
  const [externalPort, setExternalPort] = useState("");
  const [containerPort, setContainerPort] = useState("");
  const [copiedConn, setCopiedConn] = useState(false);

  const { data: service, isLoading, error } = useQuery<Service>({
    queryKey: ["service", serviceId],
    queryFn: () => api.getService(serviceId),
    refetchInterval: 5000, // Poll for status updates
  });

  // Sync public access form state when service data loads/changes
  useEffect(() => {
    if (service) {
      setPublicAccess(service.public_access);
      setExternalPort(service.external_port > 0 ? String(service.external_port) : "");
      setContainerPort(
        service.expose_container_port > 0 ? String(service.expose_container_port) : ""
      );
    }
  }, [service]);

  // Fetch project for breadcrumb
  const { data: project } = useQuery<Project>({
    queryKey: ["project", service?.project_id],
    queryFn: () => api.getProject(service!.project_id!),
    enabled: !!service?.project_id,
  });

  // Set breadcrumbs when service and project are loaded
  useEffect(() => {
    if (service) {
      const breadcrumbs = [];
      if (project) {
        breadcrumbs.push({ label: project.name, href: `/projects/${project.id}` });
      } else {
        breadcrumbs.push({ label: "Projects", href: "/projects" });
      }
      breadcrumbs.push({ label: "Services" });
      breadcrumbs.push({ label: service.name });
      setItems(breadcrumbs);
    }
  }, [service, project, setItems]);

  // Mutations
  const updateMutation = useMutation({
    mutationFn: (data: { name: string; compose_content?: string }) =>
      api.updateService(serviceId, data),
    onSuccess: () => {
      toast.success("Service updated");
      setIsEditDialogOpen(false);
      queryClient.invalidateQueries({ queryKey: ["service", serviceId] });
      queryClient.invalidateQueries({ queryKey: ["services"] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to update service");
    },
  });

  const publicAccessMutation = useMutation({
    mutationFn: (data: {
      public_access: boolean;
      external_port: number;
      expose_container_port: number;
    }) => api.updateService(serviceId, data),
    onSuccess: () => {
      toast.success("Public access settings saved");
      queryClient.invalidateQueries({ queryKey: ["service", serviceId] });
    },
    onError: (err) => {
      const msg = err instanceof Error ? err.message : "Failed to save public access settings";
      if (msg.includes("409") || msg.toLowerCase().includes("conflict")) {
        toast.error("Port conflict — that port is already in use by another service or database.");
      } else {
        toast.error(msg);
      }
    },
  });

  const startMutation = useMutation({
    mutationFn: () => api.startService(serviceId),
    onSuccess: () => {
      toast.success("Service starting");
      queryClient.invalidateQueries({ queryKey: ["service", serviceId] });
      queryClient.invalidateQueries({ queryKey: ["services"] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to start service");
    },
  });

  const stopMutation = useMutation({
    mutationFn: () => api.stopService(serviceId),
    onSuccess: () => {
      toast.success("Service stopped");
      queryClient.invalidateQueries({ queryKey: ["service", serviceId] });
      queryClient.invalidateQueries({ queryKey: ["services"] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to stop service");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: () => api.deleteService(serviceId),
    onSuccess: () => {
      toast.success("Service deleted");
      navigate("/services");
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to delete service");
    },
  });

  const isSubmitting =
    updateMutation.isPending ||
    startMutation.isPending ||
    stopMutation.isPending ||
    deleteMutation.isPending;

  const openEditDialog = () => {
    if (service) {
      setEditName(service.name);
      setEditCompose(service.compose_content);
      setIsEditDialogOpen(true);
    }
  };

  const handleUpdateSubmit = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const name = formData.get("name") as string;
    const composeContent = formData.get("compose_content") as string;

    if (!name?.trim()) {
      toast.error("Service name is required");
      return;
    }

    updateMutation.mutate({
      name: name.trim(),
      compose_content: composeContent?.trim() || undefined,
    });
  };

  const handlePublicAccessSave = () => {
    const extPort = externalPort ? parseInt(externalPort, 10) : 0;
    const ctrPort = containerPort ? parseInt(containerPort, 10) : 0;

    if (publicAccess) {
      if (!extPort || extPort < 1 || extPort > 65535) {
        toast.error("Please enter a valid host port (1–65535)");
        return;
      }
      if (!ctrPort || ctrPort < 1 || ctrPort > 65535) {
        toast.error("Please enter a valid container port (1–65535)");
        return;
      }
    }

    publicAccessMutation.mutate({
      public_access: publicAccess,
      external_port: extPort,
      expose_container_port: ctrPort,
    });
  };

  const handleDelete = () => {
    deleteMutation.mutate();
  };

  const connectionString =
    service && service.public_access && service.external_port > 0
      ? `${window.location.hostname}:${service.external_port}`
      : null;

  const copyConnectionString = () => {
    if (!connectionString) return;
    navigator.clipboard.writeText(connectionString).then(() => {
      setCopiedConn(true);
      setTimeout(() => setCopiedConn(false), 2000);
    });
  };

  if (isLoading) {
    return (
      <div className="space-y-6">
        <Button variant="ghost" asChild>
          <Link to="/services">
            <ArrowLeft className="mr-2 h-4 w-4" />
            Back to Services
          </Link>
        </Button>
        <div className="animate-pulse space-y-4">
          <div className="h-8 w-48 bg-muted rounded" />
          <div className="h-64 bg-muted rounded" />
        </div>
      </div>
    );
  }

  if (error || !service) {
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
            <Button
              variant="outline"
              onClick={() => stopMutation.mutate()}
              disabled={isSubmitting}
            >
              <Square className="mr-2 h-4 w-4" />
              Stop
            </Button>
          ) : (
            <Button
              onClick={() => startMutation.mutate()}
              disabled={isSubmitting}
            >
              <Play className="mr-2 h-4 w-4" />
              Start
            </Button>
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

      {/* Tabs for Configuration, Network, and Logs */}
      <Tabs defaultValue="config" className="space-y-4">
        <TabsList>
          <TabsTrigger value="config" className="flex items-center gap-2">
            <FileCode className="h-4 w-4" />
            Configuration
          </TabsTrigger>
          <TabsTrigger value="network" className="flex items-center gap-2">
            <Network className="h-4 w-4" />
            Network
          </TabsTrigger>
          <TabsTrigger value="logs" className="flex items-center gap-2">
            <FileText className="h-4 w-4" />
            Logs
          </TabsTrigger>
        </TabsList>

        <TabsContent value="config" className="space-y-4">
          {/* Resource Usage */}
          {service.status === "running" && (
            <ResourceMonitor serviceId={service.id} />
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
        </TabsContent>

        <TabsContent value="network" className="space-y-4">
          {/* Public Access Card */}
          <Card>
            <CardHeader>
              <div className="flex items-center gap-2">
                <Network className="h-5 w-5" />
                <CardTitle>Public Access</CardTitle>
              </div>
              <CardDescription>
                Expose a container port directly on the host so external clients (e.g. database
                tools) can connect to this service without going through the proxy.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              {/* Toggle */}
              <div className="flex items-center justify-between">
                <div>
                  <p className="font-medium text-sm">Enable Public Access</p>
                  <p className="text-muted-foreground text-xs">
                    Binds the container port on the host. The service will be restarted if it is
                    currently running.
                  </p>
                </div>
                <Switch
                  checked={publicAccess}
                  onCheckedChange={setPublicAccess}
                />
              </div>

              {/* Port inputs */}
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label htmlFor="external-port">Host Port</Label>
                  <Input
                    id="external-port"
                    type="number"
                    min={1}
                    max={65535}
                    placeholder="e.g. 6380"
                    value={externalPort}
                    onChange={(e) => setExternalPort(e.target.value)}
                    disabled={!publicAccess}
                  />
                  <p className="text-xs text-muted-foreground">
                    Port on the host machine to listen on
                  </p>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="container-port">Container Port</Label>
                  <Input
                    id="container-port"
                    type="number"
                    min={1}
                    max={65535}
                    placeholder="e.g. 6379"
                    value={containerPort}
                    onChange={(e) => setContainerPort(e.target.value)}
                    disabled={!publicAccess}
                  />
                  <p className="text-xs text-muted-foreground">
                    Port the service listens on inside the container
                  </p>
                </div>
              </div>

              {/* Connection string (shown when public_access is saved) */}
              {connectionString && (
                <div className="space-y-2">
                  <Label>Connection Address</Label>
                  <div className="flex items-center gap-2">
                    <code className="flex-1 bg-muted px-3 py-2 rounded text-sm font-mono truncate">
                      {connectionString}
                    </code>
                    <Button
                      variant="outline"
                      size="icon"
                      onClick={copyConnectionString}
                      title="Copy connection address"
                    >
                      {copiedConn ? (
                        <Check className="h-4 w-4 text-green-600" />
                      ) : (
                        <Copy className="h-4 w-4" />
                      )}
                    </Button>
                  </div>
                  <p className="text-xs text-muted-foreground">
                    Connect to this service from outside using the address above.
                  </p>
                </div>
              )}

              <Button
                onClick={handlePublicAccessSave}
                disabled={publicAccessMutation.isPending}
              >
                <Save className="mr-2 h-4 w-4" />
                {publicAccessMutation.isPending ? "Saving..." : "Save Network Settings"}
              </Button>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="logs" className="space-y-4">
          <ServiceLogs serviceId={service.id} serviceName={service.name} />
        </TabsContent>
      </Tabs>

      {/* Edit Dialog */}
      <Dialog open={isEditDialogOpen} onOpenChange={setIsEditDialogOpen}>
        <DialogContent className="max-w-2xl">
          <form onSubmit={handleUpdateSubmit}>
            <DialogHeader>
              <DialogTitle>Edit Service</DialogTitle>
              <DialogDescription>
                Update service name or Docker Compose configuration. Changing the
                compose content requires a restart to take effect.
              </DialogDescription>
            </DialogHeader>
            {updateMutation.error && (
              <div className="p-3 rounded-md bg-destructive/10 text-destructive text-sm">
                {updateMutation.error instanceof Error
                  ? updateMutation.error.message
                  : "Failed to update service"}
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
            <AlertDialogTitle>Delete Service</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete "{service.name}"? This will stop all
              containers and remove all data. This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDelete}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending ? "Deleting..." : "Delete Service"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
