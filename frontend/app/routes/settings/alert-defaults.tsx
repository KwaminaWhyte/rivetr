import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import { api } from "@/lib/api";
import type {
  GlobalAlertDefaultsResponse,
  GlobalAlertDefaultResponse,
  AlertStatsResponse,
} from "@/types/api";
import { Loader2, Cpu, MemoryStick, HardDrive, Info } from "lucide-react";

export function meta() {
  return [
    { title: "Alert Defaults - Rivetr" },
    { name: "description", content: "Configure global alert defaults for resource monitoring" },
  ];
}

interface ThresholdCardProps {
  title: string;
  icon: React.ReactNode;
  metricType: "cpu" | "memory" | "disk";
  config: GlobalAlertDefaultResponse | null;
  onUpdate: (metricType: "cpu" | "memory" | "disk", threshold: number, enabled: boolean) => void;
  isUpdating: boolean;
}

function ThresholdCard({ title, icon, metricType, config, onUpdate, isUpdating }: ThresholdCardProps) {
  const [threshold, setThreshold] = useState(config?.threshold_percent?.toString() ?? "80");
  const [enabled, setEnabled] = useState(config?.enabled ?? true);
  const [isDirty, setIsDirty] = useState(false);

  const handleThresholdChange = (value: string) => {
    setThreshold(value);
    setIsDirty(true);
  };

  const handleEnabledChange = (value: boolean) => {
    setEnabled(value);
    setIsDirty(true);
  };

  const handleSave = () => {
    const thresholdNum = parseFloat(threshold);
    if (isNaN(thresholdNum) || thresholdNum <= 0 || thresholdNum > 100) {
      toast.error("Threshold must be between 0 and 100");
      return;
    }
    onUpdate(metricType, thresholdNum, enabled);
    setIsDirty(false);
  };

  return (
    <Card>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {icon}
            <CardTitle className="text-lg">{title}</CardTitle>
          </div>
          <Switch
            checked={enabled}
            onCheckedChange={handleEnabledChange}
            aria-label={`Enable ${title} alerts`}
          />
        </div>
        <CardDescription>
          {enabled
            ? `Alert when ${title.toLowerCase()} usage exceeds ${threshold}%`
            : `${title} alerts are disabled`}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          <Label htmlFor={`${metricType}-threshold`}>Threshold (%)</Label>
          <div className="flex items-center gap-2">
            <Input
              id={`${metricType}-threshold`}
              type="number"
              min="1"
              max="100"
              value={threshold}
              onChange={(e) => handleThresholdChange(e.target.value)}
              disabled={!enabled}
              className="w-24"
            />
            <span className="text-sm text-muted-foreground">%</span>
          </div>
        </div>
        {isDirty && (
          <Button
            onClick={handleSave}
            disabled={isUpdating}
            size="sm"
          >
            {isUpdating ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Saving...
              </>
            ) : (
              "Save Changes"
            )}
          </Button>
        )}
      </CardContent>
    </Card>
  );
}

