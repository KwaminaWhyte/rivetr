import { useState } from "react";
import { Form, useNavigation } from "react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import type { Route } from "./+types/ssh-keys";
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

export async function loader({ request }: Route.LoaderArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const sshKeys = await api.getSshKeys(token).catch(() => []);
  return { sshKeys };
}

export async function action({ request }: Route.ActionArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  if (intent === "create") {
    const name = formData.get("name");
    const private_key = formData.get("private_key");
    const public_key = formData.get("public_key");

    if (typeof name !== "string" || !name.trim()) {
      return { error: "Name is required" };
    }
    if (typeof private_key !== "string" || !private_key.trim()) {
      return { error: "Private key is required" };
    }

    try {
      await api.createSshKey(token, {
        name: name.trim(),
        private_key: private_key.trim(),
        public_key: typeof public_key === "string" ? public_key.trim() || undefined : undefined,
        is_global: true,
      });
      return { success: true, action: "create" };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to create SSH key" };
    }
  }

  if (intent === "delete") {
    const keyId = formData.get("keyId");
    if (typeof keyId !== "string") {
      return { error: "Key ID is required" };
    }
    try {
      await api.deleteSshKey(token, keyId);
      return { success: true, action: "delete" };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to delete SSH key" };
    }
  }

  return { error: "Unknown action" };
}

export default function SettingsSshKeysPage({ loaderData, actionData }: Route.ComponentProps) {
  const queryClient = useQueryClient();
  const navigation = useNavigation();
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [selectedKeyId, setSelectedKeyId] = useState<string | null>(null);

  const { data: sshKeys = [] } = useQuery<SshKey[]>({
    queryKey: ["ssh-keys"],
    queryFn: () => api.getSshKeys(),
    initialData: loaderData.sshKeys,
  });

  const isSubmitting = navigation.state === "submitting";

  // Handle success actions
  if (actionData?.success) {
    if (actionData.action === "create") {
      toast.success("SSH key created");
      if (showCreateDialog) setShowCreateDialog(false);
    } else if (actionData.action === "delete") {
      toast.success("SSH key deleted");
      if (showDeleteDialog) {
        setShowDeleteDialog(false);
        setSelectedKeyId(null);
      }
    }
    queryClient.invalidateQueries({ queryKey: ["ssh-keys"] });
  }

  if (actionData?.error) {
    toast.error(actionData.error);
  }

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
          {sshKeys.length === 0 ? (
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
          <Form method="post">
            <input type="hidden" name="intent" value="create" />
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
                  name="name"
                  placeholder="e.g., github-deploy-key"
                  required
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="private_key">Private Key</Label>
                <Textarea
                  id="private_key"
                  name="private_key"
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
                  name="public_key"
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
          </Form>
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
            <Form method="post">
              <input type="hidden" name="intent" value="delete" />
              <input type="hidden" name="keyId" value={selectedKeyId || ""} />
              <Button type="submit" variant="destructive" disabled={isSubmitting}>
                {isSubmitting ? "Deleting..." : "Delete"}
              </Button>
            </Form>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
