import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Plus, Trash2, Network } from "lucide-react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
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
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { destinationsApi } from "@/lib/api/destinations";
import type { Destination } from "@/types/destinations";

export function meta() {
  return [
    { title: "Destinations - Rivetr" },
    { name: "description", content: "Manage Docker network destinations" },
  ];
}

export default function SettingsDestinationsPage() {
  const queryClient = useQueryClient();
  const [showCreate, setShowCreate] = useState(false);
  const [showDelete, setShowDelete] = useState<Destination | null>(null);
  const [newName, setNewName] = useState("");
  const [newNetworkName, setNewNetworkName] = useState("");

  const { data: destinations, isLoading } = useQuery({
    queryKey: ["destinations"],
    queryFn: () => destinationsApi.list(),
  });

  const createMutation = useMutation({
    mutationFn: () =>
      destinationsApi.create({ name: newName, network_name: newNetworkName }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["destinations"] });
      setShowCreate(false);
      setNewName("");
      setNewNetworkName("");
      toast.success("Destination created");
    },
    onError: (err: Error) => {
      toast.error(err.message || "Failed to create destination");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => destinationsApi.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["destinations"] });
      setShowDelete(null);
      toast.success("Destination deleted");
    },
    onError: (err: Error) => {
      toast.error(err.message || "Failed to delete destination");
    },
  });

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold">Destinations</h1>
        <p className="text-muted-foreground mt-1">
          Named Docker networks that apps can be assigned to. Each destination
          creates (or reuses) a Docker bridge network.
        </p>
      </div>

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Docker Destinations</CardTitle>
              <CardDescription>
                Assign apps to custom Docker networks for isolated or shared
                networking.
              </CardDescription>
            </div>
            <Button onClick={() => setShowCreate(true)}>
              <Plus className="h-4 w-4 mr-2" />
              Create Destination
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <p className="text-muted-foreground text-sm">Loading...</p>
          ) : !destinations || destinations.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <Network className="h-8 w-8 mx-auto mb-2 opacity-50" />
              <p className="text-sm">No destinations yet.</p>
              <p className="text-xs mt-1">
                Create a destination to assign apps to custom Docker networks.
              </p>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Docker Network</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead className="w-[80px]"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {destinations.map((dest) => (
                  <TableRow key={dest.id}>
                    <TableCell className="font-medium">{dest.name}</TableCell>
                    <TableCell>
                      <code className="text-xs bg-muted px-1.5 py-0.5 rounded">
                        {dest.network_name}
                      </code>
                    </TableCell>
                    <TableCell className="text-muted-foreground text-sm">
                      {new Date(dest.created_at).toLocaleDateString()}
                    </TableCell>
                    <TableCell>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setShowDelete(dest)}
                      >
                        <Trash2 className="h-4 w-4 text-destructive" />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Create Dialog */}
      <Dialog open={showCreate} onOpenChange={setShowCreate}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create Destination</DialogTitle>
            <DialogDescription>
              A destination maps to a Docker bridge network. The network will be
              created if it does not already exist.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="space-y-2">
              <Label htmlFor="dest-name">Name</Label>
              <Input
                id="dest-name"
                placeholder="e.g. Production Network"
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="dest-network">Docker Network Name</Label>
              <Input
                id="dest-network"
                placeholder="e.g. prod-net"
                value={newNetworkName}
                onChange={(e) => setNewNetworkName(e.target.value)}
              />
              <p className="text-xs text-muted-foreground">
                The Docker network name (lowercase, no spaces).
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowCreate(false)}>
              Cancel
            </Button>
            <Button
              onClick={() => createMutation.mutate()}
              disabled={
                !newName.trim() ||
                !newNetworkName.trim() ||
                createMutation.isPending
              }
            >
              {createMutation.isPending ? "Creating..." : "Create"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <AlertDialog
        open={!!showDelete}
        onOpenChange={(open) => !open && setShowDelete(null)}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Destination</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete{" "}
              <strong>{showDelete?.name}</strong>? Apps using this destination
              will be reset to the default network. The Docker network{" "}
              <code className="text-xs">{showDelete?.network_name}</code> will
              also be removed.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => showDelete && deleteMutation.mutate(showDelete.id)}
              className="bg-destructive hover:bg-destructive/90"
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
