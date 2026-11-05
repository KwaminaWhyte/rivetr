import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { CheckCircle, Clock, Download, ExternalLink, Play, RefreshCw } from "lucide-react";

import { QueryError } from "@/components/query-error";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { FormGrid, SettingsSection, UnsavedChangesBar } from "@/components/settings-section";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { api } from "@/lib/api";
import { instanceSettingsApi } from "@/lib/api/instance-settings";
import type { UpdateStatus } from "@/types/api";
import type { UpdateChannel, UpdateSettings } from "@/types/instance-settings";

export function meta() {
  return [
    { title: "Updates - Rivetr" },
    { name: "description", content: "Update channel, schedule and automatic updates" },
  ];
}

function formatDate(dateStr: string | null | undefined): string {
  if (!dateStr) return "Never";
  return new Date(dateStr).toLocaleString();
}

const CRON_HELP = "Cron in UTC: minute hour day month weekday.";

export default function UpdatesPage() {
  const queryClient = useQueryClient();
  const [isDownloading, setIsDownloading] = useState(false);
  const [applyOpen, setApplyOpen] = useState(false);
  const [isApplying, setIsApplying] = useState(false);

  const {
    data: status,
    isLoading: statusLoading,
    error: statusError,
    refetch,
  } = useQuery<UpdateStatus | null>({
    queryKey: ["update-status"],
    queryFn: () => api.getVersionInfo(),
    refetchInterval: 60000,
  });

  const { data: settings, isLoading: settingsLoading } = useQuery({
    queryKey: ["update-settings"],
    queryFn: () => instanceSettingsApi.getUpdateSettings(),
  });

  const [form, setForm] = useState<UpdateSettings | null>(null);
  useEffect(() => {
    if (settings) setForm(settings);
  }, [settings]);
  const dirty = !!form && !!settings && JSON.stringify(form) !== JSON.stringify(settings);

  const check = useMutation({
    mutationFn: () => api.checkForUpdate(),
    onSuccess: (data) => {
      queryClient.setQueryData(["update-status"], data);
      if (data?.last_error) toast.error(`Update check failed: ${data.last_error}`);
      else if (data?.update_available) toast.success(`Update available: ${data.latest_version}`);
      else toast.info("Rivetr is up to date");
    },
    onError: () => toast.error("Failed to check for updates"),
  });

  const save = useMutation({
    mutationFn: (s: UpdateSettings) => instanceSettingsApi.updateUpdateSettings(s),
    onSuccess: (saved) => {
      queryClient.setQueryData(["update-settings"], saved);
      queryClient.invalidateQueries({ queryKey: ["update-status"] });
      toast.success("Update settings saved");
    },
    onError: (e) => toast.error(e instanceof Error ? e.message : "Failed to save settings"),
  });

  const set = <K extends keyof UpdateSettings>(key: K, value: UpdateSettings[K]) =>
    setForm((f) => (f ? { ...f, [key]: value } : f));

  const handleDownload = async () => {
    setIsDownloading(true);
    try {
      const result = await api.downloadUpdate();
      toast.success(`Downloaded update ${result.version}`);
      refetch();
    } catch {
      toast.error("Failed to download update");
    } finally {
      setIsDownloading(false);
    }
  };

  const handleApply = async () => {
    setIsApplying(true);
    try {
      await api.applyUpdate();
      toast.success("Update applied. Restart the Rivetr service to finish.");
      setApplyOpen(false);
    } catch {
      toast.error("Failed to apply update");
    } finally {
      setIsApplying(false);
    }
  };

  const headerStatus = status?.update_available
    ? `${status.latest_version} is available.`
    : "Rivetr is up to date.";

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Updates</h1>
        <p className="text-sm text-muted-foreground">
          Release channel and schedules. Changes apply right away, no restart needed.
        </p>
      </div>

      <SettingsSection
        title="Version"
        description={statusLoading ? "Checking..." : headerStatus}
        actions={
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => check.mutate()}
            disabled={check.isPending}
          >
            <RefreshCw className={`mr-2 h-4 w-4 ${check.isPending ? "animate-spin" : ""}`} />
            Check Now
          </Button>
        }
      >
        {statusError ? (
          <QueryError what="update status" error={statusError} onRetry={() => refetch()} />
        ) : statusLoading ? (
          <Skeleton className="h-16 w-full" />
        ) : (
          <div className="space-y-4">
            <div className="grid gap-4 sm:grid-cols-3">
              <div>
                <div className="text-sm text-muted-foreground">Running version</div>
                <div className="font-mono text-lg font-semibold">
                  {status?.current_version || "Unknown"}
                </div>
              </div>
              <div>
                <div className="text-sm text-muted-foreground">Latest version</div>
                <div className="flex items-center gap-2 font-mono text-lg font-semibold">
                  {status?.latest_version || status?.current_version || "Unknown"}
                  {!status?.update_available && <CheckCircle className="h-4 w-4 text-green-500" />}
                </div>
              </div>
              <div>
                <div className="text-sm text-muted-foreground">Last checked</div>
                <div className="flex items-center gap-1 text-sm">
                  <Clock className="h-4 w-4 text-muted-foreground" />
                  {formatDate(status?.last_checked)}
                </div>
              </div>
            </div>

            {status?.last_error && (
              <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">
                Last check error: {status.last_error}
              </div>
            )}

            {status?.update_available && (
              <div className="space-y-3 rounded-md border p-3">
                {status.release_notes && (
                  <div className="max-h-48 overflow-y-auto whitespace-pre-wrap text-sm text-muted-foreground">
                    {status.release_notes}
                  </div>
                )}
                <div className="flex flex-wrap gap-2">
                  {status.download_url && (
                    <Button variant="outline" size="sm" onClick={handleDownload} disabled={isDownloading}>
                      <Download className="mr-2 h-4 w-4" />
                      {isDownloading ? "Downloading..." : "Download"}
                    </Button>
                  )}
                  <Button size="sm" onClick={() => setApplyOpen(true)}>
                    <Play className="mr-2 h-4 w-4" />
                    Apply update
                  </Button>
                  {status.release_url && (
                    <Button variant="ghost" size="sm" asChild>
                      <a href={status.release_url} target="_blank" rel="noopener noreferrer">
                        <ExternalLink className="mr-2 h-4 w-4" />
                        Release notes
                      </a>
                    </Button>
                  )}
                </div>
              </div>
            )}
          </div>
        )}
      </SettingsSection>

      {settingsLoading || !form ? (
        <Skeleton className="h-48 w-full" />
      ) : (
        <form
          className="space-y-8"
          onSubmit={(e) => {
            e.preventDefault();
            if (dirty) save.mutate(form);
          }}
        >
          <SettingsSection title="Update checks">
            <FormGrid>
              <div className="space-y-2">
                <Label htmlFor="channel">Channel</Label>
                <Select value={form.channel} onValueChange={(v) => set("channel", v as UpdateChannel)}>
                  <SelectTrigger id="channel" className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="stable">Stable</SelectItem>
                    <SelectItem value="beta">Beta (includes pre-releases)</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label htmlFor="check-enabled">Update checks</Label>
                <Select
                  value={form.check_enabled ? "on" : "off"}
                  onValueChange={(v) => set("check_enabled", v === "on")}
                >
                  <SelectTrigger id="check-enabled" className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="on">Enabled</SelectItem>
                    <SelectItem value="off">Disabled</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label htmlFor="check-cron">Check frequency</Label>
                <Input
                  id="check-cron"
                  className="font-mono"
                  value={form.check_cron}
                  onChange={(e) => set("check_cron", e.target.value)}
                  placeholder="0 */6 * * *"
                />
                <p className="text-xs text-muted-foreground">{CRON_HELP}</p>
              </div>
            </FormGrid>
          </SettingsSection>

          <SettingsSection
            title="Automatic updates"
            description="Downloads and installs the latest release on schedule. Restart the service afterwards to run it."
          >
            <FormGrid>
              <div className="space-y-2">
                <Label htmlFor="auto-update">Auto update</Label>
                <Select
                  value={form.auto_update_enabled ? "on" : "off"}
                  onValueChange={(v) => set("auto_update_enabled", v === "on")}
                >
                  <SelectTrigger id="auto-update" className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="on">Enabled</SelectItem>
                    <SelectItem value="off">Disabled</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label htmlFor="auto-update-cron">Auto update frequency</Label>
                <Input
                  id="auto-update-cron"
                  className="font-mono"
                  value={form.auto_update_cron}
                  onChange={(e) => set("auto_update_cron", e.target.value)}
                  placeholder="0 0 * * *"
                  disabled={!form.auto_update_enabled}
                />
                <p className="text-xs text-muted-foreground">{CRON_HELP}</p>
              </div>
            </FormGrid>
          </SettingsSection>
          <button type="submit" hidden aria-hidden tabIndex={-1} />
        </form>
      )}

      <UnsavedChangesBar
        visible={dirty}
        saving={save.isPending}
        onReset={() => settings && setForm(settings)}
        onSave={() => form && save.mutate(form)}
      />

      <ConfirmDialog
        open={applyOpen}
        onOpenChange={setApplyOpen}
        title={`Apply ${status?.latest_version ?? "update"}?`}
        description="The Rivetr binary is replaced. Restart the service to run the new version; there is a short downtime."
        resourceName={status?.latest_version ?? undefined}
        confirmLabel="Apply update"
        isPending={isApplying}
        onConfirm={handleApply}
      />
    </div>
  );
}
