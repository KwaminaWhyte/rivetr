import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { AlertTriangle } from "lucide-react";

import { QueryError } from "@/components/query-error";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { FormGrid, SettingsSection, UnsavedChangesBar } from "@/components/settings-section";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { instanceSettingsApi } from "@/lib/api/instance-settings";
import type { AdvancedSettings, ConfirmationMode } from "@/types/instance-settings";

export function meta() {
  return [
    { title: "Advanced Settings - Rivetr" },
    { name: "description", content: "Registration, API access, MCP, DNS and outbound policy" },
  ];
}

/** Form state: list settings are edited as multi-line text. */
interface FormState {
  registration_enabled: boolean;
  allow_password_registration: boolean;
  destructive_confirmation: ConfirmationMode;
  api_access_enabled: boolean;
  api_allowed_ips: string;
  mcp_enabled: boolean;
  dns_validation_enabled: boolean;
  dns_servers: string;
  outbound_allowed_targets: string;
  outbound_allow_localhost: boolean;
  passkeys_enabled: boolean;
}

function toForm(s: AdvancedSettings): FormState {
  return {
    ...s,
    api_allowed_ips: s.api_allowed_ips.join("\n"),
    dns_servers: s.dns_servers.join("\n"),
    outbound_allowed_targets: s.outbound_allowed_targets.join("\n"),
  };
}

function toList(text: string): string[] {
  return text
    .split(/[\n,\s]+/)
    .map((s) => s.trim())
    .filter(Boolean);
}

function BoolSelect({
  id,
  label,
  value,
  onChange,
  on,
  off,
  hint,
}: {
  id: string;
  label: string;
  value: boolean;
  onChange: (v: boolean) => void;
  on: string;
  off: string;
  hint?: ReactNode;
}) {
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      <Select value={value ? "on" : "off"} onValueChange={(v) => onChange(v === "on")}>
        <SelectTrigger id={id} className="w-full">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="on">{on}</SelectItem>
          <SelectItem value="off">{off}</SelectItem>
        </SelectContent>
      </Select>
      {hint && <p className="text-xs text-muted-foreground">{hint}</p>}
    </div>
  );
}

function ListField({
  id,
  label,
  value,
  onChange,
  placeholder,
  hint,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder: string;
  hint?: ReactNode;
}) {
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      <Textarea
        id={id}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        rows={3}
        className="font-mono text-sm"
      />
      {hint && <p className="text-xs text-muted-foreground">{hint}</p>}
    </div>
  );
}

