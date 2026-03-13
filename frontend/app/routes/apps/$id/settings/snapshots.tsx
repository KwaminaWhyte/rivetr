import { useState } from "react";
import { useOutletContext } from "react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Camera, RotateCcw, Trash2 } from "lucide-react";
import { bulkApi } from "@/lib/api/bulk";
import type { ConfigSnapshot } from "@/types/api";
import type { App } from "@/types/api";

export default function AppSettingsSnapshots() {
  const { app } = useOutletContext<{ app: App }>();
  const queryClient = useQueryClient();

  const [showSnapshotDialog, setShowSnapshotDialog] = useState(false);
  const [snapshotName, setSnapshotName] = useState("");
  const [snapshotDescription, setSnapshotDescription] = useState("");
  const [isSavingSnapshot, setIsSavingSnapshot] = useState(false);

  const { data: snapshots = [], refetch: refetchSnapshots } = useQuery<ConfigSnapshot[]>({
    queryKey: ["snapshots", app.id],
    queryFn: () => bulkApi.listSnapshots(app.id),
  });

  const handleCreateSnapshot = async () => {
    if (!snapshotName.trim()) return;
    setIsSavingSnapshot(true);
    try {
      await bulkApi.createSnapshot(app.id, {
        name: snapshotName.trim(),
        description: snapshotDescription.trim() || undefined,
      });
      toast.success("Snapshot saved");
      setShowSnapshotDialog(false);
      setSnapshotName("");
      setSnapshotDescription("");
      refetchSnapshots();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to save snapshot");
    } finally {
      setIsSavingSnapshot(false);
    }
  };

  const handleRestoreSnapshot = async (snapshotId: string, name: string) => {
    try {
      await bulkApi.restoreSnapshot(app.id, snapshotId);
      toast.success(`Restored from snapshot "${name}"`);
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to restore snapshot");
    }
  };

  const handleDeleteSnapshot = async (snapshotId: string) => {
    try {
      await bulkApi.deleteSnapshot(app.id, snapshotId);
      toast.success("Snapshot deleted");
      refetchSnapshots();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to delete snapshot");
    }
  };

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <div>
            <CardTitle>Config Snapshots</CardTitle>
            <CardDescription>
              Save named snapshots of your app configuration for quick restore.
            </CardDescription>
          </div>
          <Button onClick={() => setShowSnapshotDialog(true)} className="gap-2">
            <Camera className="h-4 w-4" />
            Take Snapshot
          </Button>
        </CardHeader>
        <CardContent>
          {snapshots.length === 0 ? (
            <div className="py-8 text-center text-muted-foreground">
              No snapshots yet. Take a snapshot to save the current configuration.
            </div>
          ) : (
            <div className="space-y-3">
              {snapshots.map((snap) => (
                <div
                  key={snap.id}
                  className="flex items-center justify-between rounded-md border p-3"
                >
                  <div>
                    <p className="font-medium text-sm">{snap.name}</p>
                    {snap.description && (
                      <p className="text-xs text-muted-foreground">{snap.description}</p>
                    )}
                    <p className="text-xs text-muted-foreground mt-1">
                      {new Date(snap.created_at).toLocaleString()}
                    </p>
                  </div>
                  <div className="flex gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      className="gap-1.5"
                      onClick={() => handleRestoreSnapshot(snap.id, snap.name)}
                    >
                      <RotateCcw className="h-3.5 w-3.5" />
                      Restore
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="gap-1.5 text-destructive hover:text-destructive"
                      onClick={() => handleDeleteSnapshot(snap.id)}
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Snapshot Dialog */}
      <Dialog open={showSnapshotDialog} onOpenChange={(open) => {
        setShowSnapshotDialog(open);
        if (!open) { setSnapshotName(""); setSnapshotDescription(""); }
      }}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Take Config Snapshot</DialogTitle>
            <DialogDescription>
              Save a named snapshot of the current app configuration and (masked) env vars.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="space-y-2">
              <Label htmlFor="snap-name">Snapshot Name</Label>
              <Input
                id="snap-name"
                placeholder="e.g. pre-upgrade-backup"
                value={snapshotName}
                onChange={(e) => setSnapshotName(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="snap-desc">Description (optional)</Label>
              <Textarea
                id="snap-desc"
                placeholder="What changed / why this snapshot..."
                value={snapshotDescription}
                onChange={(e) => setSnapshotDescription(e.target.value)}
                rows={2}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowSnapshotDialog(false)} disabled={isSavingSnapshot}>
              Cancel
            </Button>
            <Button onClick={handleCreateSnapshot} disabled={isSavingSnapshot || !snapshotName.trim()} className="gap-2">
              <Camera className="h-4 w-4" />
              {isSavingSnapshot ? "Saving..." : "Save Snapshot"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
