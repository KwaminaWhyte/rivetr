import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
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
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Bell,
  Plus,
  Trash2,
  Cpu,
  HardDrive,
  MemoryStick,
  AlertTriangle,
  CheckCircle,
  Clock,
} from "lucide-react";
import { api } from "@/lib/api";
import type {
  AlertConfigResponse,
  AlertEventResponse,
  AlertMetricType,
  CreateAlertConfigRequest,
} from "@/types/api";

interface AlertsCardProps {
  appId: string;
}

const METRIC_OPTIONS: { value: AlertMetricType; label: string; icon: typeof Cpu }[] = [
  { value: "cpu", label: "CPU Usage", icon: Cpu },
  { value: "memory", label: "Memory Usage", icon: MemoryStick },
  { value: "disk", label: "Disk Usage", icon: HardDrive },
];

function getMetricIcon(metricType: string) {
  switch (metricType) {
    case "cpu":
      return Cpu;
    case "memory":
      return MemoryStick;
    case "disk":
      return HardDrive;
    default:
      return AlertTriangle;
  }
}

function formatMetricType(metricType: string): string {
  switch (metricType) {
    case "cpu":
      return "CPU";
    case "memory":
      return "Memory";
    case "disk":
      return "Disk";
    default:
      return metricType;
  }
}

function formatTimestamp(timestamp: string): string {
  const date = new Date(timestamp);
  return date.toLocaleString();
}

function formatRelativeTime(timestamp: string): string {
  const date = new Date(timestamp);
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (days > 0) return `${days}d ago`;
  if (hours > 0) return `${hours}h ago`;
  if (minutes > 0) return `${minutes}m ago`;
  return "just now";
}

