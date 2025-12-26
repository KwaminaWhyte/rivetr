import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
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
import { Textarea } from "@/components/ui/textarea";
import { api } from "@/lib/api";
import type { SshKey, CreateSshKeyRequest } from "@/types/api";

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString();
}

export function SettingsSshKeysPage() {
  const queryClient = useQueryClient();
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [selectedKeyId, setSelectedKeyId] = useState<string | null>(null);
  const [newKey, setNewKey] = useState<CreateSshKeyRequest>({
    name: "",
    private_key: "",
    is_global: true,
  });

  const { data: sshKeys = [], isLoading } = useQuery<SshKey[]>({
    queryKey: ["ssh-keys"],
    queryFn: () => api.getSshKeys(),
  });

  const createMutation = useMutation({
    mutationFn: (data: CreateSshKeyRequest) => api.createSshKey(data),
    onSuccess: () => {
      toast.success("SSH key created");
      queryClient.invalidateQueries({ queryKey: ["ssh-keys"] });
      setShowCreateDialog(false);
      setNewKey({ name: "", private_key: "", is_global: true });
    },
    onError: (error: Error) => {
      toast.error(`Failed to create SSH key: ${error.message}`);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => api.deleteSshKey(id),
    onSuccess: () => {
      toast.success("SSH key deleted");
      queryClient.invalidateQueries({ queryKey: ["ssh-keys"] });
      setShowDeleteDialog(false);
      setSelectedKeyId(null);
    },
    onError: (error: Error) => {
      toast.error(`Failed to delete SSH key: ${error.message}`);
    },
  });

  const handleCreate = () => {
    if (!newKey.name.trim()) {
      toast.error("Name is required");
      return;
    }
    if (!newKey.private_key.trim()) {
      toast.error("Private key is required");
      return;
    }
    createMutation.mutate(newKey);
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">SSH Keys</h1>
          <p className="text-muted-foreground">
            Manage SSH keys for private Git repositories
          </p>
        </div>
        <Button onClick={() => setShowCreateDialog(true)}>Add SSH Key</Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>SSH Keys</CardTitle>
          <CardDescription>
            SSH keys are used to authenticate with private Git repositories during deployment.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-4">
              {[1, 2, 3].map((i) => (
                <Skeleton key={i} className="h-12 w-full" />
              ))}
            </div>
          ) : sshKeys.length === 0 ? (
            <p className="text-muted-foreground py-4 text-center">
              No SSH keys configured. Add one to deploy from private repositories.
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Scope</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead className="w-24">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {sshKeys.map((key) => (
                  <TableRow key={key.id}>
                    <TableCell className="font-medium">{key.name}</TableCell>
                    <TableCell>
                      {key.is_global ? (
                        <Badge>Global</Badge>
                      ) : (
                        <Badge variant="outline">App-specific</Badge>
                      )}
                    </TableCell>
                    <TableCell>{formatDate(key.created_at)}</TableCell>
                    <TableCell>
                      <Button
                        variant="destructive"
                        size="sm"
                        onClick={() => {
                          setSelectedKeyId(key.id);
                          setShowDeleteDialog(true);
                        }}
                      >
                        Delete
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Create SSH Key Dialog */}
      <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Add SSH Key</DialogTitle>
            <DialogDescription>
              Add a new SSH key for authenticating with private Git repositories.
              The private key will be stored securely.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="name">Name</Label>
              <Input
                id="name"
                placeholder="e.g., github-deploy-key"
                value={newKey.name}
                onChange={(e) => setNewKey({ ...newKey, name: e.target.value })}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="private_key">Private Key</Label>
              <Textarea
                id="private_key"
                placeholder="-----BEGIN OPENSSH PRIVATE KEY-----
...
-----END OPENSSH PRIVATE KEY-----"
                className="font-mono text-xs min-h-48"
                value={newKey.private_key}
                onChange={(e) =>
                  setNewKey({ ...newKey, private_key: e.target.value })
                }
              />
              <p className="text-xs text-muted-foreground">
                Paste your SSH private key. This is typically found in ~/.ssh/id_ed25519 or ~/.ssh/id_rsa
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="public_key">Public Key (optional)</Label>
              <Textarea
                id="public_key"
                placeholder="ssh-ed25519 AAAA... user@host"
                className="font-mono text-xs min-h-16"
                value={newKey.public_key || ""}
                onChange={(e) =>
                  setNewKey({ ...newKey, public_key: e.target.value || undefined })
                }
              />
              <p className="text-xs text-muted-foreground">
                Optionally provide the public key for reference
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setShowCreateDialog(false);
                setNewKey({ name: "", private_key: "", is_global: true });
              }}
            >
              Cancel
            </Button>
            <Button
              onClick={handleCreate}
              disabled={createMutation.isPending}
            >
              {createMutation.isPending ? "Creating..." : "Create"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <Dialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete SSH Key</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete this SSH key? Apps using this key
              will no longer be able to deploy from private repositories.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setShowDeleteDialog(false);
                setSelectedKeyId(null);
              }}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={() => {
                if (selectedKeyId) {
                  deleteMutation.mutate(selectedKeyId);
                }
              }}
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending ? "Deleting..." : "Delete"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
