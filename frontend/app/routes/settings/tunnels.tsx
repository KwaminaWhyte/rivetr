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
import { tunnelsApi } from "@/lib/api/tunnels";
import type { CloudflareTunnel } from "@/lib/api/tunnels";
import { Plus, Trash2, Play, Square, Loader2, Globe } from "lucide-react";

function statusBadge(status: CloudflareTunnel["status"]) {
  switch (status) {
    case "running":
      return <Badge variant="default" className="bg-green-500">Running</Badge>;
    case "starting":
      return <Badge variant="secondary">Starting…</Badge>;
    case "error":
      return <Badge variant="destructive">Error</Badge>;
    default:
      return <Badge variant="outline">Stopped</Badge>;
  }
}

export default function TunnelsPage() {
  const qc = useQueryClient();

  const { data: tunnels = [], isLoading } = useQuery({
    queryKey: ["tunnels"],
    queryFn: () => tunnelsApi.list(),
    refetchInterval: 5000,
  });

  // Add-tunnel dialog
  const [addOpen, setAddOpen] = useState(false);
  const [name, setName] = useState("");
  const [token, setToken] = useState("");

  const createMutation = useMutation({
    mutationFn: () => tunnelsApi.create({ name, tunnel_token: token }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["tunnels"] });
      toast.success("Tunnel created and starting…");
      setAddOpen(false);
      setName("");
      setToken("");
    },
    onError: () => toast.error("Failed to create tunnel"),
  });

  // Delete confirmation
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const deleteMutation = useMutation({
    mutationFn: (id: string) => tunnelsApi.delete(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["tunnels"] });
      toast.success("Tunnel deleted");
      setDeleteId(null);
    },
    onError: () => toast.error("Failed to delete tunnel"),
  });

  const startMutation = useMutation({
    mutationFn: (id: string) => tunnelsApi.start(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["tunnels"] });
      toast.success("Tunnel starting…");
    },
    onError: () => toast.error("Failed to start tunnel"),
  });

  const stopMutation = useMutation({
    mutationFn: (id: string) => tunnelsApi.stop(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["tunnels"] });
      toast.success("Tunnel stopped");
    },
    onError: () => toast.error("Failed to stop tunnel"),
  });

  return (
    <div className="container mx-auto py-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Cloudflare Tunnels</h1>
          <p className="text-muted-foreground text-sm mt-1">
            Expose your apps through Cloudflare's network without opening
            firewall ports.
          </p>
        </div>
        <Button onClick={() => setAddOpen(true)}>
          <Plus className="mr-2 h-4 w-4" />
          Add Tunnel
        </Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Tunnels</CardTitle>
          <CardDescription>
            Each tunnel runs a{" "}
            <code className="text-xs">cloudflare/cloudflared</code> container
            on the Rivetr network.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center gap-2 text-muted-foreground py-4">
              <Loader2 className="h-4 w-4 animate-spin" />
              Loading tunnels…
            </div>
          ) : tunnels.length === 0 ? (
            <div className="py-8 text-center text-muted-foreground">
              <Globe className="mx-auto h-10 w-10 mb-3 opacity-40" />
              <p>No tunnels yet. Click <strong>Add Tunnel</strong> to get started.</p>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Routes</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {tunnels.map((tunnel) => (
                  <TableRow key={tunnel.id}>
                    <TableCell className="font-medium">{tunnel.name}</TableCell>
                    <TableCell>{statusBadge(tunnel.status)}</TableCell>
                    <TableCell className="text-muted-foreground text-sm">
                      {tunnel.routes.length === 0
                        ? "None"
                        : tunnel.routes
                            .map((r) => r.hostname)
                            .join(", ")}
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="flex items-center justify-end gap-2">
                        {tunnel.status === "running" ? (
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => stopMutation.mutate(tunnel.id)}
                            disabled={stopMutation.isPending}
                          >
                            <Square className="h-3 w-3 mr-1" />
                            Stop
                          </Button>
                        ) : (
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => startMutation.mutate(tunnel.id)}
                            disabled={
                              startMutation.isPending ||
                              tunnel.status === "starting"
                            }
                          >
                            <Play className="h-3 w-3 mr-1" />
                            Start
                          </Button>
                        )}
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => setDeleteId(tunnel.id)}
                        >
                          <Trash2 className="h-4 w-4 text-destructive" />
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

      {/* Add Tunnel Dialog */}
      <Dialog open={addOpen} onOpenChange={setAddOpen}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>Add Cloudflare Tunnel</DialogTitle>
            <DialogDescription>
              Create a tunnel in the Cloudflare dashboard, copy the token, and
              paste it here. Rivetr will start a{" "}
              <code className="text-xs">cloudflared</code> container using that
              token.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="space-y-1.5">
              <Label htmlFor="tunnel-name">Name</Label>
              <Input
                id="tunnel-name"
                placeholder="my-tunnel"
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="tunnel-token">Tunnel Token</Label>
              <Input
                id="tunnel-token"
                type="password"
                placeholder="eyJhI…"
                value={token}
                onChange={(e) => setToken(e.target.value)}
              />
              <p className="text-xs text-muted-foreground">
                Found in the Cloudflare Zero Trust dashboard under
                Networks → Tunnels → your tunnel → Configure → Install and run connector.
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setAddOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={() => createMutation.mutate()}
              disabled={!name || !token || createMutation.isPending}
            >
              {createMutation.isPending && (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              )}
              Create Tunnel
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <AlertDialog
        open={deleteId !== null}
        onOpenChange={(open) => !open && setDeleteId(null)}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete tunnel?</AlertDialogTitle>
            <AlertDialogDescription>
              This will stop the cloudflared container and permanently remove the
              tunnel configuration. This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              onClick={() => deleteId && deleteMutation.mutate(deleteId)}
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending && (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              )}
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
