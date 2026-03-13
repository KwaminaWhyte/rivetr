import { useState, useEffect } from "react";
import { useOutletContext } from "react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Shield, Snowflake, Plus, Trash2 } from "lucide-react";
import { api } from "@/lib/api";
import type { App, DeploymentFreezeWindow, CreateFreezeWindowRequest } from "@/types/api";

export default function AppSettingsDeployment() {
  const { app } = useOutletContext<{ app: App }>();
  const queryClient = useQueryClient();

  // Approval & maintenance mode state
  const [requireApproval, setRequireApproval] = useState(app.require_approval ?? false);
  const [maintenanceMode, setMaintenanceMode] = useState(app.maintenance_mode ?? false);
  const [maintenanceMessage, setMaintenanceMessage] = useState(
    app.maintenance_message ?? "Service temporarily unavailable"
  );
  const [isSavingDeployControl, setIsSavingDeployControl] = useState(false);

  // Freeze windows state
  const [showFreezeWindowDialog, setShowFreezeWindowDialog] = useState(false);
  const [freezeWindowForm, setFreezeWindowForm] = useState<CreateFreezeWindowRequest>({
    name: "",
    start_time: "22:00",
    end_time: "06:00",
    days_of_week: "0,1,2,3,4,5,6",
    app_id: app.id,
  });
  const [isSavingFreezeWindow, setIsSavingFreezeWindow] = useState(false);

  // Rollback retention state
  const [rollbackRetentionCount, setRollbackRetentionCount] = useState(
    app.rollback_retention_count ?? 10
  );
  const [isSavingRetention, setIsSavingRetention] = useState(false);

  // Sync approval/maintenance from app data
  useEffect(() => {
    setRequireApproval(app.require_approval ?? false);
    setMaintenanceMode(app.maintenance_mode ?? false);
    setMaintenanceMessage(app.maintenance_message ?? "Service temporarily unavailable");
  }, [app.require_approval, app.maintenance_mode, app.maintenance_message]);

  useEffect(() => {
    setRollbackRetentionCount(app.rollback_retention_count ?? 10);
  }, [app.rollback_retention_count]);

  // Freeze windows query
  const { data: freezeWindows = [], refetch: refetchFreezeWindows } = useQuery<
    DeploymentFreezeWindow[]
  >({
    queryKey: ["freeze-windows", app.id],
    queryFn: () => api.getFreezeWindows({ appId: app.id }),
  });

  const handleSaveDeployControl = async () => {
    setIsSavingDeployControl(true);
    try {
      await api.updateApp(app.id, {
        require_approval: requireApproval,
        maintenance_mode: maintenanceMode,
        maintenance_message: maintenanceMessage,
      });
      toast.success("Deployment control settings saved");
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to save settings");
    } finally {
      setIsSavingDeployControl(false);
    }
  };

  const handleCreateFreezeWindow = async () => {
    if (!freezeWindowForm.name.trim()) return;
    setIsSavingFreezeWindow(true);
    try {
      await api.createFreezeWindow(freezeWindowForm);
      toast.success("Freeze window created");
      setShowFreezeWindowDialog(false);
      setFreezeWindowForm({
        name: "",
        start_time: "22:00",
        end_time: "06:00",
        days_of_week: "0,1,2,3,4,5,6",
        app_id: app.id,
      });
      refetchFreezeWindows();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to create freeze window");
    } finally {
      setIsSavingFreezeWindow(false);
    }
  };

  const handleDeleteFreezeWindow = async (id: string) => {
    try {
      await api.deleteFreezeWindow(app.id, id);
      toast.success("Freeze window deleted");
      refetchFreezeWindows();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to delete freeze window");
    }
  };

  const handleSaveRetention = async () => {
    setIsSavingRetention(true);
    try {
      await api.updateApp(app.id, { rollback_retention_count: rollbackRetentionCount });
      toast.success("Rollback retention saved");
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to save");
    } finally {
      setIsSavingRetention(false);
    }
  };

  return (
    <div className="space-y-6">
      {/* Approval & Maintenance */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Shield className="h-5 w-5" />
            Deployment Control
          </CardTitle>
          <CardDescription>
            Control how deployments are triggered and when the app is accessible.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Require Approval toggle */}
          <div className="flex items-center justify-between rounded-lg border p-4">
            <div className="space-y-0.5">
              <Label htmlFor="require-approval" className="text-base">
                Require Approval
              </Label>
              <p className="text-sm text-muted-foreground">
                Non-admin users must have their deployments approved by an admin before they run.
              </p>
            </div>
            <Switch
              id="require-approval"
              checked={requireApproval}
              onCheckedChange={setRequireApproval}
            />
          </div>

          {/* Maintenance Mode toggle */}
          <div className="space-y-3">
            <div className="flex items-center justify-between rounded-lg border p-4">
              <div className="space-y-0.5">
                <Label htmlFor="maintenance-mode" className="text-base">
                  Maintenance Mode
                </Label>
                <p className="text-sm text-muted-foreground">
                  Show a maintenance page instead of the live application.
                </p>
              </div>
              <Switch
                id="maintenance-mode"
                checked={maintenanceMode}
                onCheckedChange={setMaintenanceMode}
              />
            </div>
            {maintenanceMode && (
              <div className="space-y-2 ml-4">
                <Label htmlFor="maintenance-message">Maintenance Message</Label>
                <Input
                  id="maintenance-message"
                  value={maintenanceMessage}
                  onChange={(e) => setMaintenanceMessage(e.target.value)}
                  placeholder="Service temporarily unavailable"
                />
                <p className="text-xs text-muted-foreground">
                  Message shown to visitors during maintenance
                </p>
              </div>
            )}
          </div>

          <Button
            onClick={handleSaveDeployControl}
            disabled={isSavingDeployControl}
          >
            {isSavingDeployControl ? "Saving..." : "Save Changes"}
          </Button>
        </CardContent>
      </Card>

      {/* Freeze Windows */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Snowflake className="h-5 w-5" />
              Deployment Freeze Windows
            </CardTitle>
            <CardDescription>
              Block deployments during specific time windows (e.g., business hours, weekends).
            </CardDescription>
          </div>
          <Button
            size="sm"
            className="gap-2"
            onClick={() => setShowFreezeWindowDialog(true)}
          >
            <Plus className="h-4 w-4" />
            Add Window
          </Button>
        </CardHeader>
        <CardContent>
          {freezeWindows.length === 0 ? (
            <div className="py-8 text-center text-muted-foreground">
              No freeze windows configured. Add one to block deployments during specific times.
            </div>
          ) : (
            <div className="space-y-3">
              {freezeWindows.map((fw) => (
                <div
                  key={fw.id}
                  className="flex items-center justify-between rounded-md border p-3"
                >
                  <div className="space-y-0.5">
                    <div className="flex items-center gap-2">
                      <p className="font-medium text-sm">{fw.name}</p>
                      {fw.is_active ? (
                        <Badge variant="secondary" className="text-xs">Active</Badge>
                      ) : (
                        <Badge variant="outline" className="text-xs text-muted-foreground">Inactive</Badge>
                      )}
                    </div>
                    <p className="text-xs text-muted-foreground">
                      {fw.start_time} – {fw.end_time} UTC
                      {" "}·{" "}
                      Days: {fw.days_of_week}
                    </p>
                  </div>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="gap-1.5 text-destructive hover:text-destructive"
                    onClick={() => handleDeleteFreezeWindow(fw.id)}
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </Button>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Rollback Retention */}
      <Card>
        <CardHeader>
          <CardTitle>Rollback Retention</CardTitle>
          <CardDescription>
            Number of previous successful deployments to keep available for rollback.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-end gap-4">
            <div className="space-y-2 flex-1 max-w-xs">
              <Label htmlFor="rollback-retention">Retention Count</Label>
              <Input
                id="rollback-retention"
                type="number"
                min={1}
                max={50}
                value={rollbackRetentionCount}
                onChange={(e) =>
                  setRollbackRetentionCount(
                    Math.max(1, Math.min(50, parseInt(e.target.value) || 10))
                  )
                }
              />
              <p className="text-xs text-muted-foreground">
                Number of deployments to retain for rollback (1–50, default 10).
                Older successful deployments will be automatically deleted.
              </p>
            </div>
            <Button onClick={handleSaveRetention} disabled={isSavingRetention}>
              {isSavingRetention ? "Saving..." : "Save"}
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Freeze Window Create Dialog */}
      <Dialog
        open={showFreezeWindowDialog}
        onOpenChange={(open) => {
          setShowFreezeWindowDialog(open);
          if (!open) {
            setFreezeWindowForm({
              name: "",
              start_time: "22:00",
              end_time: "06:00",
              days_of_week: "0,1,2,3,4,5,6",
              app_id: app.id,
            });
          }
        }}
      >
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Add Freeze Window</DialogTitle>
            <DialogDescription>
              Define a time window during which deployments will be blocked. Times are in UTC.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="space-y-2">
              <Label htmlFor="fw-name">Name</Label>
              <Input
                id="fw-name"
                placeholder="e.g. Business Hours"
                value={freezeWindowForm.name}
                onChange={(e) =>
                  setFreezeWindowForm({ ...freezeWindowForm, name: e.target.value })
                }
              />
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="fw-start">Start Time (UTC)</Label>
                <Input
                  id="fw-start"
                  type="time"
                  value={freezeWindowForm.start_time}
                  onChange={(e) =>
                    setFreezeWindowForm({
                      ...freezeWindowForm,
                      start_time: e.target.value,
                    })
                  }
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="fw-end">End Time (UTC)</Label>
                <Input
                  id="fw-end"
                  type="time"
                  value={freezeWindowForm.end_time}
                  onChange={(e) =>
                    setFreezeWindowForm({
                      ...freezeWindowForm,
                      end_time: e.target.value,
                    })
                  }
                />
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="fw-days">Days of Week</Label>
              <Input
                id="fw-days"
                placeholder="0,1,2,3,4,5,6"
                value={freezeWindowForm.days_of_week}
                onChange={(e) =>
                  setFreezeWindowForm({
                    ...freezeWindowForm,
                    days_of_week: e.target.value,
                  })
                }
              />
              <p className="text-xs text-muted-foreground">
                Comma-separated: 0=Sunday, 1=Monday, ... 6=Saturday. Leave blank for all days.
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setShowFreezeWindowDialog(false)}
              disabled={isSavingFreezeWindow}
            >
              Cancel
            </Button>
            <Button
              onClick={handleCreateFreezeWindow}
              disabled={isSavingFreezeWindow || !freezeWindowForm.name.trim()}
              className="gap-2"
            >
              <Snowflake className="h-4 w-4" />
              {isSavingFreezeWindow ? "Creating..." : "Create Window"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