export function AlertsCard({ appId }: AlertsCardProps) {
  const queryClient = useQueryClient();
  const [showAddDialog, setShowAddDialog] = useState(false);
  const [deleteAlertId, setDeleteAlertId] = useState<string | null>(null);
  const [newAlert, setNewAlert] = useState<CreateAlertConfigRequest>({
    metric_type: "cpu",
    threshold_percent: 80,
    enabled: true,
  });

  // Fetch alert configurations
  const { data: alerts = [], isLoading: alertsLoading } = useQuery({
    queryKey: ["alerts", appId],
    queryFn: () => api.getAlerts(appId),
  });

  // Fetch alert events (history)
  const { data: alertEvents = [], isLoading: eventsLoading } = useQuery({
    queryKey: ["alertEvents", appId],
    queryFn: () => api.getAlertEvents(appId, 20),
  });

  // Create alert mutation
  const createMutation = useMutation({
    mutationFn: (data: CreateAlertConfigRequest) => api.createAlert(appId, data),
    onSuccess: () => {
      toast.success("Alert configuration created");
      queryClient.invalidateQueries({ queryKey: ["alerts", appId] });
      setShowAddDialog(false);
      setNewAlert({ metric_type: "cpu", threshold_percent: 80, enabled: true });
    },
    onError: (error: Error) => {
      if (error.message.includes("409") || error.message.toLowerCase().includes("conflict")) {
        toast.error("An alert for this metric type already exists");
      } else {
        toast.error(error.message || "Failed to create alert");
      }
    },
  });

  // Update alert mutation
  const updateMutation = useMutation({
    mutationFn: ({ alertId, data }: { alertId: string; data: { enabled?: boolean; threshold_percent?: number } }) =>
      api.updateAlert(appId, alertId, data),
    onSuccess: () => {
      toast.success("Alert configuration updated");
      queryClient.invalidateQueries({ queryKey: ["alerts", appId] });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to update alert");
    },
  });

  // Delete alert mutation
  const deleteMutation = useMutation({
    mutationFn: (alertId: string) => api.deleteAlert(appId, alertId),
    onSuccess: () => {
      toast.success("Alert configuration deleted");
      queryClient.invalidateQueries({ queryKey: ["alerts", appId] });
      setDeleteAlertId(null);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to delete alert");
    },
  });

  const handleToggleAlert = (alert: AlertConfigResponse) => {
    updateMutation.mutate({
      alertId: alert.id,
      data: { enabled: !alert.enabled },
    });
  };

  const handleUpdateThreshold = (alert: AlertConfigResponse, threshold: number) => {
    if (threshold > 0 && threshold <= 100 && threshold !== alert.threshold_percent) {
      updateMutation.mutate({
        alertId: alert.id,
        data: { threshold_percent: threshold },
      });
    }
  };

  const handleCreateAlert = () => {
    if (newAlert.threshold_percent <= 0 || newAlert.threshold_percent > 100) {
      toast.error("Threshold must be between 1 and 100");
      return;
    }
    createMutation.mutate(newAlert);
  };

  // Get available metric types (those not already configured)
  const configuredMetrics = new Set(alerts.map((a) => a.metric_type));
  const availableMetrics = METRIC_OPTIONS.filter((m) => !configuredMetrics.has(m.value));

  const isLoading = alertsLoading || eventsLoading;

  return (
    <>
      {/* Alert Configurations Card */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Bell className="h-5 w-5" />
              Alert Thresholds
            </CardTitle>
            <CardDescription>
              Configure resource usage thresholds to receive alerts when limits are exceeded.
            </CardDescription>
          </div>
          {availableMetrics.length > 0 && (
            <Button
              size="sm"
              onClick={() => {
                setNewAlert({
                  metric_type: availableMetrics[0].value,
                  threshold_percent: 80,
                  enabled: true,
                });
                setShowAddDialog(true);
              }}
            >
              <Plus className="h-4 w-4 mr-1" />
              Add Alert
            </Button>
          )}
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="text-center py-4 text-muted-foreground">Loading...</div>
          ) : alerts.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <Bell className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p>No alert configurations yet.</p>
              <p className="text-sm mt-1">
                Add thresholds to get notified when resource usage exceeds limits.
              </p>
            </div>
          ) : (
            <div className="space-y-4">
              {alerts.map((alert) => {
                const Icon = getMetricIcon(alert.metric_type);
                return (
                  <div
                    key={alert.id}
                    className="flex items-center justify-between p-4 rounded-lg border"
                  >
                    <div className="flex items-center gap-4">
                      <div className="p-2 rounded-md bg-muted">
                        <Icon className="h-5 w-5" />
                      </div>
                      <div>
                        <div className="font-medium">
                          {formatMetricType(alert.metric_type)} Usage
                        </div>
                        <div className="text-sm text-muted-foreground">
                          Alert when above{" "}
                          <Input
                            type="number"
                            min={1}
                            max={100}
                            value={alert.threshold_percent}
                            onChange={(e) => {
                              const value = parseInt(e.target.value);
                              if (!isNaN(value)) {
                                handleUpdateThreshold(alert, value);
                              }
                            }}
                            className="inline-block w-16 h-6 px-2 mx-1 text-center"
                          />
                          %
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center gap-3">
                      <Switch
                        checked={alert.enabled}
                        onCheckedChange={() => handleToggleAlert(alert)}
                        disabled={updateMutation.isPending}
                      />
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => setDeleteAlertId(alert.id)}
                        disabled={deleteMutation.isPending}
                      >
                        <Trash2 className="h-4 w-4 text-muted-foreground hover:text-destructive" />
                      </Button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Alert History Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Clock className="h-5 w-5" />
            Alert History
          </CardTitle>
          <CardDescription>
            Recent alerts triggered by threshold breaches.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {eventsLoading ? (
            <div className="text-center py-4 text-muted-foreground">Loading...</div>
          ) : alertEvents.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <CheckCircle className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p>No alerts triggered yet.</p>
              <p className="text-sm mt-1">
                Alerts will appear here when thresholds are exceeded.
              </p>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Metric</TableHead>
                  <TableHead>Value</TableHead>
                  <TableHead>Threshold</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Time</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {alertEvents.map((event) => {
                  const Icon = getMetricIcon(event.metric_type);
                  const isFiring = event.status === "firing";
                  return (
                    <TableRow key={event.id}>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <Icon className="h-4 w-4 text-muted-foreground" />
                          {formatMetricType(event.metric_type)}
                        </div>
                      </TableCell>
                      <TableCell>
                        <span className={isFiring ? "text-destructive font-medium" : ""}>
                          {event.current_value.toFixed(1)}%
                        </span>
                      </TableCell>
                      <TableCell>{event.threshold_percent}%</TableCell>
                      <TableCell>
                        <Badge variant={isFiring ? "destructive" : "secondary"}>
                          {isFiring ? (
                            <AlertTriangle className="h-3 w-3 mr-1" />
                          ) : (
                            <CheckCircle className="h-3 w-3 mr-1" />
                          )}
                          {isFiring ? "Firing" : "Resolved"}
                        </Badge>
                      </TableCell>
                      <TableCell>
                        <span title={formatTimestamp(event.fired_at)}>
                          {formatRelativeTime(event.fired_at)}
                        </span>
                        {event.resolved_at && (
                          <span
                            className="text-muted-foreground ml-1"
                            title={`Resolved: ${formatTimestamp(event.resolved_at)}`}
                          >
                            (resolved {formatRelativeTime(event.resolved_at)})
                          </span>
                        )}
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Add Alert Dialog */}
      <Dialog open={showAddDialog} onOpenChange={setShowAddDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Alert Configuration</DialogTitle>
            <DialogDescription>
              Configure a threshold for resource usage alerts.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="metric-type">Metric Type</Label>
              <Select
                value={newAlert.metric_type}
                onValueChange={(value: AlertMetricType) =>
                  setNewAlert({ ...newAlert, metric_type: value })
                }
              >
                <SelectTrigger>
                  <SelectValue placeholder="Select metric type" />
                </SelectTrigger>
                <SelectContent>
                  {availableMetrics.map((metric) => {
                    const Icon = metric.icon;
                    return (
                      <SelectItem key={metric.value} value={metric.value}>
                        <div className="flex items-center gap-2">
                          <Icon className="h-4 w-4" />
                          {metric.label}
                        </div>
                      </SelectItem>
                    );
                  })}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="threshold">Threshold (%)</Label>
              <Input
                id="threshold"
                type="number"
                min={1}
                max={100}
                value={newAlert.threshold_percent}
                onChange={(e) =>
                  setNewAlert({
                    ...newAlert,
                    threshold_percent: parseInt(e.target.value) || 80,
                  })
                }
              />
              <p className="text-xs text-muted-foreground">
                Alert when usage exceeds this percentage.
              </p>
            </div>
            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label htmlFor="enabled">Enabled</Label>
                <p className="text-xs text-muted-foreground">
                  Enable this alert configuration.
                </p>
              </div>
              <Switch
                id="enabled"
                checked={newAlert.enabled}
                onCheckedChange={(checked) =>
                  setNewAlert({ ...newAlert, enabled: checked })
                }
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowAddDialog(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleCreateAlert}
              disabled={createMutation.isPending}
            >
              {createMutation.isPending ? "Creating..." : "Create Alert"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <Dialog open={!!deleteAlertId} onOpenChange={() => setDeleteAlertId(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Alert Configuration</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete this alert configuration? This action
              cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteAlertId(null)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={() => deleteAlertId && deleteMutation.mutate(deleteAlertId)}
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending ? "Deleting..." : "Delete"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
