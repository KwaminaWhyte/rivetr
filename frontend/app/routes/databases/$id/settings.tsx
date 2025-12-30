import { useState } from "react";
import { useOutletContext, useNavigate } from "react-router";
import { useMutation } from "@tanstack/react-query";
import type { ManagedDatabase } from "@/types/api";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { toast } from "sonner";
import { api } from "@/lib/api";
import { AlertTriangle, Trash2, Settings2, Cpu, HardDrive } from "lucide-react";

interface OutletContext {
  database: ManagedDatabase;
}

export default function DatabaseSettingsTab() {
  const { database } = useOutletContext<OutletContext>();
  const navigate = useNavigate();
  const [deleteConfirmName, setDeleteConfirmName] = useState("");
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);

  const deleteMutation = useMutation({
    mutationFn: () => api.deleteDatabase(database.id),
    onSuccess: () => {
      toast.success("Database deleted");
      navigate("/projects");
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to delete database");
    },
  });

  const isSubmitting = deleteMutation.isPending;
  const canDelete = deleteConfirmName === database.name;

  const handleDelete = () => {
    deleteMutation.mutate();
  };

  return (
    <div className="space-y-6">
      {/* Resource Limits Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Cpu className="h-5 w-5" />
            Resource Limits
          </CardTitle>
          <CardDescription>
            Configure CPU and memory limits for this database
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="memory_limit">Memory Limit</Label>
              <Input
                id="memory_limit"
                value={database.memory_limit || "512mb"}
                readOnly
                className="font-mono"
              />
              <p className="text-xs text-muted-foreground">
                Maximum memory the database can use (e.g., 512mb, 1g, 2g)
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="cpu_limit">CPU Limit</Label>
              <Input
                id="cpu_limit"
                value={database.cpu_limit || "0.5"}
                readOnly
                className="font-mono"
              />
              <p className="text-xs text-muted-foreground">
                Maximum CPU cores the database can use (e.g., 0.5, 1, 2)
              </p>
            </div>
          </div>
          <div className="rounded-md bg-muted p-3">
            <p className="text-sm text-muted-foreground">
              Resource limits can only be changed by recreating the database.
              This feature will be available in a future update.
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Public Access Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Settings2 className="h-5 w-5" />
            Network Settings
          </CardTitle>
          <CardDescription>
            Configure network access settings
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between rounded-lg border p-4">
            <div className="space-y-0.5">
              <Label className="text-base">Public Access</Label>
              <p className="text-sm text-muted-foreground">
                Allow external connections to this database
              </p>
            </div>
            <div className="flex items-center gap-2">
              <Badge
                variant={database.public_access ? "default" : "secondary"}
                className={database.public_access ? "bg-green-500" : ""}
              >
                {database.public_access ? "Enabled" : "Disabled"}
              </Badge>
            </div>
          </div>
          <div className="rounded-md bg-muted p-3">
            <p className="text-sm text-muted-foreground">
              Changing public access requires recreating the database container.
              This feature will be available in a future update.
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Database Info Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <HardDrive className="h-5 w-5" />
            Database Information
          </CardTitle>
          <CardDescription>
            Read-only database configuration details
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label>Database ID</Label>
              <Input value={database.id} readOnly className="font-mono text-xs" />
            </div>
            <div className="space-y-2">
              <Label>Container ID</Label>
              <Input
                value={database.container_id || "Not running"}
                readOnly
                className="font-mono text-xs"
              />
            </div>
            <div className="space-y-2">
              <Label>Type</Label>
              <Input value={database.db_type.toUpperCase()} readOnly />
            </div>
            <div className="space-y-2">
              <Label>Version</Label>
              <Input value={database.version} readOnly />
            </div>
            <div className="space-y-2">
              <Label>Created</Label>
              <Input value={new Date(database.created_at).toLocaleString()} readOnly />
            </div>
            <div className="space-y-2">
              <Label>Last Updated</Label>
              <Input value={new Date(database.updated_at).toLocaleString()} readOnly />
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Danger Zone Card */}
      <Card className="border-destructive">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-destructive">
            <AlertTriangle className="h-5 w-5" />
            Danger Zone
          </CardTitle>
          <CardDescription>
            Irreversible actions that permanently affect your database
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="rounded-lg border border-destructive/50 p-4">
            <div className="flex items-start justify-between gap-4">
              <div className="space-y-1">
                <h4 className="font-medium text-destructive">Delete Database</h4>
                <p className="text-sm text-muted-foreground">
                  Permanently delete this database and all its data. This action cannot be undone.
                </p>
                <ul className="mt-2 text-sm text-muted-foreground list-disc list-inside">
                  <li>The database container will be stopped and removed</li>
                  <li>All data stored in the container will be lost</li>
                  <li>The database record will be permanently deleted</li>
                </ul>
              </div>
              <AlertDialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
                <AlertDialogTrigger asChild>
                  <Button variant="destructive" className="shrink-0">
                    <Trash2 className="h-4 w-4 mr-2" />
                    Delete Database
                  </Button>
                </AlertDialogTrigger>
                <AlertDialogContent>
                  <AlertDialogHeader>
                    <AlertDialogTitle className="flex items-center gap-2 text-destructive">
                      <AlertTriangle className="h-5 w-5" />
                      Delete Database
                    </AlertDialogTitle>
                    <AlertDialogDescription className="space-y-3">
                      <p>
                        This action is <strong>permanent and irreversible</strong>. Deleting this
                        database will:
                      </p>
                      <ul className="list-disc list-inside space-y-1">
                        <li>Stop and remove the database container</li>
                        <li>Delete all data stored in the container</li>
                        <li>Remove the database from your project</li>
                      </ul>
                      <p className="font-medium">
                        To confirm, type the database name: <code className="bg-muted rounded px-1">{database.name}</code>
                      </p>
                    </AlertDialogDescription>
                  </AlertDialogHeader>
                  <div className="py-4">
                    <Input
                      placeholder={`Type "${database.name}" to confirm`}
                      value={deleteConfirmName}
                      onChange={(e) => setDeleteConfirmName(e.target.value)}
                      className="font-mono"
                    />
                  </div>
                  <AlertDialogFooter>
                    <AlertDialogCancel onClick={() => setDeleteConfirmName("")}>
                      Cancel
                    </AlertDialogCancel>
                    <AlertDialogAction
                      onClick={handleDelete}
                      disabled={!canDelete || isSubmitting}
                      className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                    >
                      {isSubmitting ? "Deleting..." : "Delete Database"}
                    </AlertDialogAction>
                  </AlertDialogFooter>
                </AlertDialogContent>
              </AlertDialog>
            </div>
          </div>

          {/* Warning about data persistence */}
          {database.volume_path && (
            <div className="rounded-md bg-yellow-50 dark:bg-yellow-950 p-3">
              <p className="text-sm text-yellow-700 dark:text-yellow-300">
                <strong>Note:</strong> The data directory at{" "}
                <code className="bg-yellow-100 dark:bg-yellow-900 rounded px-1">
                  {database.volume_path}
                </code>{" "}
                will <strong>not</strong> be automatically deleted. You may want to manually remove
                it if you no longer need the data.
              </p>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
