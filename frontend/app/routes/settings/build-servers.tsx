import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
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
import { buildServersApi } from "@/lib/api/build-servers";
import type { BuildServer, CreateBuildServerRequest } from "@/lib/api/build-servers";
import {
  Hammer,
  Plus,
  Trash2,
  RefreshCw,
  Loader2,
} from "lucide-react";

export function meta() {
  return [
    { title: "Build Servers - Rivetr" },
    { name: "description", content: "Manage dedicated remote build servers" },
  ];
}

function formatDate(dateString: string) {
  return new Date(dateString).toLocaleString();
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  }
  if (bytes >= 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
  return `${(bytes / 1024).toFixed(1)} KB`;
}

function StatusDot({ status }: { status: BuildServer["status"] }) {
  const colorClass =
    status === "online"
      ? "bg-green-500"
      : status === "offline"
      ? "bg-red-500"
      : "bg-yellow-400";
  return <span className={`inline-block w-2.5 h-2.5 rounded-full ${colorClass}`} />;
}

function StatusBadge({ status }: { status: BuildServer["status"] }) {
  const variant =
    status === "online"
      ? "default"
      : status === "offline"
      ? "destructive"
      : "secondary";
  return (
    <Badge variant={variant} className="gap-1.5 capitalize">
      <StatusDot status={status} />
      {status}
    </Badge>
  );
}

