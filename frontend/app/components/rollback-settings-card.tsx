import { useState, useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { RotateCcw, History, AlertTriangle, Info } from "lucide-react";
import { api } from "@/lib/api";
import type { App, UpdateAppRequest } from "@/types/api";

interface RollbackSettingsCardProps {
  app: App;
}

const MAX_VERSIONS_OPTIONS = [
  { value: 3, label: "3 versions" },
  { value: 5, label: "5 versions" },
  { value: 10, label: "10 versions" },
  { value: 15, label: "15 versions" },
  { value: 20, label: "20 versions" },
];

export function RollbackSettingsCard({ app }: RollbackSettingsCardProps) {
  const queryClient = useQueryClient();
  // Convert to boolean in case API returns 0/1 integers from SQLite
  const [autoRollbackEnabled, setAutoRollbackEnabled] = useState(Boolean(app.auto_rollback_enabled));
  const [registryPushEnabled, setRegistryPushEnabled] = useState(Boolean(app.registry_push_enabled));
  const [maxVersions, setMaxVersions] = useState(app.max_rollback_versions || 5);
  const [isSaving, setIsSaving] = useState(false);
  const [isDirty, setIsDirty] = useState(false);

  // Sync state when app changes
  useEffect(() => {
    setAutoRollbackEnabled(Boolean(app.auto_rollback_enabled));
    setRegistryPushEnabled(Boolean(app.registry_push_enabled));
    setMaxVersions(app.max_rollback_versions || 5);
    setIsDirty(false);
  }, [app.auto_rollback_enabled, app.registry_push_enabled, app.max_rollback_versions]);

  const handleSave = async () => {
    setIsSaving(true);
    try {
      const updates: UpdateAppRequest = {
        auto_rollback_enabled: autoRollbackEnabled,
        registry_push_enabled: registryPushEnabled,
        max_rollback_versions: maxVersions,
      };
      await api.updateApp(app.id, updates);
      toast.success("Rollback settings saved");
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
      setIsDirty(false);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to save settings");
    } finally {
      setIsSaving(false);
    }
  };

  const hasRegistryConfigured = !!app.registry_url;

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <RotateCcw className="h-5 w-5" />
          Rollback Settings
        </CardTitle>
        <CardDescription>
          Configure automatic rollback behavior when deployments fail health checks.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Auto Rollback Toggle */}
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <Label htmlFor="auto-rollback" className="text-base">
              Automatic Rollback
            </Label>
            <p className="text-sm text-muted-foreground">
              Automatically rollback to the previous version when health checks fail
            </p>
          </div>
          <Switch
            id="auto-rollback"
            checked={autoRollbackEnabled}
            onCheckedChange={(checked) => {
              setAutoRollbackEnabled(checked);
              setIsDirty(true);
            }}
            disabled={isSaving}
          />
        </div>

        {autoRollbackEnabled && (
          <div className="flex items-start gap-3 p-3 rounded-lg bg-green-50 dark:bg-green-950/30 border border-green-200 dark:border-green-900">
            <Info className="h-5 w-5 text-green-600 dark:text-green-500 shrink-0 mt-0.5" />
            <div className="text-sm text-green-800 dark:text-green-200">
              <p className="font-medium">How it works</p>
              <p className="mt-1 text-green-700 dark:text-green-300">
                If a deployment fails health checks after 10 attempts, the system will
                automatically revert to the last successful deployment. This ensures
                minimal downtime for your application.
              </p>
            </div>
          </div>
        )}

        {/* Registry Push Toggle */}
        <div className="flex items-center justify-between pt-4 border-t">
          <div className="space-y-0.5">
            <Label htmlFor="registry-push" className="text-base flex items-center gap-2">
              <History className="h-4 w-4" />
              Push Images to Registry
            </Label>
            <p className="text-sm text-muted-foreground">
              Store built images in a Docker registry for reliable rollbacks
            </p>
          </div>
          <Switch
            id="registry-push"
            checked={registryPushEnabled}
            onCheckedChange={(checked) => {
              setRegistryPushEnabled(checked);
              setIsDirty(true);
            }}
            disabled={isSaving || !hasRegistryConfigured}
          />
        </div>

        {!hasRegistryConfigured && (
          <div className="flex items-start gap-3 p-3 rounded-lg bg-amber-50 dark:bg-amber-950/30 border border-amber-200 dark:border-amber-900">
            <AlertTriangle className="h-5 w-5 text-amber-600 dark:text-amber-500 shrink-0 mt-0.5" />
            <div className="text-sm text-amber-800 dark:text-amber-200">
              <p className="font-medium">Registry not configured</p>
              <p className="mt-1 text-amber-700 dark:text-amber-300">
                To enable registry push, configure a Docker registry in the Build tab under
                "Docker Registry" settings.
              </p>
            </div>
          </div>
        )}

        {registryPushEnabled && hasRegistryConfigured && (
          <>
            <div className="flex items-start gap-3 p-3 rounded-lg bg-blue-50 dark:bg-blue-950/30 border border-blue-200 dark:border-blue-900">
              <Info className="h-5 w-5 text-blue-600 dark:text-blue-500 shrink-0 mt-0.5" />
              <div className="text-sm text-blue-800 dark:text-blue-200">
                <p className="font-medium">Registry storage enabled</p>
                <p className="mt-1 text-blue-700 dark:text-blue-300">
                  After each successful deployment, the built image will be pushed to
                  your configured registry. This allows rollback even after local images
                  are cleaned up.
                </p>
              </div>
            </div>

            {/* Max Versions Select */}
            <div className="space-y-2 pt-4 border-t">
              <Label htmlFor="max-versions">Maximum Rollback Versions</Label>
              <Select
                value={maxVersions.toString()}
                onValueChange={(value) => {
                  setMaxVersions(parseInt(value));
                  setIsDirty(true);
                }}
              >
                <SelectTrigger className="w-48">
                  <SelectValue placeholder="Select versions" />
                </SelectTrigger>
                <SelectContent>
                  {MAX_VERSIONS_OPTIONS.map((option) => (
                    <SelectItem key={option.value} value={option.value.toString()}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                Number of deployment versions to keep in the registry for rollback
              </p>
            </div>
          </>
        )}

        {/* Save Button */}
        <Button
          onClick={handleSave}
          disabled={isSaving || !isDirty}
          className="w-full sm:w-auto"
        >
          {isSaving ? "Saving..." : "Save Changes"}
        </Button>
      </CardContent>
    </Card>
  );
}