export default function SettingsAlertDefaultsPage() {
  const queryClient = useQueryClient();

  const { data: defaults, isLoading: defaultsLoading } = useQuery<GlobalAlertDefaultsResponse>({
    queryKey: ["alert-defaults"],
    queryFn: () => api.getAlertDefaults(),
  });

  const { data: stats, isLoading: statsLoading } = useQuery<AlertStatsResponse>({
    queryKey: ["alert-stats"],
    queryFn: () => api.getAlertStats(),
  });

  const updateMutation = useMutation({
    mutationFn: async ({
      metricType,
      threshold,
      enabled,
    }: {
      metricType: "cpu" | "memory" | "disk";
      threshold: number;
      enabled: boolean;
    }) => {
      return api.updateAlertDefaults({
        [metricType]: {
          threshold_percent: threshold,
          enabled,
        },
      });
    },
    onSuccess: () => {
      toast.success("Alert defaults updated");
      queryClient.invalidateQueries({ queryKey: ["alert-defaults"] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to update alert defaults");
    },
  });

  const handleUpdate = (metricType: "cpu" | "memory" | "disk", threshold: number, enabled: boolean) => {
    updateMutation.mutate({ metricType, threshold, enabled });
  };

  const isLoading = defaultsLoading || statsLoading;

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Alert Defaults</h1>
        <p className="text-muted-foreground">
          Configure global alert thresholds for resource monitoring. Apps without custom alert configurations will use these defaults.
        </p>
      </div>

      {/* Stats Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Info className="h-5 w-5" />
            Alert Configuration Status
          </CardTitle>
          <CardDescription>
            Overview of how apps are configured for resource alerts
          </CardDescription>
        </CardHeader>
        <CardContent>
          {statsLoading ? (
            <div className="flex items-center justify-center py-4">
              <Loader2 className="h-6 w-6 animate-spin" />
            </div>
          ) : stats ? (
            <div className="grid gap-4 md:grid-cols-3">
              <div className="space-y-1">
                <div className="text-sm text-muted-foreground">Total Apps</div>
                <div className="text-2xl font-bold">{stats.total_apps}</div>
              </div>
              <div className="space-y-1">
                <div className="text-sm text-muted-foreground">Using Defaults</div>
                <div className="flex items-center gap-2">
                  <span className="text-2xl font-bold">{stats.apps_using_defaults}</span>
                  <Badge variant="secondary">
                    {stats.total_apps > 0
                      ? Math.round((stats.apps_using_defaults / stats.total_apps) * 100)
                      : 0}%
                  </Badge>
                </div>
              </div>
              <div className="space-y-1">
                <div className="text-sm text-muted-foreground">Custom Configs</div>
                <div className="flex items-center gap-2">
                  <span className="text-2xl font-bold">{stats.apps_with_custom_configs}</span>
                  <Badge variant="outline">
                    {stats.total_apps > 0
                      ? Math.round((stats.apps_with_custom_configs / stats.total_apps) * 100)
                      : 0}%
                  </Badge>
                </div>
              </div>
            </div>
          ) : (
            <p className="text-muted-foreground">Unable to load statistics</p>
          )}
        </CardContent>
      </Card>

      {/* Threshold Cards */}
      {isLoading ? (
        <div className="flex items-center justify-center py-8">
          <Loader2 className="h-8 w-8 animate-spin" />
        </div>
      ) : (
        <div className="grid gap-6 md:grid-cols-3">
          <ThresholdCard
            title="CPU"
            icon={<Cpu className="h-5 w-5 text-blue-500" />}
            metricType="cpu"
            config={defaults?.cpu ?? null}
            onUpdate={handleUpdate}
            isUpdating={updateMutation.isPending}
          />
          <ThresholdCard
            title="Memory"
            icon={<MemoryStick className="h-5 w-5 text-green-500" />}
            metricType="memory"
            config={defaults?.memory ?? null}
            onUpdate={handleUpdate}
            isUpdating={updateMutation.isPending}
          />
          <ThresholdCard
            title="Disk"
            icon={<HardDrive className="h-5 w-5 text-orange-500" />}
            metricType="disk"
            config={defaults?.disk ?? null}
            onUpdate={handleUpdate}
            isUpdating={updateMutation.isPending}
          />
        </div>
      )}

      {/* Help Text */}
      <Card>
        <CardContent className="pt-6">
          <div className="flex items-start gap-4">
            <Info className="h-5 w-5 text-muted-foreground mt-0.5" />
            <div className="space-y-2">
              <h4 className="font-medium">How Alert Defaults Work</h4>
              <ul className="text-sm text-muted-foreground space-y-1 list-disc list-inside">
                <li>These thresholds apply to all apps that don't have custom alert configurations</li>
                <li>Apps with custom configs (set in App Settings &gt; Alerts) will use their own thresholds</li>
                <li>Alerts are triggered after resource usage exceeds the threshold for 2 consecutive checks</li>
                <li>Notifications are sent via configured team notification channels</li>
              </ul>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
