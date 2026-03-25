import { useState, useEffect } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { api } from "@/lib/api";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Globe,
  Info,
  Server,
  CheckCircle,
  XCircle,
  Database,
  Container,
  Loader2,
  Trash2,
  Sparkles,
  Eye,
  EyeOff,
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
  const [instanceTimezone, setInstanceTimezone] = useState("");
  const [maxDeployments, setMaxDeployments] = useState(5);
  const [pruneImages, setPruneImages] = useState(true);

  // AI provider state
  const [aiProvider, setAiProvider] = useState("claude");
  const [aiApiKey, setAiApiKey] = useState("");
  const [aiModel, setAiModel] = useState("");
  const [aiMaxTokens, setAiMaxTokens] = useState("");
  const [showApiKey, setShowApiKey] = useState(false);

  // Sync state when data loads
  useEffect(() => {
    if (instanceSettings) {
      setInstanceDomain(instanceSettings.instance_domain ?? "");
      setInstanceName(instanceSettings.instance_name ?? "");
      setInstanceTimezone(instanceSettings.instance_timezone ?? "");
      setMaxDeployments(instanceSettings.max_deployments_per_app ?? 5);
      setPruneImages(instanceSettings.prune_images ?? true);
      setAiProvider(instanceSettings.ai_provider ?? "claude");
      setAiModel(instanceSettings.ai_model ?? "");
      setAiMaxTokens(instanceSettings.ai_max_tokens ? String(instanceSettings.ai_max_tokens) : "");
      // Never pre-fill the key; user must re-enter to change it
    }
  }, [instanceSettings]);

  const updateInstanceMutation = useMutation({
    mutationFn: () =>
      api.updateInstanceSettings({
        instance_domain: instanceDomain.trim() || null,
        instance_name: instanceName.trim() || null,
        instance_timezone: instanceTimezone.trim() || null,
      }),
    onSuccess: () => {
      toast.success("Settings saved and applied");
      queryClient.invalidateQueries({ queryKey: ["instance-settings"] });
    },
    onError: (error) => {
      toast.error(
        error instanceof Error ? error.message : "Failed to save instance settings"
      );
    },
  });

  const updateCleanupMutation = useMutation({
    mutationFn: () =>
      api.updateInstanceSettings({
        max_deployments_per_app: maxDeployments,
        prune_images: pruneImages,
      }),
    onSuccess: () => {
      toast.success("Cleanup settings saved");
      queryClient.invalidateQueries({ queryKey: ["instance-settings"] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to save cleanup settings");
    },
  });

  const updateAiMutation = useMutation({
    mutationFn: () =>
      api.updateInstanceSettings({
        ai_provider: aiProvider || null,
        ai_api_key: aiApiKey.trim() || null,
        ai_model: aiModel.trim() || null,
        ai_max_tokens: aiMaxTokens ? parseInt(aiMaxTokens) || null : null,
      }),
    onSuccess: () => {
      toast.success("AI provider settings saved");
      setAiApiKey("");
      queryClient.invalidateQueries({ queryKey: ["instance-settings"] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to save AI settings");
    },
  });

  const isLoading = healthLoading || versionLoading;

  // Docker cleanup
  const [cleanupOutput, setCleanupOutput] = useState<string | null>(null);
  const dockerCleanupMutation = useMutation({
    mutationFn: () => api.runDockerCleanup(),
    onSuccess: (data) => {
      setCleanupOutput(data.output || "No output returned.");
      if (data.success) {
        toast.success("Docker cleanup completed");
      } else {
        toast.error("Docker cleanup finished with errors");
      }
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Docker cleanup failed");
    },
  });

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
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="instance-timezone">Timezone</Label>
                  <Input
                    id="instance-timezone"
                    placeholder="UTC"
                    value={instanceTimezone}
                    onChange={(e) => setInstanceTimezone(e.target.value)}
                  />
                  <p className="text-xs text-muted-foreground">
                    IANA timezone for this Rivetr instance (e.g. America/New_York, Europe/London, Asia/Tokyo). Defaults to UTC.
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
                The <strong className="text-foreground">instance_domain</strong> can be changed live using the Instance form above —
                no restart needed. Other proxy settings such as <code className="bg-background px-1 rounded font-mono">base_domain</code> and{" "}
                <code className="bg-background px-1 rounded font-mono">acme_email</code> still live in{" "}
                <code className="bg-background px-1 rounded font-mono">rivetr.toml</code> and require a restart to apply.
              </p>
              <pre className="bg-background rounded p-3 font-mono text-xs overflow-x-auto leading-relaxed">
{`[proxy]
# Base domain for auto-generated app subdomains
# e.g., app "myapp" becomes "myapp.apps.yourdomain.com"
base_domain = "apps.yourdomain.com"

# ACME / Let's Encrypt email for automatic TLS certificates
acme_email = "you@yourdomain.com"`}
              </pre>
              <div className="space-y-1.5 text-muted-foreground">
                <p>
                  <strong className="text-foreground">base_domain</strong> — when set, new apps automatically receive a subdomain
                  like <code className="bg-background px-1 rounded font-mono">myapp.apps.yourdomain.com</code>. Requires a wildcard DNS record
                  (<code className="bg-background px-1 rounded font-mono">*.apps.yourdomain.com → server IP</code>).
                </p>
                <p>
                  <strong className="text-foreground">acme_email</strong> — email used by Let's Encrypt for certificate expiry notices.
                  Required to enable automatic TLS for the instance domain.
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

      {/* Deployment Cleanup */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Trash2 className="h-5 w-5" />
            Deployment Cleanup
          </CardTitle>
          <CardDescription>
            Control how many old deployments are kept per app. Older ones are automatically
            removed along with their Docker images to preserve disk space.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {instanceLoading ? (
            <div className="flex items-center gap-2 text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              <span>Loading…</span>
            </div>
          ) : (
            <>
              <div className="grid gap-6 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="max-deployments">Deployments to keep per app</Label>
                  <Input
                    id="max-deployments"
                    type="number"
                    min={1}
                    max={50}
                    value={maxDeployments}
                    onChange={(e) => setMaxDeployments(Math.max(1, parseInt(e.target.value) || 1))}
                    className="w-32"
                  />
                  <p className="text-xs text-muted-foreground">
                    Old stopped deployments beyond this count are deleted along with their images.
                    Rollback always uses the most recent kept deployment.
                  </p>
                </div>
                <div className="space-y-3">
                  <Label>Prune unused images</Label>
                  <div className="flex items-center gap-3">
                    <Switch
                      id="prune-images"
                      checked={pruneImages}
                      onCheckedChange={setPruneImages}
                    />
                    <Label htmlFor="prune-images" className="font-normal text-sm text-muted-foreground">
                      {pruneImages ? "Enabled — dangling images removed after each cycle" : "Disabled"}
                    </Label>
                  </div>
                </div>
              </div>
              <div className="flex justify-end">
                <Button
                  onClick={() => updateCleanupMutation.mutate()}
                  disabled={updateCleanupMutation.isPending}
                >
                  {updateCleanupMutation.isPending ? (
                    <>
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      Saving…
                    </>
                  ) : (
                    "Save"
                  )}
                </Button>
              </div>
            </>
          )}
        </CardContent>
      </Card>

      {/* AI Provider */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Sparkles className="h-5 w-5" />
            AI Provider
          </CardTitle>
          <CardDescription>
            Configure the AI provider used for security scans, log analysis, and deployment suggestions.
            The API key is stored securely in the database and never exposed via the API.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {instanceLoading ? (
            <div className="flex items-center gap-2 text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              <span>Loading…</span>
            </div>
          ) : (
            <>
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="ai-provider">Provider</Label>
                  <Select value={aiProvider} onValueChange={setAiProvider}>
                    <SelectTrigger id="ai-provider">
                      <SelectValue placeholder="Select provider" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="claude">Anthropic Claude</SelectItem>
                      <SelectItem value="openai">OpenAI</SelectItem>
                      <SelectItem value="gemini">Google Gemini</SelectItem>
                      <SelectItem value="moonshot">Moonshot</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="ai-api-key">
                    API Key
                    {instanceSettings?.ai_configured && (
                      <Badge variant="outline" className="ml-2 text-xs text-green-600 border-green-500">
                        Configured
                      </Badge>
                    )}
                  </Label>
                  <div className="relative">
                    <Input
                      id="ai-api-key"
                      type={showApiKey ? "text" : "password"}
                      placeholder={instanceSettings?.ai_configured ? "Leave blank to keep current key" : "sk-…"}
                      value={aiApiKey}
                      onChange={(e) => setAiApiKey(e.target.value)}
                      className="pr-10"
                    />
                    <button
                      type="button"
                      className="absolute inset-y-0 right-0 flex items-center pr-3 text-muted-foreground hover:text-foreground"
                      onClick={() => setShowApiKey((v) => !v)}
                      tabIndex={-1}
                    >
                      {showApiKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                    </button>
                  </div>
                </div>
              </div>
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="ai-model">Model Override <span className="text-muted-foreground font-normal">(optional)</span></Label>
                  <Input
                    id="ai-model"
                    placeholder={
                      aiProvider === "claude"
                        ? "e.g. claude-opus-4-6"
                        : aiProvider === "openai"
                        ? "e.g. gpt-4o"
                        : aiProvider === "gemini"
                        ? "e.g. gemini-1.5-pro"
                        : "model name"
                    }
                    value={aiModel}
                    onChange={(e) => setAiModel(e.target.value)}
                  />
                  <p className="text-xs text-muted-foreground">
                    Leave blank to use the provider's default model.
                  </p>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="ai-max-tokens">Max Tokens <span className="text-muted-foreground font-normal">(optional)</span></Label>
                  <Input
                    id="ai-max-tokens"
                    type="number"
                    min={256}
                    max={128000}
                    placeholder="e.g. 4096"
                    value={aiMaxTokens}
                    onChange={(e) => setAiMaxTokens(e.target.value)}
                    className="w-40"
                  />
                </div>
              </div>
              <div className="flex justify-end">
                <Button
                  onClick={() => updateAiMutation.mutate()}
                  disabled={updateAiMutation.isPending}
                >
                  {updateAiMutation.isPending ? (
                    <>
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      Saving…
                    </>
                  ) : (
                    "Save AI Settings"
                  )}
                </Button>
              </div>
            </>
          )}
        </CardContent>
      </Card>

      {/* Docker Cleanup */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Trash2 className="h-5 w-5" />
            Docker Cleanup
          </CardTitle>
          <CardDescription>
            Remove dangling (untagged) Docker images to reclaim disk space. Running containers and named images are not affected.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center gap-4">
            <Button
              variant="outline"
              onClick={() => {
                setCleanupOutput(null);
                dockerCleanupMutation.mutate();
              }}
              disabled={dockerCleanupMutation.isPending}
            >
              {dockerCleanupMutation.isPending ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Running…
                </>
              ) : (
                <>
                  <Trash2 className="mr-2 h-4 w-4" />
                  Run Cleanup
                </>
              )}
            </Button>
            <p className="text-xs text-muted-foreground">
              Runs <code className="bg-muted px-1 rounded font-mono">docker system prune --filter dangling=true -f</code>
            </p>
          </div>
          {cleanupOutput !== null && (
            <pre className="bg-muted rounded p-3 font-mono text-xs overflow-x-auto whitespace-pre-wrap">
              {cleanupOutput}
            </pre>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
