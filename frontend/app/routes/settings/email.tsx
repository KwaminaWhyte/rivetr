import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Loader2, Send } from "lucide-react";

import { QueryError } from "@/components/query-error";
import { FormGrid, SettingsSection, UnsavedChangesBar } from "@/components/settings-section";
import { Badge } from "@/components/ui/badge";
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
import { instanceSettingsApi } from "@/lib/api/instance-settings";
import type {
  EmailProvider,
  EmailSettings,
  SmtpEncryption,
  UpdateEmailSettingsRequest,
} from "@/types/instance-settings";

export function meta() {
  return [
    { title: "Email Settings - Rivetr" },
    { name: "description", content: "Transactional email for invitations and password resets" },
  ];
}

interface FormState {
  enabled: boolean;
  provider: EmailProvider;
  from_name: string;
  from_address: string;
  smtp_host: string;
  smtp_port: string;
  smtp_encryption: SmtpEncryption;
  smtp_username: string;
  smtp_password: string;
  smtp_timeout_secs: string;
  smtp_ehlo_domain: string;
  resend_api_key: string;
  sendry_api_key: string;
}

function toForm(s: EmailSettings): FormState {
  return {
    enabled: s.enabled,
    provider: s.provider,
    from_name: s.from_name ?? "",
    from_address: s.from_address ?? "",
    smtp_host: s.smtp_host ?? "",
    smtp_port: s.smtp_port ? String(s.smtp_port) : "",
    smtp_encryption: s.smtp_encryption,
    smtp_username: s.smtp_username ?? "",
    // Secrets are never sent back; empty means "keep what is stored".
    smtp_password: "",
    smtp_timeout_secs: String(s.smtp_timeout_secs),
    smtp_ehlo_domain: s.smtp_ehlo_domain ?? "",
    resend_api_key: "",
    sendry_api_key: "",
  };
}

function toRequest(f: FormState): UpdateEmailSettingsRequest {
  const req: UpdateEmailSettingsRequest = {
    enabled: f.enabled,
    provider: f.provider,
    from_name: f.from_name,
    from_address: f.from_address,
    smtp_host: f.smtp_host,
    smtp_encryption: f.smtp_encryption,
    smtp_username: f.smtp_username,
    smtp_ehlo_domain: f.smtp_ehlo_domain,
  };
  const port = parseInt(f.smtp_port, 10);
  if (!Number.isNaN(port)) req.smtp_port = port;
  const timeout = parseInt(f.smtp_timeout_secs, 10);
  if (!Number.isNaN(timeout)) req.smtp_timeout_secs = timeout;
  if (f.smtp_password) req.smtp_password = f.smtp_password;
  if (f.resend_api_key) req.resend_api_key = f.resend_api_key;
  if (f.sendry_api_key) req.sendry_api_key = f.sendry_api_key;
  return req;
}

const sourceLabel: Record<string, string> = {
  database: "Using these settings",
  config: "Using rivetr.toml",
};

