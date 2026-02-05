import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { api } from "@/lib/api";
import type { UpdateStatus } from "@/types/api";
import { RefreshCw, Download, Play, CheckCircle, AlertTriangle, Clock, ExternalLink } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

export function meta() {
  return [
    { title: "Auto Updates - Rivetr" },
    { name: "description", content: "Configure automatic updates for your Rivetr instance" },
  ];
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return "Never";
  const date = new Date(dateStr);
  return date.toLocaleString();
}

export default function AutoUpdatePage() {
  const queryClient = useQueryClient();
  const [isDownloading, setIsDownloading] = useState(false);
  const [isApplying, setIsApplying] = useState(false);

  const { data: updateStatus, isLoading, refetch, isRefetching } = useQuery<UpdateStatus | null>({
    queryKey: ["update-status"],
    queryFn: () => api.getVersionInfo(),
    refetchInterval: 60000, // Check every minute
  });

  const checkMutation = useMutation({
    mutationFn: () => api.checkForUpdate(),
    onSuccess: (data) => {
      queryClient.setQueryData(["update-status"], data);
      if (data?.update_available) {
        toast.success(`Update available: ${data.latest_version}`);
      } else {
        toast.info("You're running the latest version");
      }
    },
    onError: (error) => {
      toast.error("Failed to check for updates");
      console.error(error);
    },
  });

  const handleDownload = async () => {
    setIsDownloading(true);
    try {
      const result = await api.downloadUpdate();
      toast.success(`Downloaded update ${result.version}`);
      refetch();
    } catch (error) {
      toast.error("Failed to download update");
      console.error(error);
    } finally {
      setIsDownloading(false);
    }
  };

  const handleApply = async () => {
    if (!confirm("This will restart the Rivetr server. Continue?")) {
      return;
    }
    setIsApplying(true);
    try {
      await api.applyUpdate();
      toast.success("Update applied! Server is restarting...");
      // The server will restart, so the page will eventually reload
    } catch (error) {
      toast.error("Failed to apply update");
      console.error(error);
    } finally {
      setIsApplying(false);
    }
  };

  return (
    <div className="space-y-6">
      <h1 className="text-3xl font-bold">Auto Updates</h1>

      {/* Current Version Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center justify-between">
            <span>Current Version</span>
            {updateStatus?.update_available ? (
              <Badge variant="secondary" className="bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400">
                Update Available
              </Badge>
            ) : (
              <Badge variant="secondary" className="bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400">
                Up to Date
              </Badge>
            )}
          </CardTitle>
          <CardDescription>
            Information about your Rivetr installation
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="animate-pulse space-y-3">
              <div className="h-4 bg-muted rounded w-1/3"></div>
              <div className="h-4 bg-muted rounded w-1/2"></div>
            </div>
          ) : (
            <div className="space-y-4">
              <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
                <div>
                  <div className="text-sm text-muted-foreground">Running Version</div>
                  <div className="font-mono text-lg font-semibold">{updateStatus?.current_version || "Unknown"}</div>
                </div>
                <div>
                  <div className="text-sm text-muted-foreground">Latest Version</div>
                  <div className="font-mono text-lg font-semibold">
                    {updateStatus?.latest_version || updateStatus?.current_version || "Unknown"}
                  </div>
                </div>
                <div>
                  <div className="text-sm text-muted-foreground">Last Checked</div>
                  <div className="flex items-center gap-1">
                    <Clock className="h-4 w-4 text-muted-foreground" />
                    <span>{formatDate(updateStatus?.last_checked || null)}</span>
                  </div>
                </div>
                <div>
                  <div className="text-sm text-muted-foreground">Auto-Update</div>
                  <div className="flex items-center gap-1">
                    {updateStatus?.auto_update_enabled ? (
                      <>
                        <CheckCircle className="h-4 w-4 text-green-500" />
                        <span>Enabled</span>
                      </>
                    ) : (
                      <>
                        <AlertTriangle className="h-4 w-4 text-amber-500" />
                        <span>Disabled</span>
                      </>
                    )}
                  </div>
                </div>
              </div>

              {updateStatus?.last_error && (
                <div className="p-3 bg-destructive/10 text-destructive rounded-md text-sm">
                  Last check error: {updateStatus.last_error}
                </div>
              )}

              <div className="flex flex-wrap gap-2">
                <Button
                  variant="outline"
                  onClick={() => checkMutation.mutate()}
                  disabled={checkMutation.isPending || isRefetching}
                >
                  <RefreshCw className={`h-4 w-4 mr-2 ${(checkMutation.isPending || isRefetching) ? "animate-spin" : ""}`} />
                  Check for Updates
                </Button>

                {updateStatus?.update_available && updateStatus?.download_url && (
                  <Button
                    variant="outline"
                    onClick={handleDownload}
                    disabled={isDownloading}
                  >
                    <Download className={`h-4 w-4 mr-2 ${isDownloading ? "animate-pulse" : ""}`} />
                    {isDownloading ? "Downloading..." : "Download Update"}
                  </Button>
                )}

                {updateStatus?.release_url && (
                  <Button variant="outline" asChild>
                    <a href={updateStatus.release_url} target="_blank" rel="noopener noreferrer">
                      <ExternalLink className="h-4 w-4 mr-2" />
                      View Release Notes
                    </a>
                  </Button>
                )}
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Update Actions */}
      {updateStatus?.update_available && (
        <Card>
          <CardHeader>
            <CardTitle>Available Update</CardTitle>
            <CardDescription>
              Version {updateStatus.latest_version} is available for download
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {updateStatus.release_notes && (
              <div className="p-4 bg-muted rounded-md">
                <h4 className="font-medium mb-2">Release Notes</h4>
                <div className="text-sm text-muted-foreground whitespace-pre-wrap max-h-48 overflow-y-auto">
                  {updateStatus.release_notes}
                </div>
              </div>
            )}

            <div className="flex gap-2">
              <Button
                onClick={handleApply}
                disabled={isApplying}
              >
                <Play className={`h-4 w-4 mr-2 ${isApplying ? "animate-pulse" : ""}`} />
                {isApplying ? "Applying..." : "Apply Update & Restart"}
              </Button>
            </div>

            <p className="text-xs text-muted-foreground">
              Applying the update will restart the Rivetr server. There will be brief downtime.
            </p>
          </CardContent>
        </Card>
      )}

      {/* Configuration Info */}
      <Card>
        <CardHeader>
          <CardTitle>Update Configuration</CardTitle>
          <CardDescription>
            Auto-update settings are configured in rivetr.toml
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            <div className="grid gap-4 md:grid-cols-2">
              <div className="p-3 bg-muted rounded-md">
                <div className="text-sm font-medium">Auto-Update Check</div>
                <div className="text-sm text-muted-foreground">
                  {updateStatus?.auto_update_enabled ? "Enabled (checks every 6 hours)" : "Disabled"}
                </div>
              </div>
              <div className="p-3 bg-muted rounded-md">
                <div className="text-sm font-medium">Auto-Apply</div>
                <div className="text-sm text-muted-foreground">
                  {updateStatus?.auto_apply_enabled ? "Enabled (automatically applies updates)" : "Disabled (manual approval required)"}
                </div>
              </div>
            </div>

            <div className="p-4 bg-muted/50 rounded-md">
              <h4 className="font-medium mb-2">Configuration Example</h4>
              <pre className="text-xs bg-background p-3 rounded overflow-x-auto">
{`[auto_update]
# Enable automatic update checking (default: true)
enabled = true

# Check interval in hours (default: 6)
check_interval_hours = 6

# Automatically apply updates (default: false)
auto_apply = false`}
              </pre>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
