import { useState, useEffect } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";
import {
  Globe,
  Info,
  Server,
  CheckCircle,
  XCircle,
  Database,
  Container,
  Loader2,
} from "lucide-react";
import type { SystemHealthStatus, UpdateStatus } from "@/types/api";

export function meta() {
  return [
    { title: "Settings - Rivetr" },
    { name: "description", content: "Configure your Rivetr instance settings" },
  ];
}

function HealthIcon({ healthy }: { healthy: boolean }) {
  return healthy ? (
    <CheckCircle className="h-4 w-4 text-green-500" />
  ) : (
    <XCircle className="h-4 w-4 text-red-500" />
  );
}

export default function SettingsPage() {
  const queryClient = useQueryClient();

  const { data: health, isLoading: healthLoading } = useQuery<SystemHealthStatus>({
    queryKey: ["system-health"],
    queryFn: () => api.getSystemHealth(),
    refetchInterval: 30000,
  });

  const { data: versionInfo, isLoading: versionLoading } = useQuery<UpdateStatus>({
    queryKey: ["version-info"],
    queryFn: () => api.getVersionInfo(),
    refetchInterval: 60000,
  });

  // Instance settings
  const { data: instanceSettings, isLoading: instanceLoading } = useQuery({
    queryKey: ["instance-settings"],
    queryFn: () => api.getInstanceSettings(),
  });

  const [instanceDomain, setInstanceDomain] = useState("");
  const [instanceName, setInstanceName] = useState("");

  // Sync state when data loads
  useEffect(() => {
    if (instanceSettings) {
      setInstanceDomain(instanceSettings.instance_domain ?? "");
      setInstanceName(instanceSettings.instance_name ?? "");
    }
  }, [instanceSettings]);

  const updateInstanceMutation = useMutation({
    mutationFn: () =>
      api.updateInstanceSettings({
        instance_domain: instanceDomain.trim() || null,
        instance_name: instanceName.trim() || null,
      }),
    onSuccess: () => {
      toast.success("Instance settings saved");
      queryClient.invalidateQueries({ queryKey: ["instance-settings"] });
    },
    onError: (error) => {
      toast.error(
        error instanceof Error ? error.message : "Failed to save instance settings"
      );
    },
  });

  const isLoading = healthLoading || versionLoading;

  return (
    <div className="space-y-6">
      <h1 className="text-3xl font-bold">Settings</h1>

      {/* Instance Configuration */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Globe className="h-5 w-5" />
            Instance
          </CardTitle>
          <CardDescription>
            Configure the domain and display name for this Rivetr instance.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {instanceLoading ? (
            <div className="flex items-center gap-2 text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              <span>Loading…</span>
            </div>
          ) : (
            <>
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="instance-domain">Instance Domain</Label>
                  <Input
                    id="instance-domain"
                    placeholder="rivetr.example.com"
                    value={instanceDomain}
                    onChange={(e) => setInstanceDomain(e.target.value)}
                  />
                  <p className="text-xs text-muted-foreground">
                    The domain where this Rivetr dashboard is accessible. Used for generating links and callbacks.
                  </p>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="instance-name">Instance Name</Label>
                  <Input
                    id="instance-name"
                    placeholder="My Rivetr"
                    value={instanceName}
                    onChange={(e) => setInstanceName(e.target.value)}
                  />
                  <p className="text-xs text-muted-foreground">
                    A friendly name shown in the dashboard header and notification messages.
                  </p>
                </div>
              </div>
              <div className="flex justify-end">
                <Button
                  onClick={() => updateInstanceMutation.mutate()}
                  disabled={updateInstanceMutation.isPending}
                >
                  {updateInstanceMutation.isPending ? (
                    <>
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      Saving…
                    </>
                  ) : (
                    "Save Changes"
                  )}
                </Button>
              </div>
            </>
          )}
        </CardContent>
      </Card>

      {/* Server Information */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Server className="h-5 w-5" />
            Server Information
          </CardTitle>
          <CardDescription>
            Current status and version of your Rivetr instance.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4 md:grid-cols-3">
            <div className="space-y-1">
              <p className="text-xs text-muted-foreground font-medium uppercase tracking-wide">Version</p>
              <p className="font-semibold font-mono">
                {isLoading ? "..." : (versionInfo?.current_version ?? health?.version ?? "Unknown")}
              </p>
              {versionInfo?.update_available && versionInfo.latest_version && (
                <Badge variant="outline" className="text-xs text-amber-600 border-amber-500">
                  Update available: {versionInfo.latest_version}
                </Badge>
              )}
            </div>
            <div className="space-y-1">
              <p className="text-xs text-muted-foreground font-medium uppercase tracking-wide">Overall Health</p>
              <div className="flex items-center gap-2">
                {isLoading ? (
                  <span className="text-muted-foreground">...</span>
                ) : (
                  <>
                    <HealthIcon healthy={health?.healthy ?? false} />
                    <span className="font-medium">
                      {health?.healthy ? "Healthy" : "Degraded"}
                    </span>
                  </>
                )}
              </div>
            </div>
            <div className="space-y-1">
              <p className="text-xs text-muted-foreground font-medium uppercase tracking-wide">Auto-Update</p>
              <p className="font-medium">
                {isLoading
                  ? "..."
                  : versionInfo?.auto_update_enabled
                  ? "Enabled"
                  : "Disabled"}
              </p>
            </div>
          </div>

          {health && !isLoading && (
            <>
              <Separator />
              <div className="space-y-2">
                <p className="text-sm font-medium text-muted-foreground">Component Status</p>
                <div className="grid gap-3 md:grid-cols-3">
                  <div className="flex items-center gap-2 text-sm">
                    <Database className="h-4 w-4 text-muted-foreground" />
                    <span>Database</span>
                    <HealthIcon healthy={health.database_healthy} />
                  </div>
                  <div className="flex items-center gap-2 text-sm">
                    <Container className="h-4 w-4 text-muted-foreground" />
                    <span>Container Runtime</span>
                    <HealthIcon healthy={health.runtime_healthy} />
                  </div>
                  <div className="flex items-center gap-2 text-sm">
                    <Server className="h-4 w-4 text-muted-foreground" />
                    <span>Disk Space</span>
                    <HealthIcon healthy={health.disk_healthy} />
                  </div>
                </div>
              </div>
            </>
          )}
        </CardContent>
      </Card>

      {/* Instance Domain Configuration */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Globe className="h-5 w-5" />
            Domain & Proxy Configuration
          </CardTitle>
          <CardDescription>
            Configure the instance domain, base domain for app subdomains, and TLS settings.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-start gap-2 rounded-md bg-muted p-4 text-sm">
            <Info className="h-4 w-4 mt-0.5 shrink-0 text-muted-foreground" />
            <div className="space-y-3">
              <p className="font-medium">How to configure domain settings</p>
              <p className="text-muted-foreground">
                These settings live in <code className="bg-background px-1 rounded font-mono">rivetr.toml</code> under the <code className="bg-background px-1 rounded font-mono">[proxy]</code> section.
                Edit the file and restart Rivetr to apply changes.
              </p>
              <pre className="bg-background rounded p-3 font-mono text-xs overflow-x-auto leading-relaxed">
{`[proxy]
# Dashboard domain — the proxy forwards traffic here to Rivetr's API
instance_domain = "rivetr.yourdomain.com"

# Base domain for auto-generated app subdomains
# e.g., app "myapp" becomes "myapp.apps.yourdomain.com"
base_domain = "apps.yourdomain.com"

# ACME / Let's Encrypt email for automatic TLS certificates
acme_email = "you@yourdomain.com"`}
              </pre>
              <div className="space-y-1.5 text-muted-foreground">
                <p>
                  <strong className="text-foreground">instance_domain</strong> — the domain where the Rivetr dashboard is accessible.
                  Point an A record for this domain to your server IP and the proxy will handle TLS automatically.
                </p>
                <p>
                  <strong className="text-foreground">base_domain</strong> — when set, new apps automatically receive a subdomain
                  like <code className="bg-background px-1 rounded font-mono">myapp.apps.yourdomain.com</code>. Requires a wildcard DNS record
                  (<code className="bg-background px-1 rounded font-mono">*.apps.yourdomain.com → server IP</code>).
                </p>
                <p>
                  <strong className="text-foreground">acme_email</strong> — email used by Let's Encrypt for certificate expiry notices.
                  Required to enable automatic TLS for <code className="bg-background px-1 rounded font-mono">instance_domain</code>.
                </p>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Container Runtime */}
      <Card>
        <CardHeader>
          <CardTitle>Container Runtime</CardTitle>
          <CardDescription>
            Information about the active container runtime.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-1">
              <p className="text-xs text-muted-foreground font-medium uppercase tracking-wide">Runtime</p>
              <p className="font-medium">Docker / Podman (auto-detected)</p>
              <p className="text-xs text-muted-foreground">
                Set <code className="bg-muted px-1 rounded font-mono">runtime = "docker"</code> or{" "}
                <code className="bg-muted px-1 rounded font-mono">runtime = "podman"</code> in{" "}
                <code className="bg-muted px-1 rounded font-mono">rivetr.toml</code> to pin a specific runtime.
              </p>
            </div>
            <div className="space-y-1">
              <p className="text-xs text-muted-foreground font-medium uppercase tracking-wide">Status</p>
              {isLoading ? (
                <p className="text-muted-foreground">...</p>
              ) : (
                <div className="flex items-center gap-2">
                  <HealthIcon healthy={health?.runtime_healthy ?? false} />
                  <p className="font-medium">
                    {health?.runtime_healthy ? "Connected" : "Unavailable"}
                  </p>
                </div>
              )}
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