export default function EmailSettingsPage() {
  const queryClient = useQueryClient();
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ["email-settings"],
    queryFn: () => instanceSettingsApi.getEmailSettings(),
  });

  const initial = useMemo(() => (data ? toForm(data) : null), [data]);
  const [form, setForm] = useState<FormState | null>(null);
  const [testTo, setTestTo] = useState("");

  useEffect(() => {
    if (initial) setForm(initial);
  }, [initial]);

  const dirty = !!form && !!initial && JSON.stringify(form) !== JSON.stringify(initial);

  const save = useMutation({
    mutationFn: (f: FormState) => instanceSettingsApi.updateEmailSettings(toRequest(f)),
    onSuccess: (saved) => {
      queryClient.setQueryData(["email-settings"], saved);
      toast.success("Email settings saved");
    },
    onError: (e) => toast.error(e instanceof Error ? e.message : "Failed to save settings"),
  });

  const clearSecret = useMutation({
    mutationFn: (field: "smtp_password" | "resend_api_key" | "sendry_api_key") =>
      instanceSettingsApi.updateEmailSettings({ [field]: "" }),
    onSuccess: (saved) => {
      queryClient.setQueryData(["email-settings"], saved);
      toast.success("Secret removed");
    },
    onError: (e) => toast.error(e instanceof Error ? e.message : "Failed to remove secret"),
  });

  const test = useMutation({
    mutationFn: () => instanceSettingsApi.sendTestEmail(testTo.trim() || undefined),
    onSuccess: (r) => toast.success(`Test email sent to ${r.sent_to}`),
    onError: (e) => toast.error(e instanceof Error ? e.message : "Failed to send test email"),
  });

  const set = <K extends keyof FormState>(key: K, value: FormState[K]) =>
    setForm((f) => (f ? { ...f, [key]: value } : f));

  if (error) {
    return <QueryError what="email settings" error={error} onRetry={() => refetch()} />;
  }

  return (
    <div className="space-y-8">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div>
          <h1 className="text-2xl font-bold">Email</h1>
          <p className="text-sm text-muted-foreground">
            Transactional email for invitations and password resets. Overrides the [email]
            section of rivetr.toml while enabled.
          </p>
        </div>
        {data && (
          <Badge variant={data.active_source ? "secondary" : "outline"}>
            {data.active_source ? sourceLabel[data.active_source] : "Email not configured"}
          </Badge>
        )}
      </div>

      {isLoading || !form || !data ? (
        <div className="space-y-4">
          <Skeleton className="h-32 w-full" />
          <Skeleton className="h-48 w-full" />
        </div>
      ) : (
        <form
          className="space-y-8"
          onSubmit={(e) => {
            e.preventDefault();
            if (dirty) save.mutate(form);
          }}
        >
          <SettingsSection title="Sender">
            <FormGrid>
              <div className="space-y-2">
                <Label htmlFor="email-enabled">Status</Label>
                <Select
                  value={form.enabled ? "on" : "off"}
                  onValueChange={(v) => set("enabled", v === "on")}
                >
                  <SelectTrigger id="email-enabled" className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="on">Enabled</SelectItem>
                    <SelectItem value="off">
                      {data.config_file_configured ? "Disabled (use rivetr.toml)" : "Disabled"}
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label htmlFor="email-provider">Provider</Label>
                <Select
                  value={form.provider}
                  onValueChange={(v) => set("provider", v as EmailProvider)}
                >
                  <SelectTrigger id="email-provider" className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="smtp">SMTP</SelectItem>
                    <SelectItem value="resend">Resend</SelectItem>
                    <SelectItem value="sendry">Sendry</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label htmlFor="from-name">From name</Label>
                <Input
                  id="from-name"
                  value={form.from_name}
                  onChange={(e) => set("from_name", e.target.value)}
                  placeholder="Rivetr"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="from-address">From address</Label>
                <Input
                  id="from-address"
                  type="email"
                  value={form.from_address}
                  onChange={(e) => set("from_address", e.target.value)}
                  placeholder="noreply@example.com"
                />
              </div>
            </FormGrid>
          </SettingsSection>

          {form.provider === "smtp" ? (
            <SettingsSection
              title="SMTP"
              description="Empty fields fall back to rivetr.toml when it has a value."
            >
              <FormGrid>
                <div className="space-y-2">
                  <Label htmlFor="smtp-host">Host</Label>
                  <Input
                    id="smtp-host"
                    value={form.smtp_host}
                    onChange={(e) => set("smtp_host", e.target.value)}
                    placeholder="smtp.example.com"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="smtp-port">Port</Label>
                  <Input
                    id="smtp-port"
                    inputMode="numeric"
                    value={form.smtp_port}
                    onChange={(e) => set("smtp_port", e.target.value.replace(/\D/g, ""))}
                    placeholder="587"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="smtp-encryption">Encryption</Label>
                  <Select
                    value={form.smtp_encryption}
                    onValueChange={(v) => set("smtp_encryption", v as SmtpEncryption)}
                  >
                    <SelectTrigger id="smtp-encryption" className="w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="starttls">STARTTLS (port 587)</SelectItem>
                      <SelectItem value="tls">TLS (port 465)</SelectItem>
                      <SelectItem value="none">None</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="smtp-timeout">Timeout (seconds)</Label>
                  <Input
                    id="smtp-timeout"
                    inputMode="numeric"
                    value={form.smtp_timeout_secs}
                    onChange={(e) => set("smtp_timeout_secs", e.target.value.replace(/\D/g, ""))}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="smtp-username">Username</Label>
                  <Input
                    id="smtp-username"
                    autoComplete="off"
                    value={form.smtp_username}
                    onChange={(e) => set("smtp_username", e.target.value)}
                  />
                </div>
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <Label htmlFor="smtp-password">Password</Label>
                    {data.smtp_password_set && (
                      <button
                        type="button"
                        className="text-xs text-muted-foreground hover:text-foreground hover:underline"
                        onClick={() => clearSecret.mutate("smtp_password")}
                      >
                        Remove saved password
                      </button>
                    )}
                  </div>
                  <Input
                    id="smtp-password"
                    type="password"
                    autoComplete="new-password"
                    value={form.smtp_password}
                    onChange={(e) => set("smtp_password", e.target.value)}
                    placeholder={data.smtp_password_set ? "Saved. Type to replace" : ""}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="smtp-ehlo">EHLO domain</Label>
                  <Input
                    id="smtp-ehlo"
                    value={form.smtp_ehlo_domain}
                    onChange={(e) => set("smtp_ehlo_domain", e.target.value)}
                    placeholder="Optional, e.g. mail.example.com"
                  />
                </div>
              </FormGrid>
            </SettingsSection>
          ) : form.provider === "sendry" ? (
            <SettingsSection
              title="Sendry"
              description="Create a sending key at sendry.online. The from address domain must be verified there."
            >
              <FormGrid>
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <Label htmlFor="sendry-key">API key</Label>
                    {data.sendry_api_key_set && (
                      <button
                        type="button"
                        className="text-xs text-muted-foreground hover:text-foreground hover:underline"
                        onClick={() => clearSecret.mutate("sendry_api_key")}
                      >
                        Remove saved key
                      </button>
                    )}
                  </div>
                  <Input
                    id="sendry-key"
                    type="password"
                    autoComplete="off"
                    value={form.sendry_api_key}
                    onChange={(e) => set("sendry_api_key", e.target.value)}
                    placeholder={data.sendry_api_key_set ? "Saved. Type to replace" : "sn_live_..."}
                  />
                </div>
              </FormGrid>
            </SettingsSection>
          ) : (
            <SettingsSection title="Resend">
              <FormGrid>
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <Label htmlFor="resend-key">API key</Label>
                    {data.resend_api_key_set && (
                      <button
                        type="button"
                        className="text-xs text-muted-foreground hover:text-foreground hover:underline"
                        onClick={() => clearSecret.mutate("resend_api_key")}
                      >
                        Remove saved key
                      </button>
                    )}
                  </div>
                  <Input
                    id="resend-key"
                    type="password"
                    autoComplete="off"
                    value={form.resend_api_key}
                    onChange={(e) => set("resend_api_key", e.target.value)}
                    placeholder={data.resend_api_key_set ? "Saved. Type to replace" : "re_..."}
                  />
                </div>
              </FormGrid>
            </SettingsSection>
          )}

          <button type="submit" hidden aria-hidden tabIndex={-1} />
        </form>
      )}

      {data && (
        <SettingsSection
          title="Send test email"
          description="Uses the saved settings. Save your changes first."
        >
          <div className="flex flex-col gap-2 sm:flex-row">
            <Input
              type="email"
              value={testTo}
              onChange={(e) => setTestTo(e.target.value)}
              placeholder="Recipient (defaults to your email)"
              className="sm:max-w-sm"
            />
            <Button
              type="button"
              variant="outline"
              onClick={() => test.mutate()}
              disabled={test.isPending || dirty || !data.active_source}
            >
              {test.isPending ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <Send className="mr-2 h-4 w-4" />
              )}
              Send test email
            </Button>
          </div>
        </SettingsSection>
      )}

      <UnsavedChangesBar
        visible={dirty}
        saving={save.isPending}
        onReset={() => initial && setForm(initial)}
        onSave={() => form && save.mutate(form)}
      />
    </div>
  );
}
