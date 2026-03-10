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
import { serversApi } from "@/lib/api/servers";
import type { Server, CreateServerRequest } from "@/lib/api/servers";
import { Server as ServerIcon, Plus, Trash2, RefreshCw, Loader2, Cpu, MemoryStick, HardDrive } from "lucide-react";

export function meta() {
  return [
    { title: "Servers - Rivetr" },
    { name: "description", content: "Manage remote servers for multi-server deployments" },
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

function StatusDot({ status }: { status: Server["status"] }) {
  const colorClass =
    status === "online"
      ? "bg-green-500"
      : status === "offline"
      ? "bg-red-500"
      : "bg-yellow-400";
  return (
    <span className={`inline-block w-2.5 h-2.5 rounded-full ${colorClass}`} />
  );
}

function StatusBadge({ status }: { status: Server["status"] }) {
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

export default function ServersPage() {
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
  const [isSubmitting, setIsSubmitting] = useState(false);

  const { data: servers = [], isLoading } = useQuery<Server[]>({
    queryKey: ["servers"],
    queryFn: () => serversApi.list(),
  });

  const createMutation = useMutation({
    mutationFn: (data: CreateServerRequest) => serversApi.create(data),
    onSuccess: () => {
      toast.success("Server added successfully");
      queryClient.invalidateQueries({ queryKey: ["servers"] });
      resetForm();
      setAddDialogOpen(false);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to add server");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => serversApi.delete(id),
    onSuccess: () => {
      toast.success("Server removed");
      queryClient.invalidateQueries({ queryKey: ["servers"] });
      setDeleteId(null);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to remove server");
    },
  });

  const resetForm = () => {
    setFormName("");
    setFormHost("");
    setFormPort("22");
    setFormUsername("root");
    setFormSshKey("");
  };

  const handleAddServer = async () => {
    if (!formName.trim() || !formHost.trim()) {
      toast.error("Name and host are required");
      return;
    }
    setIsSubmitting(true);
    try {
      const port = parseInt(formPort, 10);
      await createMutation.mutateAsync({
        name: formName.trim(),
        host: formHost.trim(),
        port: isNaN(port) ? 22 : port,
        username: formUsername.trim() || "root",
        ssh_private_key: formSshKey.trim() || undefined,
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleCheckHealth = async (server: Server) => {
    setCheckingId(server.id);
    try {
      await serversApi.check(server.id);
      toast.success(`Health check complete for ${server.name}`);
      queryClient.invalidateQueries({ queryKey: ["servers"] });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Health check failed");
    } finally {
      setCheckingId(null);
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Remote Servers</h1>
        <p className="text-muted-foreground">
          Register and manage remote servers for multi-server deployments.
        </p>
      </div>

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <ServerIcon className="h-5 w-5" />
                Servers
              </CardTitle>
              <CardDescription>
                Add remote servers accessible via SSH to deploy applications across your infrastructure.
              </CardDescription>
            </div>
            <Button onClick={() => setAddDialogOpen(true)} className="gap-2">
              <Plus className="h-4 w-4" />
              Add Server
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
              <ServerIcon className="h-12 w-12 mx-auto text-muted-foreground/50" />
              <div>
                <p className="text-lg font-medium">No Servers Registered</p>
                <p className="text-sm text-muted-foreground">
                  Add a remote server to start deploying across multiple machines.
                </p>
              </div>
              <Button onClick={() => setAddDialogOpen(true)} className="gap-2">
                <Plus className="h-4 w-4" />
                Add Server
              </Button>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Host</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Resources</TableHead>
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
                    <TableCell>
                      {server.status === "online" ? (
                        <div className="flex items-center gap-3 text-xs text-muted-foreground">
                          {server.cpu_usage != null && (
                            <span className="flex items-center gap-1">
                              <Cpu className="h-3 w-3" />
                              {server.cpu_usage.toFixed(1)}%
                            </span>
                          )}
                          {server.memory_usage != null && (
                            <span className="flex items-center gap-1">
                              <MemoryStick className="h-3 w-3" />
                              {server.memory_usage.toFixed(1)}%
                            </span>
                          )}
                          {server.disk_usage != null && (
                            <span className="flex items-center gap-1">
                              <HardDrive className="h-3 w-3" />
                              {server.disk_usage.toFixed(1)}%
                            </span>
                          )}
                          {server.memory_total != null && (
                            <span className="text-muted-foreground/70">
                              {formatBytes(server.memory_total)} RAM
                            </span>
                          )}
                        </div>
                      ) : (
                        <span className="text-xs text-muted-foreground">—</span>
                      )}
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

      {/* Add Server Dialog */}
      <Dialog open={addDialogOpen} onOpenChange={(open) => { setAddDialogOpen(open); if (!open) resetForm(); }}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <ServerIcon className="h-5 w-5" />
              Add Remote Server
            </DialogTitle>
            <DialogDescription>
              Register a remote server accessible via SSH.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="grid grid-cols-2 gap-4">
              <div className="col-span-2 space-y-2">
                <Label htmlFor="server-name">Name</Label>
                <Input
                  id="server-name"
                  placeholder="production-us-east"
                  value={formName}
                  onChange={(e) => setFormName(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="server-host">Host / IP</Label>
                <Input
                  id="server-host"
                  placeholder="192.168.1.100"
                  value={formHost}
                  onChange={(e) => setFormHost(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="server-port">SSH Port</Label>
                <Input
                  id="server-port"
                  type="number"
                  placeholder="22"
                  value={formPort}
                  onChange={(e) => setFormPort(e.target.value)}
                />
              </div>
              <div className="col-span-2 space-y-2">
                <Label htmlFor="server-username">SSH Username</Label>
                <Input
                  id="server-username"
                  placeholder="root"
                  value={formUsername}
                  onChange={(e) => setFormUsername(e.target.value)}
                />
              </div>
              <div className="col-span-2 space-y-2">
                <Label htmlFor="server-ssh-key">
                  SSH Private Key{" "}
                  <span className="text-muted-foreground font-normal">(optional)</span>
                </Label>
                <Textarea
                  id="server-ssh-key"
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
              onClick={() => { setAddDialogOpen(false); resetForm(); }}
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
              Add Server
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <AlertDialog open={!!deleteId} onOpenChange={() => setDeleteId(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Remove Server</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to remove this server? All app assignments for this
              server will also be removed. This action cannot be undone.
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
