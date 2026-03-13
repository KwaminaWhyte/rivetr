import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
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
import {
  Network,
  Plus,
  Trash2,
  RefreshCw,
  Loader2,
  ServerCrash,
  AlertCircle,
  LogOut,
  Server,
} from "lucide-react";
import { swarmApi } from "@/lib/api/swarm";
import type { SwarmNode, SwarmService } from "@/lib/api/swarm";

export function meta() {
  return [
    { title: "Docker Swarm - Rivetr" },
    { name: "description", content: "Manage Docker Swarm cluster" },
  ];
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatDate(dateString: string) {
  return new Date(dateString).toLocaleString();
}

function NodeStatusBadge({ status }: { status: SwarmNode["status"] }) {
  const variant =
    status === "ready"
      ? "default"
      : status === "down"
      ? "destructive"
      : "secondary";
  return (
    <Badge variant={variant} className="capitalize">
      {status}
    </Badge>
  );
}

function NodeAvailabilityBadge({
  availability,
}: {
  availability: SwarmNode["availability"];
}) {
  const variant =
    availability === "active"
      ? "default"
      : availability === "drain"
      ? "destructive"
      : "secondary";
  return (
    <Badge variant={variant} className="capitalize">
      {availability}
    </Badge>
  );
}

function ServiceStatusBadge({ status }: { status: SwarmService["status"] }) {
  const variant =
    status === "running"
      ? "default"
      : status === "failed"
      ? "destructive"
      : "secondary";
  return (
    <Badge variant={variant} className="capitalize">
      {status}
    </Badge>
  );
}

// ---------------------------------------------------------------------------
// Main page
// ---------------------------------------------------------------------------

export default function SwarmPage() {
  const queryClient = useQueryClient();

  // Dialogs
  const [leaveDialogOpen, setLeaveDialogOpen] = useState(false);
  const [createServiceDialogOpen, setCreateServiceDialogOpen] = useState(false);
  const [deleteServiceId, setDeleteServiceId] = useState<string | null>(null);
  const [scaleServiceId, setScaleServiceId] = useState<string | null>(null);
  const [scaleReplicas, setScaleReplicas] = useState("1");

  // Create service form
  const [formServiceName, setFormServiceName] = useState("");
  const [formImage, setFormImage] = useState("");
  const [formReplicas, setFormReplicas] = useState("1");
  const [isCreatingService, setIsCreatingService] = useState(false);

  // Init token result
  const [initResult, setInitResult] = useState<{
    manager_token: string;
    worker_token: string;
    node_id: string;
  } | null>(null);

  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------

  const {
    data: status,
    isLoading: statusLoading,
    isError: statusError,
  } = useQuery({
    queryKey: ["swarm-status"],
    queryFn: () => swarmApi.getStatus(),
    retry: false,
  });

  const isSwarmActive = status?.local_node_state === "active";

  const { data: nodes = [], isLoading: nodesLoading, isError: nodesError } = useQuery<SwarmNode[]>({
    queryKey: ["swarm-nodes"],
    queryFn: () => swarmApi.listNodes(),
    enabled: isSwarmActive,
    retry: false,
  });

  const { data: services = [], isLoading: servicesLoading, isError: servicesError } = useQuery<
    SwarmService[]
  >({
    queryKey: ["swarm-services"],
    queryFn: () => swarmApi.listServices(),
    enabled: isSwarmActive,
    retry: false,
  });

  // ---------------------------------------------------------------------------
  // Mutations
  // ---------------------------------------------------------------------------

  const initMutation = useMutation({
    mutationFn: () => swarmApi.init(),
    onSuccess: (data) => {
      toast.success("Swarm initialized successfully");
      setInitResult(data);
      queryClient.invalidateQueries({ queryKey: ["swarm-status"] });
      queryClient.invalidateQueries({ queryKey: ["swarm-nodes"] });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to initialize swarm");
    },
  });

  const leaveMutation = useMutation({
    mutationFn: () => swarmApi.leave(),
    onSuccess: () => {
      toast.success("Left the swarm");
      setLeaveDialogOpen(false);
      queryClient.invalidateQueries({ queryKey: ["swarm-status"] });
      queryClient.invalidateQueries({ queryKey: ["swarm-nodes"] });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to leave swarm");
    },
  });

  const syncNodesMutation = useMutation({
    mutationFn: () => swarmApi.syncNodes(),
    onSuccess: () => {
      toast.success("Nodes synced");
      queryClient.invalidateQueries({ queryKey: ["swarm-nodes"] });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to sync nodes");
    },
  });

  const updateAvailabilityMutation = useMutation({
    mutationFn: ({
      id,
      availability,
    }: {
      id: string;
      availability: "active" | "pause" | "drain";
    }) => swarmApi.updateNodeAvailability(id, availability),
    onSuccess: () => {
      toast.success("Node availability updated");
      queryClient.invalidateQueries({ queryKey: ["swarm-nodes"] });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to update node availability");
    },
  });

  const deleteServiceMutation = useMutation({
    mutationFn: (id: string) => swarmApi.deleteService(id),
    onSuccess: () => {
      toast.success("Service removed");
      setDeleteServiceId(null);
      queryClient.invalidateQueries({ queryKey: ["swarm-services"] });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to remove service");
    },
  });

  const scaleServiceMutation = useMutation({
    mutationFn: ({ id, replicas }: { id: string; replicas: number }) =>
      swarmApi.scaleService(id, replicas),
    onSuccess: () => {
      toast.success("Service scaled");
      setScaleServiceId(null);
      queryClient.invalidateQueries({ queryKey: ["swarm-services"] });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to scale service");
    },
  });

  // ---------------------------------------------------------------------------
  // Handlers
  // ---------------------------------------------------------------------------

  const handleCreateService = async () => {
    if (!formServiceName.trim() || !formImage.trim()) {
      toast.error("Service name and image are required");
      return;
    }
    setIsCreatingService(true);
    try {
      const replicas = parseInt(formReplicas, 10);
      await swarmApi.createService({
        service_name: formServiceName.trim(),
        image: formImage.trim(),
        replicas: isNaN(replicas) ? 1 : replicas,
      });
      toast.success("Service created");
      queryClient.invalidateQueries({ queryKey: ["swarm-services"] });
      setCreateServiceDialogOpen(false);
      setFormServiceName("");
      setFormImage("");
      setFormReplicas("1");
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to create service"
      );
    } finally {
      setIsCreatingService(false);
    }
  };

  const handleScale = async () => {
    if (!scaleServiceId) return;
    const replicas = parseInt(scaleReplicas, 10);
    if (isNaN(replicas) || replicas < 0) {
      toast.error("Enter a valid replica count");
      return;
    }
    scaleServiceMutation.mutate({ id: scaleServiceId, replicas });
  };

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Docker Swarm</h1>
        <p className="text-muted-foreground">
          Manage your Docker Swarm cluster — nodes and services.
        </p>
      </div>

      {/* Status Card */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Network className="h-5 w-5" />
                Swarm Status
              </CardTitle>
              <CardDescription>
                Current Docker Swarm state for this node.
              </CardDescription>
            </div>
            {isSwarmActive && (
              <Button
                variant="destructive"
                size="sm"
                onClick={() => setLeaveDialogOpen(true)}
                className="gap-2"
              >
                <LogOut className="h-4 w-4" />
                Leave Swarm
              </Button>
            )}
          </div>
        </CardHeader>
        <CardContent>
          {statusLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : statusError ? (
            <div className="text-center py-8 space-y-4">
              <AlertCircle className="h-12 w-12 mx-auto text-destructive/50" />
              <div>
                <p className="text-lg font-medium">Could Not Reach Docker</p>
                <p className="text-sm text-muted-foreground">
                  Failed to get swarm status. Make sure Docker is running and the
                  Rivetr process has access to the Docker socket.
                </p>
              </div>
            </div>
          ) : !isSwarmActive ? (
            <div className="text-center py-8 space-y-4">
              <ServerCrash className="h-12 w-12 mx-auto text-muted-foreground/50" />
              <div>
                <p className="text-lg font-medium">Swarm Not Initialized</p>
                <p className="text-sm text-muted-foreground">
                  This node is not part of a Docker Swarm cluster.
                </p>
              </div>
              <Button
                onClick={() => initMutation.mutate()}
                disabled={initMutation.isPending}
                className="gap-2"
              >
                {initMutation.isPending ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <Network className="h-4 w-4" />
                )}
                Initialize Swarm
              </Button>
            </div>
          ) : (
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
              <div>
                <div className="text-sm text-muted-foreground">State</div>
                <div className="font-medium capitalize">
                  {status?.local_node_state ?? "—"}
                </div>
              </div>
              <div>
                <div className="text-sm text-muted-foreground">Role</div>
                <div className="font-medium">
                  {status?.is_manager ? "Manager" : "Worker"}
                </div>
              </div>
              <div>
                <div className="text-sm text-muted-foreground">Nodes</div>
                <div className="font-medium">{status?.node_count ?? 0}</div>
              </div>
              <div>
                <div className="text-sm text-muted-foreground">
                  Managers / Workers
                </div>
                <div className="font-medium">
                  {status?.managers ?? 0} / {status?.workers ?? 0}
                </div>
              </div>
              {status?.node_id && (
                <div className="md:col-span-2">
                  <div className="text-sm text-muted-foreground">Node ID</div>
                  <div className="font-mono text-sm">{status.node_id}</div>
                </div>
              )}
            </div>
          )}

          {/* Show join tokens after init */}
          {initResult && (
            <div className="mt-4 p-4 rounded-md bg-muted space-y-3">
              <p className="text-sm font-medium">
                Swarm initialized. Save these join tokens:
              </p>
              <div className="space-y-1">
                <p className="text-xs text-muted-foreground">Manager Token</p>
                <code className="block text-xs bg-background rounded p-2 break-all">
                  {initResult.manager_token}
                </code>
              </div>
              <div className="space-y-1">
                <p className="text-xs text-muted-foreground">Worker Token</p>
                <code className="block text-xs bg-background rounded p-2 break-all">
                  {initResult.worker_token}
                </code>
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Nodes Table */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Server className="h-5 w-5" />
                Nodes
              </CardTitle>
              <CardDescription>
                Swarm nodes synced from Docker.
              </CardDescription>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => syncNodesMutation.mutate()}
              disabled={syncNodesMutation.isPending}
              className="gap-2"
            >
              {syncNodesMutation.isPending ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <RefreshCw className="h-4 w-4" />
              )}
              Sync Nodes
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {nodesLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : nodesError ? (
            <div className="flex items-center gap-2 py-8 justify-center text-sm text-destructive">
              <AlertCircle className="h-4 w-4" />
              Failed to load nodes. Try syncing again.
            </div>
          ) : nodes.length === 0 ? (
            <div className="text-center py-8">
              <p className="text-sm text-muted-foreground">
                No nodes found. Click Sync Nodes to fetch from Docker.
              </p>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Hostname</TableHead>
                  <TableHead>Role</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Availability</TableHead>
                  <TableHead>Last Seen</TableHead>
                  <TableHead className="w-[160px]"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {nodes.map((node) => (
                  <TableRow key={node.id}>
                    <TableCell className="font-medium">
                      {node.hostname}
                      {node.ip_address && (
                        <span className="ml-2 text-xs text-muted-foreground font-mono">
                          {node.ip_address}
                        </span>
                      )}
                    </TableCell>
                    <TableCell>
                      <Badge
                        variant={
                          node.role === "manager" ? "default" : "secondary"
                        }
                        className="capitalize"
                      >
                        {node.role}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <NodeStatusBadge status={node.status} />
                    </TableCell>
                    <TableCell>
                      <NodeAvailabilityBadge availability={node.availability} />
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {node.last_seen_at ? formatDate(node.last_seen_at) : "—"}
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-1">
                        {node.availability !== "drain" ? (
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() =>
                              updateAvailabilityMutation.mutate({
                                id: node.id,
                                availability: "drain",
                              })
                            }
                            disabled={updateAvailabilityMutation.isPending}
                          >
                            Drain
                          </Button>
                        ) : (
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() =>
                              updateAvailabilityMutation.mutate({
                                id: node.id,
                                availability: "active",
                              })
                            }
                            disabled={updateAvailabilityMutation.isPending}
                          >
                            Activate
                          </Button>
                        )}
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Services Table */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Network className="h-5 w-5" />
                Services
              </CardTitle>
              <CardDescription>
                Docker Swarm services running across the cluster.
              </CardDescription>
            </div>
            <Button
              size="sm"
              onClick={() => setCreateServiceDialogOpen(true)}
              className="gap-2"
            >
              <Plus className="h-4 w-4" />
              Add Service
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {servicesLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : servicesError ? (
            <div className="flex items-center gap-2 py-8 justify-center text-sm text-destructive">
              <AlertCircle className="h-4 w-4" />
              Failed to load services.
            </div>
          ) : services.length === 0 ? (
            <div className="text-center py-8 space-y-4">
              <Network className="h-10 w-10 mx-auto text-muted-foreground/50" />
              <p className="text-sm text-muted-foreground">
                No swarm services found.
              </p>
              <Button
                variant="outline"
                size="sm"
                onClick={() => setCreateServiceDialogOpen(true)}
                className="gap-2"
              >
                <Plus className="h-4 w-4" />
                Add Service
              </Button>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Image</TableHead>
                  <TableHead>Mode</TableHead>
                  <TableHead>Replicas</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead className="w-[180px]"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {services.map((service) => (
                  <TableRow key={service.id}>
                    <TableCell className="font-medium">
                      {service.service_name}
                    </TableCell>
                    <TableCell className="font-mono text-xs text-muted-foreground">
                      {service.image}
                    </TableCell>
                    <TableCell>
                      <Badge variant="secondary" className="capitalize">
                        {service.mode}
                      </Badge>
                    </TableCell>
                    <TableCell>{service.replicas}</TableCell>
                    <TableCell>
                      <ServiceStatusBadge status={service.status} />
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-1">
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => {
                            setScaleServiceId(service.id);
                            setScaleReplicas(String(service.replicas));
                          }}
                        >
                          Scale
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => setDeleteServiceId(service.id)}
                          className="text-destructive hover:text-destructive hover:bg-destructive/10"
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

      {/* Leave Swarm Dialog */}
      <AlertDialog
        open={leaveDialogOpen}
        onOpenChange={(open) => {
          if (!leaveMutation.isPending) setLeaveDialogOpen(open);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Leave Docker Swarm</AlertDialogTitle>
            <AlertDialogDescription>
              This will force-leave the current swarm. All swarm services on
              this node will be disrupted. This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={leaveMutation.isPending}>
              Cancel
            </AlertDialogCancel>
            <AlertDialogAction
              onClick={() => leaveMutation.mutate()}
              disabled={leaveMutation.isPending}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {leaveMutation.isPending ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                "Leave Swarm"
              )}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Create Service Dialog */}
      <Dialog
        open={createServiceDialogOpen}
        onOpenChange={(open) => {
          setCreateServiceDialogOpen(open);
          if (!open) {
            setFormServiceName("");
            setFormImage("");
            setFormReplicas("1");
          }
        }}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Network className="h-5 w-5" />
              Create Swarm Service
            </DialogTitle>
            <DialogDescription>
              Deploy a new service across the Docker Swarm cluster.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="space-y-2">
              <Label htmlFor="svc-name">Service Name</Label>
              <Input
                id="svc-name"
                placeholder="my-service"
                value={formServiceName}
                onChange={(e) => setFormServiceName(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="svc-image">Image</Label>
              <Input
                id="svc-image"
                placeholder="nginx:latest"
                value={formImage}
                onChange={(e) => setFormImage(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="svc-replicas">Replicas</Label>
              <Input
                id="svc-replicas"
                type="number"
                min={1}
                placeholder="1"
                value={formReplicas}
                onChange={(e) => setFormReplicas(e.target.value)}
              />
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setCreateServiceDialogOpen(false)}
            >
              Cancel
            </Button>
            <Button
              onClick={handleCreateService}
              disabled={
                isCreatingService ||
                !formServiceName.trim() ||
                !formImage.trim()
              }
              className="gap-2"
            >
              {isCreatingService ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Plus className="h-4 w-4" />
              )}
              Create Service
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Scale Service Dialog */}
      <Dialog
        open={!!scaleServiceId}
        onOpenChange={(open) => {
          if (!open) setScaleServiceId(null);
        }}
      >
        <DialogContent className="sm:max-w-sm">
          <DialogHeader>
            <DialogTitle>Scale Service</DialogTitle>
            <DialogDescription>
              Set the desired number of replicas for this service.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-2 py-2">
            <Label htmlFor="scale-replicas">Replicas</Label>
            <Input
              id="scale-replicas"
              type="number"
              min={0}
              value={scaleReplicas}
              onChange={(e) => setScaleReplicas(e.target.value)}
            />
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setScaleServiceId(null)}
            >
              Cancel
            </Button>
            <Button
              onClick={handleScale}
              disabled={scaleServiceMutation.isPending}
              className="gap-2"
            >
              {scaleServiceMutation.isPending ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : null}
              Scale
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Service Dialog */}
      <AlertDialog
        open={!!deleteServiceId}
        onOpenChange={(open) => {
          if (!open && !deleteServiceMutation.isPending) setDeleteServiceId(null);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Remove Service</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to remove this swarm service? This will
              stop all running tasks and remove the service from the cluster.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={deleteServiceMutation.isPending}>
              Cancel
            </AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                if (deleteServiceId) {
                  deleteServiceMutation.mutate(deleteServiceId);
                }
              }}
              disabled={deleteServiceMutation.isPending}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {deleteServiceMutation.isPending ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                "Remove"
              )}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
