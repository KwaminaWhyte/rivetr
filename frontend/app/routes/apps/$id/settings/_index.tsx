import { useState } from "react";
import { useOutletContext } from "react-router";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { GitHubSourceCard } from "@/components/github-source-card";
import { Zap, Copy, Check } from "lucide-react";
import { api } from "@/lib/api";
import type { App, AppEnvironment, UpdateAppRequest } from "@/types/api";

function getWebhookProvider(gitUrl: string): string {
  if (gitUrl.includes("github.com")) return "github";
  if (gitUrl.includes("gitlab.com") || gitUrl.includes("gitlab.")) return "gitlab";
  if (gitUrl.includes("bitbucket.org")) return "bitbucket";
  if (gitUrl.includes("gitea.")) return "gitea";
  return "github";
}

function WebhookSetupCard({ app }: { app: App }) {
  const [copied, setCopied] = useState(false);

  if (!app.git_url) return null;

  const provider = getWebhookProvider(app.git_url);
  const webhookUrl = `${window.location.origin}/api/webhooks/${provider}`;

  const handleCopy = () => {
    navigator.clipboard.writeText(webhookUrl);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const instructions: Record<string, { steps: string[]; label: string }> = {
    github: {
      label: "GitHub",
      steps: [
        "Go to your repository → Settings → Webhooks → Add webhook",
        'Set the Payload URL to the value above',
        'Set Content type to application/json',
        'Select "Just the push event"',
        "Click Add webhook",
      ],
    },
    gitlab: {
      label: "GitLab",
      steps: [
        "Go to your repository → Settings → Webhooks",
        "Paste the URL above as the URL",
        'Check "Push events"',
        "Click Add webhook",
      ],
    },
    bitbucket: {
      label: "Bitbucket",
      steps: [
        "Go to your repository → Repository settings → Webhooks → Add webhook",
        "Paste the URL above",
        'Check "Repository: Push"',
        "Click Save",
      ],
    },
    gitea: {
      label: "Gitea",
      steps: [
        "Go to your repository → Settings → Webhooks → Add Webhook → Gitea",
        "Paste the URL above as the Target URL",
        'Set Content type to application/json',
        'Select "Push events"',
        "Click Add Webhook",
      ],
    },
  };

  const { label, steps } = instructions[provider] ?? instructions.github;

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Zap className="h-5 w-5" />
          Auto-Deploy Setup
        </CardTitle>
        <CardDescription>
          Add this webhook URL to your {label} repository to enable automatic deployments on every push.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          <Label>Webhook URL</Label>
          <div className="flex gap-2">
            <Input value={webhookUrl} readOnly className="font-mono text-sm bg-muted" />
            <Button variant="outline" size="icon" onClick={handleCopy} title="Copy webhook URL">
              {copied ? <Check className="h-4 w-4 text-green-600" /> : <Copy className="h-4 w-4" />}
            </Button>
          </div>
        </div>
        <div className="rounded-md border bg-muted/40 p-4 text-sm space-y-2">
          <p className="font-medium">{label} setup:</p>
          <ol className="list-decimal list-inside space-y-1 text-muted-foreground">
            {steps.map((step, i) => (
              <li key={i}>{step}</li>
            ))}
          </ol>
        </div>
      </CardContent>
    </Card>
  );
}

const ENVIRONMENT_OPTIONS: { value: AppEnvironment; label: string }[] = [
  { value: "development", label: "Development" },
  { value: "staging", label: "Staging" },
  { value: "production", label: "Production" },
];

const RESTART_POLICY_OPTIONS: { value: string; label: string }[] = [
  { value: "unless-stopped", label: "Unless Stopped (recommended)" },
  { value: "always", label: "Always" },
  { value: "on-failure", label: "On Failure" },
  { value: "never", label: "Never" },
];

export default function AppSettingsGeneral() {
  const { app } = useOutletContext<{ app: App }>();
  const queryClient = useQueryClient();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [generalForm, setGeneralForm] = useState({
    name: app.name,
    git_url: app.git_url,
    branch: app.branch,
    port: app.port,
    environment: app.environment || "development",
    healthcheck: app.healthcheck || "",
    restart_policy: app.restart_policy || "unless-stopped",
  });

  const handleGeneralSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    try {
      const updates: UpdateAppRequest = {
        name: generalForm.name,
        git_url: generalForm.git_url,
        branch: generalForm.branch,
        port: generalForm.port,
        environment: generalForm.environment as AppEnvironment,
        healthcheck: generalForm.healthcheck,
        restart_policy: generalForm.restart_policy,
      };
      await api.updateApp(app.id, updates);
      toast.success("Settings saved");
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Update failed");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="space-y-6">
      <GitHubSourceCard app={app} />
      <WebhookSetupCard app={app} />
      <Card>
        <CardHeader>
          <CardTitle>General Settings</CardTitle>
          <CardDescription>
            Basic application configuration. Changes will take effect on the next deployment.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleGeneralSubmit} className="space-y-6">
            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="name">Name</Label>
                <Input
                  id="name"
                  value={generalForm.name}
                  onChange={(e) => setGeneralForm({ ...generalForm, name: e.target.value })}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="git_url">Git URL</Label>
                <Input
                  id="git_url"
                  value={generalForm.git_url}
                  onChange={(e) => setGeneralForm({ ...generalForm, git_url: e.target.value })}
                />
              </div>
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="branch">Branch</Label>
                <Input
                  id="branch"
                  value={generalForm.branch}
                  onChange={(e) => setGeneralForm({ ...generalForm, branch: e.target.value })}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="port">Port</Label>
                <Input
                  id="port"
                  type="number"
                  value={generalForm.port}
                  onChange={(e) => setGeneralForm({ ...generalForm, port: parseInt(e.target.value) || 0 })}
                />
              </div>
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="environment">Environment</Label>
                <Select
                  value={generalForm.environment}
                  onValueChange={(value) => setGeneralForm({ ...generalForm, environment: value as AppEnvironment })}
                >
                  <SelectTrigger>
                    <SelectValue placeholder="Select environment" />
                  </SelectTrigger>
                  <SelectContent>
                    {ENVIRONMENT_OPTIONS.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {option.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label htmlFor="healthcheck">Healthcheck Path</Label>
                <Input
                  id="healthcheck"
                  placeholder="/health"
                  value={generalForm.healthcheck}
                  onChange={(e) => setGeneralForm({ ...generalForm, healthcheck: e.target.value })}
                />
                <p className="text-xs text-muted-foreground">
                  Endpoint to check if the app is running
                </p>
              </div>
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="restart_policy">Restart Policy</Label>
                <Select
                  value={generalForm.restart_policy}
                  onValueChange={(value) => setGeneralForm({ ...generalForm, restart_policy: value })}
                >
                  <SelectTrigger>
                    <SelectValue placeholder="Select restart policy" />
                  </SelectTrigger>
                  <SelectContent>
                    {RESTART_POLICY_OPTIONS.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {option.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  Controls when Docker restarts your container
                </p>
              </div>
            </div>

            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? "Saving..." : "Save Changes"}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
