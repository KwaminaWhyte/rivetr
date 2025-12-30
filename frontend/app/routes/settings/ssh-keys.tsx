import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
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
import type { SshKey } from "@/types/api";

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString();
}

export default function SettingsSshKeysPage() {
  const queryClient = useQueryClient();
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [selectedKeyId, setSelectedKeyId] = useState<string | null>(null);

  // Form state
  const [name, setName] = useState("");
  const [privateKey, setPrivateKey] = useState("");
  const [publicKey, setPublicKey] = useState("");

  const { data: sshKeys = [], isLoading } = useQuery<SshKey[]>({
    queryKey: ["ssh-keys"],
    queryFn: () => api.getSshKeys(),
  });

  const createMutation = useMutation({
    mutationFn: () =>
      api.createSshKey({
        name: name.trim(),
        private_key: privateKey.trim(),
        public_key: publicKey.trim() || undefined,
        is_global: true,
      }),
    onSuccess: () => {
      toast.success("SSH key created");
      queryClient.invalidateQueries({ queryKey: ["ssh-keys"] });
      setShowCreateDialog(false);
      resetForm();
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to create SSH key");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (keyId: string) => api.deleteSshKey(keyId),
    onSuccess: () => {
      toast.success("SSH key deleted");
      queryClient.invalidateQueries({ queryKey: ["ssh-keys"] });
      setShowDeleteDialog(false);
      setSelectedKeyId(null);
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to delete SSH key");
    },
  });

  const resetForm = () => {
    setName("");
    setPrivateKey("");
    setPublicKey("");
  };

  const handleCreateSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) {
      toast.error("Name is required");
      return;
    }
    if (!privateKey.trim()) {
      toast.error("Private key is required");
      return;
    }
    createMutation.mutate();
  };

  const handleDeleteConfirm = () => {
    if (selectedKeyId) {
      deleteMutation.mutate(selectedKeyId);
    }
  };

  const isSubmitting = createMutation.isPending || deleteMutation.isPending;

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
            <p className="text-muted-foreground py-4 text-center">Loading...</p>
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
      <Dialog open={showCreateDialog} onOpenChange={(open) => {
        setShowCreateDialog(open);
        if (!open) resetForm();
      }}>
        <DialogContent className="max-w-2xl">
          <form onSubmit={handleCreateSubmit}>
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
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="e.g., github-deploy-key"
                  required
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="private_key">Private Key</Label>
                <Textarea
                  id="private_key"
                  value={privateKey}
                  onChange={(e) => setPrivateKey(e.target.value)}
                  placeholder="-----BEGIN OPENSSH PRIVATE KEY-----
...
-----END OPENSSH PRIVATE KEY-----"
                  className="font-mono text-xs min-h-48"
                  required
                />
                <p className="text-xs text-muted-foreground">
                  Paste your SSH private key. This is typically found in ~/.ssh/id_ed25519 or ~/.ssh/id_rsa
                </p>
              </div>
              <div className="space-y-2">
                <Label htmlFor="public_key">Public Key (optional)</Label>
                <Textarea
                  id="public_key"
                  value={publicKey}
                  onChange={(e) => setPublicKey(e.target.value)}
                  placeholder="ssh-ed25519 AAAA... user@host"
                  className="font-mono text-xs min-h-16"
                />
                <p className="text-xs text-muted-foreground">
                  Optionally provide the public key for reference
                </p>
              </div>
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => setShowCreateDialog(false)}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? "Creating..." : "Create"}
              </Button>
            </DialogFooter>
          </form>
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
              onClick={handleDeleteConfirm}
              disabled={isSubmitting}
            >
              {isSubmitting ? "Deleting..." : "Delete"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