export default function AdvancedSettingsPage() {
  const queryClient = useQueryClient();
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ["advanced-settings"],
    queryFn: () => instanceSettingsApi.getAdvancedSettings(),
  });

  const initial = useMemo(() => (data ? toForm(data) : null), [data]);
  const [form, setForm] = useState<FormState | null>(null);
  const [previewOpen, setPreviewOpen] = useState(false);

  useEffect(() => {
    if (initial) setForm(initial);
  }, [initial]);

  const dirty = !!form && !!initial && JSON.stringify(form) !== JSON.stringify(initial);

  const save = useMutation({
    mutationFn: (f: FormState) =>
      instanceSettingsApi.updateAdvancedSettings({
        ...f,
        api_allowed_ips: toList(f.api_allowed_ips),
        dns_servers: toList(f.dns_servers),
        outbound_allowed_targets: toList(f.outbound_allowed_targets),
      }),
    onSuccess: (saved) => {
      queryClient.setQueryData(["advanced-settings"], saved);
      queryClient.invalidateQueries({ queryKey: ["destructive-confirmation"] });
      toast.success("Advanced settings saved");
    },
    onError: (e) => toast.error(e instanceof Error ? e.message : "Failed to save settings"),
  });

  const set = <K extends keyof FormState>(key: K, value: FormState[K]) =>
    setForm((f) => (f ? { ...f, [key]: value } : f));

  if (error) {
    return <QueryError what="advanced settings" error={error} onRetry={() => refetch()} />;
  }

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-2xl font-bold">Advanced</h1>
        <p className="text-sm text-muted-foreground">
          Sign up, API and network policies for this instance.
        </p>
      </div>

      {isLoading || !form ? (
        <div className="space-y-4">
          <Skeleton className="h-32 w-full" />
          <Skeleton className="h-32 w-full" />
        </div>
      ) : (
        <form
          className="space-y-8"
          onSubmit={(e) => {
            e.preventDefault();
            if (dirty) save.mutate(form);
          }}
        >
          <SettingsSection
            title="Registration"
            description="Who can create an account. Invitations always work."
          >
            <FormGrid>
              <BoolSelect
                id="registration"
                label="Registration"
                value={form.registration_enabled}
                onChange={(v) => set("registration_enabled", v)}
                on="Registration enabled"
                off="Registration disabled"
                hint="When disabled, new users join only through team invitations. First-time OAuth and SSO logins follow this setting too."
              />
              <BoolSelect
                id="password-registration"
                label="Password registration"
                value={form.allow_password_registration}
                onChange={(v) => set("allow_password_registration", v)}
                on="Allowed"
                off="OAuth and SSO only"
                hint="Applies while an OAuth or SSO login provider is enabled."
              />
            </FormGrid>
          </SettingsSection>

          <SettingsSection
            title="Destructive actions"
            description="How deletes and other irreversible actions are confirmed."
            actions={
              <Button type="button" variant="outline" size="sm" onClick={() => setPreviewOpen(true)}>
                Preview
              </Button>
            }
          >
            <FormGrid>
              <div className="space-y-2">
                <Label htmlFor="confirmation">Confirmation</Label>
                <Select
                  value={form.destructive_confirmation}
                  onValueChange={(v) => set("destructive_confirmation", v as ConfirmationMode)}
                >
                  <SelectTrigger id="confirmation" className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="simple">Simple confirmation</SelectItem>
                    <SelectItem value="two_step">Two-step: type the resource name</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </FormGrid>
          </SettingsSection>

          <SettingsSection
            title="API access"
            description="Applies to API tokens and the admin token. Dashboard sessions are never blocked."
          >
            <div className="space-y-4">
              {form.api_access_enabled && toList(form.api_allowed_ips).length === 0 && (
                <div className="flex gap-2 rounded-md border border-yellow-500/40 bg-yellow-500/10 p-3 text-sm text-yellow-800 dark:text-yellow-300">
                  <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
                  <span>
                    API access is open to every source. Restrict the allowlist before using API
                    access on a public production instance.
                  </span>
                </div>
              )}
              <FormGrid>
                <BoolSelect
                  id="api-access"
                  label="API access"
                  value={form.api_access_enabled}
                  onChange={(v) => set("api_access_enabled", v)}
                  on="Enabled"
                  off="Disabled"
                />
                <ListField
                  id="api-allowed-ips"
                  label="Allowed IPs"
                  value={form.api_allowed_ips}
                  onChange={(v) => set("api_allowed_ips", v)}
                  placeholder={"203.0.113.10\n10.0.0.0/8"}
                  hint="One IP or CIDR per line. Leave empty to allow every source."
                />
              </FormGrid>
            </div>
          </SettingsSection>

          <SettingsSection
            title="Passkeys"
            description="Let users register passkeys under Account security and sign in with them. A passkey sign in also satisfies two-factor authentication."
          >
            <FormGrid>
              <BoolSelect
                id="passkeys"
                label="Passkeys"
                value={form.passkeys_enabled}
                onChange={(v) => set("passkeys_enabled", v)}
                on="Enabled"
                off="Disabled"
                hint="Passkeys are bound to the instance domain. Set it in Settings > General before users register one."
              />
            </FormGrid>
          </SettingsSection>

          <SettingsSection
            title="MCP server"
            description="Model Context Protocol endpoint at /mcp for AI assistants. Requires an API token."
          >
            <FormGrid>
              <BoolSelect
                id="mcp"
                label="MCP server"
                value={form.mcp_enabled}
                onChange={(v) => set("mcp_enabled", v)}
                on="Enabled"
                off="Disabled"
              />
            </FormGrid>
          </SettingsSection>

          <SettingsSection
            title="DNS validation"
            description="Checks that custom domains point at this server."
          >
            <FormGrid>
              <BoolSelect
                id="dns-validation"
                label="DNS validation"
                value={form.dns_validation_enabled}
                onChange={(v) => set("dns_validation_enabled", v)}
                on="Enabled"
                off="Disabled"
              />
              <ListField
                id="dns-servers"
                label="Custom DNS servers"
                value={form.dns_servers}
                onChange={(v) => set("dns_servers", v)}
                placeholder={"1.1.1.1\n8.8.8.8"}
                hint="Leave empty to use the server's resolver."
              />
            </FormGrid>
          </SettingsSection>

          <SettingsSection
            title="Outbound endpoints"
            description="Webhooks, notification channels and log drains can not reach internal addresses unless listed here."
          >
            <FormGrid>
              <ListField
                id="outbound-targets"
                label="Allowed internal targets"
                value={form.outbound_allowed_targets}
                onChange={(v) => set("outbound_allowed_targets", v)}
                placeholder={"10.20.0.0/16"}
                hint="One IP or CIDR per line. Cloud metadata addresses stay blocked unless listed."
              />
              <BoolSelect
                id="outbound-localhost"
                label="Localhost"
                value={form.outbound_allow_localhost}
                onChange={(v) => set("outbound_allow_localhost", v)}
                on="Allowed"
                off="Blocked"
              />
            </FormGrid>
          </SettingsSection>

          <button type="submit" hidden aria-hidden tabIndex={-1} />
        </form>
      )}

      <UnsavedChangesBar
        visible={dirty}
        saving={save.isPending}
        onReset={() => initial && setForm(initial)}
        onSave={() => form && save.mutate(form)}
      />

      {form && (
        <ConfirmDialog
          open={previewOpen}
          onOpenChange={setPreviewOpen}
          mode={form.destructive_confirmation}
          title="Delete my-app?"
          description="This is a preview of the confirmation dialog. Nothing is deleted."
          resourceName="my-app"
          onConfirm={() => setPreviewOpen(false)}
        />
      )}
    </div>
  );
}
