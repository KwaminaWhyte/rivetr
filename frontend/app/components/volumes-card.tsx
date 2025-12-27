import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
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
import { Skeleton } from "@/components/ui/skeleton";
import { Badge } from "@/components/ui/badge";
import { Pencil, Trash2, Plus, Download, HardDrive, FolderOpen } from "lucide-react";
import { api } from "@/lib/api";
import type { Volume, CreateVolumeRequest, UpdateVolumeRequest } from "@/types/api";

interface VolumesCardProps {
  appId: string;
  token: string;
}

export function VolumesCard({ appId, token }: VolumesCardProps) {
  const queryClient = useQueryClient();
  const [showAddDialog, setShowAddDialog] = useState(false);
  const [showEditDialog, setShowEditDialog] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [selectedVolume, setSelectedVolume] = useState<Volume | null>(null);
  const [isBackingUp, setIsBackingUp] = useState<string | null>(null);

  // Form state for add/edit
  const [formName, setFormName] = useState("");
  const [formHostPath, setFormHostPath] = useState("");
  const [formContainerPath, setFormContainerPath] = useState("");
  const [formReadOnly, setFormReadOnly] = useState(false);

  // Fetch volumes
  const {
    data: volumes = [],
    isLoading,
    error,
  } = useQuery<Volume[]>({
    queryKey: ["volumes", appId],
    queryFn: () => api.getVolumes(appId, token),
  });

  // Create mutation
  const createMutation = useMutation({
    mutationFn: (data: CreateVolumeRequest) => api.createVolume(appId, data, token),
    onSuccess: () => {
      toast.success("Volume created");
      queryClient.invalidateQueries({ queryKey: ["volumes", appId] });
      resetForm();
      setShowAddDialog(false);
    },
    onError: (error: Error) => {
      if (error.message.includes("409") || error.message.includes("CONFLICT")) {
        toast.error("A volume with this name or container path already exists");
      } else if (error.message.includes("400")) {
        toast.error("Invalid input. Container path must be absolute (start with /).");
      } else {
        toast.error(`Failed to create: ${error.message}`);
      }
    },
  });

  // Update mutation
  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateVolumeRequest }) =>
      api.updateVolume(id, data, token),
    onSuccess: () => {
      toast.success("Volume updated");
      queryClient.invalidateQueries({ queryKey: ["volumes", appId] });
      resetForm();
      setShowEditDialog(false);
    },
    onError: (error: Error) => {
      if (error.message.includes("409") || error.message.includes("CONFLICT")) {
        toast.error("A volume with this name or container path already exists");
      } else {
        toast.error(`Failed to update: ${error.message}`);
      }
    },
  });

  // Delete mutation
  const deleteMutation = useMutation({
    mutationFn: (id: string) => api.deleteVolume(id, token),
    onSuccess: () => {
      toast.success("Volume deleted");
      queryClient.invalidateQueries({ queryKey: ["volumes", appId] });
      setShowDeleteDialog(false);
      setSelectedVolume(null);
    },
    onError: (error: Error) => {
      toast.error(`Failed to delete: ${error.message}`);
    },
  });

  const resetForm = () => {
    setFormName("");
    setFormHostPath("");
    setFormContainerPath("");
    setFormReadOnly(false);
    setSelectedVolume(null);
  };

  const handleAdd = () => {
    resetForm();
    setShowAddDialog(true);
  };

  const handleEdit = (volume: Volume) => {
    setSelectedVolume(volume);
    setFormName(volume.name);
    setFormHostPath(volume.host_path);
    setFormContainerPath(volume.container_path);
    setFormReadOnly(volume.read_only);
    setShowEditDialog(true);
  };

  const handleDelete = (volume: Volume) => {
    setSelectedVolume(volume);
    setShowDeleteDialog(true);
  };

  const handleBackup = async (volume: Volume) => {
    setIsBackingUp(volume.id);
    try {
      const response = await api.backupVolume(volume.id, token);
      if (!response.ok) {
        const error = await response.text();
        throw new Error(error || "Backup failed");
      }

      // Get the filename from Content-Disposition header or create one
      const contentDisposition = response.headers.get("Content-Disposition");
      let filename = `${volume.name}-backup.tar.gz`;
      if (contentDisposition) {
        const match = contentDisposition.match(/filename="(.+)"/);
        if (match) {
          filename = match[1];
        }
      }

      // Download the file
      const blob = await response.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);

      toast.success("Backup downloaded");
    } catch (error) {
      toast.error(`Backup failed: ${error instanceof Error ? error.message : "Unknown error"}`);
    } finally {
      setIsBackingUp(null);
    }
  };

  const handleSubmitAdd = () => {
    if (!formName.trim()) {
      toast.error("Name is required");
      return;
    }
    if (!formHostPath.trim()) {
      toast.error("Host path is required");
      return;
    }
    if (!formContainerPath.trim()) {
      toast.error("Container path is required");
      return;
    }
    if (!formContainerPath.startsWith("/")) {
      toast.error("Container path must be absolute (start with /)");
      return;
    }
    createMutation.mutate({
      name: formName.trim(),
      host_path: formHostPath.trim(),
      container_path: formContainerPath.trim(),
      read_only: formReadOnly,
    });
  };

  const handleSubmitEdit = () => {
    if (!selectedVolume) return;

    const updates: UpdateVolumeRequest = {};

    if (formName !== selectedVolume.name) {
      updates.name = formName.trim();
    }
    if (formHostPath !== selectedVolume.host_path) {
      updates.host_path = formHostPath.trim();
    }
    if (formContainerPath !== selectedVolume.container_path) {
      if (!formContainerPath.startsWith("/")) {
        toast.error("Container path must be absolute (start with /)");
        return;
      }
      updates.container_path = formContainerPath.trim();
    }
    if (formReadOnly !== selectedVolume.read_only) {
      updates.read_only = formReadOnly;
    }

    if (Object.keys(updates).length === 0) {
      toast.info("No changes to save");
      return;
    }

    updateMutation.mutate({ id: selectedVolume.id, data: updates });
  };

  if (isLoading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <HardDrive className="h-5 w-5" />
            Volumes
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            {[1, 2, 3].map((i) => (
              <Skeleton key={i} className="h-10 w-full" />
            ))}
          </div>
        </CardContent>
      </Card>
    );
  }

  if (error) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <HardDrive className="h-5 w-5" />
            Volumes
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-center py-4 text-red-500">
            Failed to load volumes
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between">
        <div>
          <CardTitle className="flex items-center gap-2">
            <HardDrive className="h-5 w-5" />
            Volumes
          </CardTitle>
          <CardDescription>
            Mount host directories into your container for persistent storage. Changes take effect on next deployment.
          </CardDescription>
        </div>
        <Button onClick={handleAdd} size="sm">
          <Plus className="h-4 w-4 mr-1" />
          Add Volume
        </Button>
      </CardHeader>
      <CardContent>
        {volumes.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground">
            No volumes configured.
            <br />
            Click "Add Volume" to mount a host directory.
          </div>
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Host Path</TableHead>
                <TableHead>Container Path</TableHead>
                <TableHead className="w-[100px]">Mode</TableHead>
                <TableHead className="w-[150px]">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {volumes.map((volume) => (
                <TableRow key={volume.id}>
                  <TableCell className="font-medium">{volume.name}</TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1 text-sm text-muted-foreground font-mono">
                      <FolderOpen className="h-3 w-3" />
                      <span className="truncate max-w-[200px]" title={volume.host_path}>
                        {volume.host_path}
                      </span>
                    </div>
                  </TableCell>
                  <TableCell>
                    <span className="font-mono text-sm truncate max-w-[200px]" title={volume.container_path}>
                      {volume.container_path}
                    </span>
                  </TableCell>
                  <TableCell>
                    <Badge variant={volume.read_only ? "secondary" : "outline"}>
                      {volume.read_only ? "Read Only" : "Read/Write"}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <div className="flex gap-1">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleBackup(volume)}
                        disabled={isBackingUp === volume.id}
                        className="h-7 w-7 p-0"
                        title="Backup volume"
                      >
                        <Download className="h-3 w-3" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleEdit(volume)}
                        className="h-7 w-7 p-0"
                        title="Edit volume"
                      >
                        <Pencil className="h-3 w-3" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleDelete(volume)}
                        className="h-7 w-7 p-0 text-red-500 hover:text-red-600"
                        title="Delete volume"
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

      {/* Add Dialog */}
      <Dialog open={showAddDialog} onOpenChange={setShowAddDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Volume</DialogTitle>
            <DialogDescription>
              Mount a host directory into your container. The host path will be created if it doesn't exist.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="add-name">Name</Label>
              <Input
                id="add-name"
                placeholder="data"
                value={formName}
                onChange={(e) => setFormName(e.target.value)}
              />
              <p className="text-xs text-muted-foreground">
                A descriptive name for this volume.
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="add-host-path">Host Path</Label>
              <Input
                id="add-host-path"
                placeholder="/var/rivetr/data/my-app"
                value={formHostPath}
                onChange={(e) => setFormHostPath(e.target.value)}
                className="font-mono"
              />
              <p className="text-xs text-muted-foreground">
                Path on the host machine to mount.
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="add-container-path">Container Path</Label>
              <Input
                id="add-container-path"
                placeholder="/app/data"
                value={formContainerPath}
                onChange={(e) => setFormContainerPath(e.target.value)}
                className="font-mono"
              />
              <p className="text-xs text-muted-foreground">
                Path inside the container. Must be absolute (start with /).
              </p>
            </div>
            <div className="flex items-center gap-2">
              <Switch
                id="add-read-only"
                checked={formReadOnly}
                onCheckedChange={setFormReadOnly}
              />
              <Label htmlFor="add-read-only" className="text-sm font-normal cursor-pointer">
                Mount as read-only
              </Label>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowAddDialog(false)}>
              Cancel
            </Button>
            <Button onClick={handleSubmitAdd} disabled={createMutation.isPending}>
              {createMutation.isPending ? "Creating..." : "Create"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit Dialog */}
      <Dialog open={showEditDialog} onOpenChange={setShowEditDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit Volume</DialogTitle>
            <DialogDescription>
              Update the volume configuration. Changes take effect on next deployment.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="edit-name">Name</Label>
              <Input
                id="edit-name"
                value={formName}
                onChange={(e) => setFormName(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-host-path">Host Path</Label>
              <Input
                id="edit-host-path"
                value={formHostPath}
                onChange={(e) => setFormHostPath(e.target.value)}
                className="font-mono"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-container-path">Container Path</Label>
              <Input
                id="edit-container-path"
                value={formContainerPath}
                onChange={(e) => setFormContainerPath(e.target.value)}
                className="font-mono"
              />
            </div>
            <div className="flex items-center gap-2">
              <Switch
                id="edit-read-only"
                checked={formReadOnly}
                onCheckedChange={setFormReadOnly}
              />
              <Label htmlFor="edit-read-only" className="text-sm font-normal cursor-pointer">
                Mount as read-only
              </Label>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowEditDialog(false)}>
              Cancel
            </Button>
            <Button onClick={handleSubmitEdit} disabled={updateMutation.isPending}>
              {updateMutation.isPending ? "Saving..." : "Save Changes"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <AlertDialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Volume</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete the volume <strong>{selectedVolume?.name}</strong>?
              This will unmount the directory from future containers. The data on the host will not be deleted.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => selectedVolume && deleteMutation.mutate(selectedVolume.id)}
              className="bg-red-500 hover:bg-red-600"
            >
              {deleteMutation.isPending ? "Deleting..." : "Delete"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </Card>
  );
}
