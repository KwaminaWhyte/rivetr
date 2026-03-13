import { useState } from "react";
import { useOutletContext } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Plus, Trash2, RotateCcw } from "lucide-react";
import { replicasApi, type AppReplica } from "@/lib/api/replicas";
import { autoscalingApi } from "@/lib/api/autoscaling";
import type { App, AutoscalingRule, CreateAutoscalingRuleRequest } from "@/types/api";

export default function AppSettingsReplicas() {
  const { app } = useOutletContext<{ app: App }>();

  // Replicas state
  const [replicaCount, setReplicaCount] = useState(app.replica_count ?? 1);
  const [isSavingReplicas, setIsSavingReplicas] = useState(false);
  const [restartingReplica, setRestartingReplica] = useState<number | null>(null);

  const { data: replicas = [], refetch: refetchReplicas } = useQuery<AppReplica[]>({
    queryKey: ["replicas", app.id],
    queryFn: () => replicasApi.list(app.id),
  });

  // Autoscaling state
  const { data: autoscalingRules = [], refetch: refetchAutoscaling } = useQuery<AutoscalingRule[]>({
    queryKey: ["autoscaling", app.id],
    queryFn: () => autoscalingApi.list(app.id),
  });
  const [showAutoscalingDialog, setShowAutoscalingDialog] = useState(false);
  const [editingRule, setEditingRule] = useState<AutoscalingRule | null>(null);
  const [autoscalingForm, setAutoscalingForm] = useState<CreateAutoscalingRuleRequest>({
    metric: "cpu",
    scale_up_threshold: 80,
    scale_down_threshold: 20,
    min_replicas: 1,
    max_replicas: 10,
    cooldown_seconds: 300,
    enabled: true,
  });
  const [isSavingAutoscaling, setIsSavingAutoscaling] = useState(false);

  const handleSetReplicaCount = async () => {
    setIsSavingReplicas(true);
    try {
      await replicasApi.setCount(app.id, replicaCount);
      toast.success(`Replica count updated to ${replicaCount}`);
      refetchReplicas();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to update replica count");
    } finally {
      setIsSavingReplicas(false);
    }
  };

  const handleRestartReplica = async (index: number) => {
    setRestartingReplica(index);
    try {
      await replicasApi.restart(app.id, index);
      toast.success(`Replica ${index} restarted`);
      refetchReplicas();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to restart replica");
    } finally {
      setRestartingReplica(null);
    }
  };

  const handleOpenAutoscalingDialog = (rule?: AutoscalingRule) => {
    if (rule) {
      setEditingRule(rule);
      setAutoscalingForm({
        metric: rule.metric,
        scale_up_threshold: rule.scale_up_threshold,
        scale_down_threshold: rule.scale_down_threshold,
        min_replicas: rule.min_replicas,
        max_replicas: rule.max_replicas,
        cooldown_seconds: rule.cooldown_seconds,
        enabled: rule.enabled === 1,
      });
    } else {
      setEditingRule(null);
      setAutoscalingForm({
        metric: "cpu",
        scale_up_threshold: 80,
        scale_down_threshold: 20,
        min_replicas: 1,
        max_replicas: 10,
        cooldown_seconds: 300,
        enabled: true,
      });
    }
    setShowAutoscalingDialog(true);
  };

  const handleSaveAutoscalingRule = async () => {
    setIsSavingAutoscaling(true);
    try {
      if (editingRule) {
        await autoscalingApi.update(app.id, editingRule.id, autoscalingForm);
        toast.success("Autoscaling rule updated");
      } else {
        await autoscalingApi.create(app.id, autoscalingForm);
        toast.success("Autoscaling rule created");
      }
      setShowAutoscalingDialog(false);
      refetchAutoscaling();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to save rule");
    } finally {
      setIsSavingAutoscaling(false);
    }
  };

  const handleDeleteAutoscalingRule = async (ruleId: string) => {
    try {
      await autoscalingApi.delete(app.id, ruleId);
      toast.success("Rule deleted");
      refetchAutoscaling();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to delete rule");
    }
  };

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Container Replicas</CardTitle>
          <CardDescription>
            Run multiple container instances to distribute load. Changes apply on the next deployment.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Replica count input */}
          <div className="flex items-end gap-4">
            <div className="space-y-2 flex-1 max-w-xs">
              <Label htmlFor="replica-count">Replica Count</Label>
              <Input
                id="replica-count"
                type="number"
                min={1}
                max={10}
                value={replicaCount}
                onChange={(e) => setReplicaCount(Math.max(1, Math.min(10, parseInt(e.target.value) || 1)))}
              />
              <p className="text-xs text-muted-foreground">
                Number of container instances to run (1–10)
              </p>
            </div>
            <Button onClick={handleSetReplicaCount} disabled={isSavingReplicas}>
              {isSavingReplicas ? "Updating..." : "Update"}
            </Button>
          </div>

          {/* Replica status table */}
          {replicas.length > 0 ? (
            <div className="rounded-md border">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b bg-muted/50">
                    <th className="px-4 py-2 text-left font-medium">Index</th>
                    <th className="px-4 py-2 text-left font-medium">Container ID</th>
                    <th className="px-4 py-2 text-left font-medium">Status</th>
                    <th className="px-4 py-2 text-left font-medium">Started At</th>
                    <th className="px-4 py-2 text-right font-medium">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {replicas.map((replica) => (
                    <tr key={replica.id} className="border-b last:border-0">
                      <td className="px-4 py-2 font-mono">{replica.replica_index}</td>
                      <td className="px-4 py-2 font-mono text-xs text-muted-foreground">
                        {replica.container_id ? replica.container_id.slice(0, 12) : "—"}
                      </td>
                      <td className="px-4 py-2">
                        <Badge
                          variant={
                            replica.status === "running"
                              ? "secondary"
                              : replica.status === "error"
                              ? "destructive"
                              : "outline"
                          }
                          className="text-xs"
                        >
                          {replica.status}
                        </Badge>
                      </td>
                      <td className="px-4 py-2 text-xs text-muted-foreground">
                        {replica.started_at
                          ? new Date(replica.started_at).toLocaleString()
                          : "—"}
                      </td>
                      <td className="px-4 py-2 text-right">
                        <Button
                          variant="ghost"
                          size="sm"
                          className="gap-1.5"
                          disabled={restartingReplica === replica.replica_index}
                          onClick={() => handleRestartReplica(replica.replica_index)}
                        >
                          <RotateCcw className="h-3.5 w-3.5" />
                          {restartingReplica === replica.replica_index ? "Restarting..." : "Restart"}
                        </Button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <div className="py-8 text-center text-muted-foreground">
              No replica data yet. Deploy your app to start tracking replicas.
            </div>
          )}
        </CardContent>
      </Card>

      {/* Autoscaling */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <div>
            <CardTitle>Auto-scaling</CardTitle>
            <CardDescription>
              Automatically scale replicas based on CPU or memory usage.
            </CardDescription>
          </div>
          <Button size="sm" className="gap-2" onClick={() => handleOpenAutoscalingDialog()}>
            <Plus className="h-4 w-4" />
            Add Rule
          </Button>
        </CardHeader>
        <CardContent>
          {autoscalingRules.length === 0 ? (
            <div className="py-8 text-center text-muted-foreground">
              No autoscaling rules configured. Add a rule to enable automatic scaling.
            </div>
          ) : (
            <div className="space-y-3">
              {autoscalingRules.map((rule) => (
                <div
                  key={rule.id}
                  className="flex items-center justify-between rounded-md border p-3"
                >
                  <div className="space-y-1">
                    <div className="flex items-center gap-2">
                      <p className="font-medium text-sm capitalize">{rule.metric}</p>
                      <Badge variant={rule.enabled === 1 ? "secondary" : "outline"} className="text-xs">
                        {rule.enabled === 1 ? "Enabled" : "Disabled"}
                      </Badge>
                    </div>
                    <p className="text-xs text-muted-foreground">
                      Scale up at {rule.scale_up_threshold}% · Scale down at {rule.scale_down_threshold}%
                      {" "}· {rule.min_replicas}–{rule.max_replicas} replicas · {rule.cooldown_seconds}s cooldown
                    </p>
                  </div>
                  <div className="flex gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleOpenAutoscalingDialog(rule)}
                    >
                      Edit
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="text-destructive hover:text-destructive"
                      onClick={() => handleDeleteAutoscalingRule(rule.id)}
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

      {/* Autoscaling Rule Dialog */}
      <Dialog open={showAutoscalingDialog} onOpenChange={setShowAutoscalingDialog}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>{editingRule ? "Edit" : "Add"} Autoscaling Rule</DialogTitle>
            <DialogDescription>
              Configure when to scale replicas up or down based on a metric threshold.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="space-y-2">
              <Label>Metric</Label>
              <Select
                value={autoscalingForm.metric}
                onValueChange={(v) =>
                  setAutoscalingForm({ ...autoscalingForm, metric: v as "cpu" | "memory" | "request_rate" })
                }
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="cpu">CPU %</SelectItem>
                  <SelectItem value="memory">Memory %</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="scale-up">Scale Up Threshold (%)</Label>
                <Input
                  id="scale-up"
                  type="number"
                  min={0}
                  max={100}
                  value={autoscalingForm.scale_up_threshold}
                  onChange={(e) =>
                    setAutoscalingForm({
                      ...autoscalingForm,
                      scale_up_threshold: parseFloat(e.target.value) || 80,
                    })
                  }
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="scale-down">Scale Down Threshold (%)</Label>
                <Input
                  id="scale-down"
                  type="number"
                  min={0}
                  max={100}
                  value={autoscalingForm.scale_down_threshold}
                  onChange={(e) =>
                    setAutoscalingForm({
                      ...autoscalingForm,
                      scale_down_threshold: parseFloat(e.target.value) || 20,
                    })
                  }
                />
              </div>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="min-replicas">Min Replicas</Label>
                <Input
                  id="min-replicas"
                  type="number"
                  min={1}
                  max={100}
                  value={autoscalingForm.min_replicas}
                  onChange={(e) =>
                    setAutoscalingForm({
                      ...autoscalingForm,
                      min_replicas: parseInt(e.target.value) || 1,
                    })
                  }
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="max-replicas">Max Replicas</Label>
                <Input
                  id="max-replicas"
                  type="number"
                  min={1}
                  max={100}
                  value={autoscalingForm.max_replicas}
                  onChange={(e) =>
                    setAutoscalingForm({
                      ...autoscalingForm,
                      max_replicas: parseInt(e.target.value) || 10,
                    })
                  }
                />
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="cooldown">Cooldown (seconds)</Label>
              <Input
                id="cooldown"
                type="number"
                min={30}
                value={autoscalingForm.cooldown_seconds}
                onChange={(e) =>
                  setAutoscalingForm({
                    ...autoscalingForm,
                    cooldown_seconds: parseInt(e.target.value) || 300,
                  })
                }
              />
              <p className="text-xs text-muted-foreground">
                Minimum time between scaling actions
              </p>
            </div>
            <div className="flex items-center justify-between rounded-lg border p-3">
              <Label htmlFor="as-enabled">Enabled</Label>
              <Switch
                id="as-enabled"
                checked={autoscalingForm.enabled}
                onCheckedChange={(v) => setAutoscalingForm({ ...autoscalingForm, enabled: v })}
              />
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setShowAutoscalingDialog(false)}
              disabled={isSavingAutoscaling}
            >
              Cancel
            </Button>
            <Button onClick={handleSaveAutoscalingRule} disabled={isSavingAutoscaling}>
              {isSavingAutoscaling ? "Saving..." : editingRule ? "Update" : "Create"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