export default function BuildServersPage() {
  const queryClient = useQueryClient();
  const [addDialogOpen, setAddDialogOpen] = useState(false);
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const [checkingId, setCheckingId] = useState<string | null>(null);

  // Form state
  const [formName, setFormName] = useState("");
  const [formHost, setFormHost] = useState("");
  const [formPort, setFormPort] = useState("22");
  const [formUsername, setFormUsername] = useState("root");
  const [formSshKey, setFormSshKey] = useState("");
  const [formConcurrentBuilds, setFormConcurrentBuilds] = useState("2");
  const [isSubmitting, setIsSubmitting] = useState(false);

  const { data: servers = [], isLoading } = useQuery<BuildServer[]>({
    queryKey: ["build-servers"],
    queryFn: () => buildServersApi.list(),
  });

  const createMutation = useMutation({
    mutationFn: (data: CreateBuildServerRequest) => buildServersApi.create(data),
    onSuccess: () => {
      toast.success("Build server added successfully");
      queryClient.invalidateQueries({ queryKey: ["build-servers"] });
      resetForm();
      setAddDialogOpen(false);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to add build server");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => buildServersApi.delete(id),
    onSuccess: () => {
      toast.success("Build server removed");
      queryClient.invalidateQueries({ queryKey: ["build-servers"] });
      setDeleteId(null);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to remove build server");
    },
  });

  const resetForm = () => {
    setFormName("");
    setFormHost("");
    setFormPort("22");
    setFormUsername("root");
    setFormSshKey("");
    setFormConcurrentBuilds("2");
  };

  const handleAddServer = async () => {
    if (!formName.trim() || !formHost.trim()) {
      toast.error("Name and host are required");
      return;
    }
    setIsSubmitting(true);
    try {
      const port = parseInt(formPort, 10);
      const concurrentBuilds = parseInt(formConcurrentBuilds, 10);
      await createMutation.mutateAsync({
        name: formName.trim(),
        host: formHost.trim(),
        port: isNaN(port) ? 22 : port,
        username: formUsername.trim() || "root",
        ssh_private_key: formSshKey.trim() || undefined,
        concurrent_builds: isNaN(concurrentBuilds) ? 2 : concurrentBuilds,
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleCheckHealth = async (server: BuildServer) => {
    setCheckingId(server.id);
    try {
      await buildServersApi.check(server.id);
      toast.success(`Health check complete for ${server.name}`);
      queryClient.invalidateQueries({ queryKey: ["build-servers"] });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Health check failed");
    } finally {
      setCheckingId(null);
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Build Servers</h1>
        <p className="text-muted-foreground">
          Register dedicated remote servers for building applications.
        </p>
      </div>

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Hammer className="h-5 w-5" />
                Build Servers
              </CardTitle>
              <CardDescription>
                Dedicated servers that handle builds, keeping your deployment targets free for serving traffic.
              </CardDescription>
            </div>
            <Button onClick={() => setAddDialogOpen(true)} className="gap-2">
              <Plus className="h-4 w-4" />
              Add Build Server
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : servers.length === 0 ? (
            <div className="text-center py-12 space-y-4">
              <Hammer className="h-12 w-12 mx-auto text-muted-foreground/50" />
              <div>
                <p className="text-lg font-medium">No Build Servers Registered</p>
                <p className="text-sm text-muted-foreground">
                  Add a dedicated build server to offload build jobs from your deployment targets.
                </p>
              </div>
              <Button onClick={() => setAddDialogOpen(true)} className="gap-2">
                <Plus className="h-4 w-4" />
                Add Build Server
              </Button>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Host</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Builds</TableHead>
                  <TableHead>Docker</TableHead>
                  <TableHead>Last Seen</TableHead>
                  <TableHead className="w-[120px]"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {servers.map((server) => (
                  <TableRow key={server.id}>
                    <TableCell className="font-medium">{server.name}</TableCell>
                    <TableCell className="text-muted-foreground font-mono text-sm">
                      {server.host}:{server.port}
                    </TableCell>
                    <TableCell>
                      <StatusBadge status={server.status} />
                    </TableCell>
                    <TableCell className="text-sm">
                      <span className="font-medium">{server.active_builds}</span>
                      <span className="text-muted-foreground"> / {server.concurrent_builds}</span>
                      <span className="text-muted-foreground text-xs ml-1">concurrent</span>
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {server.docker_version
                        ? `v${server.docker_version}`
                        : server.status === "online"
                        ? "—"
                        : "—"}
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {server.last_seen_at ? formatDate(server.last_seen_at) : "Never"}
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-1">
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => handleCheckHealth(server)}
                          disabled={checkingId === server.id}
                          className="gap-1"
                        >
                          {checkingId === server.id ? (
                            <Loader2 className="h-3 w-3 animate-spin" />
                          ) : (
                            <RefreshCw className="h-3 w-3" />
                          )}
                          Check
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => setDeleteId(server.id)}
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

      {/* Add Build Server Dialog */}
      <Dialog
        open={addDialogOpen}
        onOpenChange={(open) => {
          setAddDialogOpen(open);
          if (!open) resetForm();
        }}
      >
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Hammer className="h-5 w-5" />
              Add Build Server
            </DialogTitle>
            <DialogDescription>
              Register a remote server dedicated to building application images.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="grid grid-cols-2 gap-4">
              <div className="col-span-2 space-y-2">
                <Label htmlFor="bs-name">Name</Label>
                <Input
                  id="bs-name"
                  placeholder="build-server-01"
                  value={formName}
                  onChange={(e) => setFormName(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="bs-host">Host / IP</Label>
                <Input
                  id="bs-host"
                  placeholder="192.168.1.50"
                  value={formHost}
                  onChange={(e) => setFormHost(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="bs-port">SSH Port</Label>
                <Input
                  id="bs-port"
                  type="number"
                  placeholder="22"
                  value={formPort}
                  onChange={(e) => setFormPort(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="bs-username">SSH Username</Label>
                <Input
                  id="bs-username"
                  placeholder="root"
                  value={formUsername}
                  onChange={(e) => setFormUsername(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="bs-concurrent">Max Concurrent Builds</Label>
                <Input
                  id="bs-concurrent"
                  type="number"
                  min={1}
                  max={32}
                  placeholder="2"
                  value={formConcurrentBuilds}
                  onChange={(e) => setFormConcurrentBuilds(e.target.value)}
                />
              </div>
              <div className="col-span-2 space-y-2">
                <Label htmlFor="bs-ssh-key">
                  SSH Private Key{" "}
                  <span className="text-muted-foreground font-normal">(optional)</span>
                </Label>
                <Textarea
                  id="bs-ssh-key"
                  placeholder="-----BEGIN RSA PRIVATE KEY-----&#10;...&#10;-----END RSA PRIVATE KEY-----"
                  rows={5}
                  value={formSshKey}
                  onChange={(e) => setFormSshKey(e.target.value)}
                  className="font-mono text-xs"
                />
                <p className="text-xs text-muted-foreground">
                  The private key will be encrypted with AES-256-GCM before storage.
                  Leave empty to use the system default SSH key.
                </p>
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setAddDialogOpen(false);
                resetForm();
              }}
            >
              Cancel
            </Button>
            <Button
              onClick={handleAddServer}
              disabled={isSubmitting || !formName.trim() || !formHost.trim()}
              className="gap-2"
            >
              {isSubmitting ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Plus className="h-4 w-4" />
              )}
              Add Build Server
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <AlertDialog open={!!deleteId} onOpenChange={() => setDeleteId(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Remove Build Server</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to remove this build server? Apps assigned to it will need to
              be reassigned. This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                if (deleteId) {
                  deleteMutation.mutate(deleteId);
                }
              }}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Remove
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
