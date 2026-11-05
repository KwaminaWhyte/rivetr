/**
 * Editable app basics, split by where they belong: name and environment on
 * the General section, repository, branch and port on Source and build.
 * Each card saves only its own fields.
 */
import { useMemo, useState } from "react";
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
import { UnsavedChangesBar } from "@/components/unsaved-changes-bar";
import { useDirtyForm } from "@/hooks/use-dirty-form";
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

export function WebhookSetupCard({ app }: { app: App }) {
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

/** Name and environment. Shown on the app's General section. */
export function AppIdentitySettings({ app }: { app: App }) {
  const queryClient = useQueryClient();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const initialForm = useMemo(
    () => ({
      name: app.name,
      environment: (app.environment || "development") as AppEnvironment,
    }),
    [app.name, app.environment]
  );
  const form = useDirtyForm(initialForm);

  const save = async () => {
    setIsSubmitting(true);
    try {
      const updates: UpdateAppRequest = {
        name: form.values.name,
        environment: form.values.environment,
      };
      await api.updateApp(app.id, updates);
      form.markSaved();
      toast.success("Settings saved");
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Update failed");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Name and environment</CardTitle>
        <CardDescription>How this application is labelled across the dashboard.</CardDescription>
      </CardHeader>
      <CardContent>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            if (form.isDirty) void save();
          }}
          className="grid gap-4 md:grid-cols-2"
        >
          <div className="space-y-2">
            <Label htmlFor="name">Name</Label>
            <Input
              id="name"
              value={form.values.name}
              onChange={(e) => form.setField("name", e.target.value)}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="environment">Environment</Label>
            <Select
              value={form.values.environment}
              onValueChange={(value) => form.setField("environment", value as AppEnvironment)}
            >
              <SelectTrigger id="environment" aria-label="Environment">
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
        </form>
      </CardContent>
      <UnsavedChangesBar
        show={form.isDirty}
        onSave={() => void save()}
        onReset={form.reset}
        saving={isSubmitting}
      />
    </Card>
  );
}

/** Repository URL, branch and port. Shown on Source and build. */
export function AppSourceSettings({ app }: { app: App }) {
  const queryClient = useQueryClient();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const initialForm = useMemo(
    () => ({ git_url: app.git_url, branch: app.branch, port: app.port }),
    [app.git_url, app.branch, app.port]
  );
  const form = useDirtyForm(initialForm);

  const save = async () => {
    setIsSubmitting(true);
    try {
      const updates: UpdateAppRequest = {
        git_url: form.values.git_url,
        branch: form.values.branch,
        port: form.values.port,
      };
      // Setting a git URL on a non-registry app implies git-based deploys.
      // This converts an upload-created app into a git app so pushes deploy it.
      if (form.values.git_url.trim() && !app.docker_image && app.deployment_source !== "git") {
        updates.deployment_source = "git";
      }
      await api.updateApp(app.id, updates);
      form.markSaved();
      toast.success("Source saved");
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Update failed");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Source</CardTitle>
        <CardDescription>
          Repository, branch and the port the app listens on. Changes take effect on the next
          deployment.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            if (form.isDirty) void save();
          }}
          className="space-y-4"
        >
          <div className="space-y-2">
            <Label htmlFor="git_url">Git URL</Label>
            <Input
              id="git_url"
              value={form.values.git_url}
              onChange={(e) => form.setField("git_url", e.target.value)}
            />
          </div>
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="branch">Branch</Label>
              <Input
                id="branch"
                value={form.values.branch}
                onChange={(e) => form.setField("branch", e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="port">Port</Label>
              <Input
                id="port"
                type="number"
                value={form.values.port}
                onChange={(e) => form.setField("port", parseInt(e.target.value) || 0)}
              />
            </div>
          </div>
        </form>
      </CardContent>
      <UnsavedChangesBar
        show={form.isDirty}
        onSave={() => void save()}
        onReset={form.reset}
        saving={isSubmitting}
      />
    </Card>
  );
}
