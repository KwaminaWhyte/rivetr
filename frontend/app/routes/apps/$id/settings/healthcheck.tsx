import { useMemo } from "react";
import { useOutletContext } from "react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { HeartPulse, Timer } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { UnsavedChangesBar } from "@/components/unsaved-changes-bar";
import { useDirtyForm } from "@/hooks/use-dirty-form";
import { appConfigApi } from "@/lib/api";
import type { App } from "@/types/api";
import type { HealthcheckConfig } from "@/types/app-config";

export function meta() {
  return [
    { title: "Healthcheck - Rivetr" },
    { name: "description", content: "Configure how Rivetr checks that the app is healthy" },
  ];
}

const DEFAULT_CONFIG: HealthcheckConfig = {
  enabled: false,
  check_type: "http",
  method: "GET",
  scheme: "http",
  host: "localhost",
  port: null,
  path: "/",
  expected_status: 200,
  expected_body: null,
  command: null,
  interval_secs: 5,
  timeout_secs: 5,
  retries: 10,
  start_period_secs: 5,
};

/** Form values keep numbers as strings so fields can be cleared while typing. */
interface FormValues {
  enabled: boolean;
  check_type: HealthcheckConfig["check_type"];
  method: HealthcheckConfig["method"];
  scheme: HealthcheckConfig["scheme"];
  host: string;
  port: string;
  path: string;
  expected_status: string;
  expected_body: string;
  command: string;
  interval_secs: string;
  timeout_secs: string;
  retries: string;
  start_period_secs: string;
}

function toForm(c: HealthcheckConfig): FormValues {
  return {
    enabled: c.enabled,
    check_type: c.check_type,
    method: c.method,
    scheme: c.scheme,
    host: c.host,
    port: c.port == null ? "" : String(c.port),
    path: c.path,
    expected_status: String(c.expected_status),
    expected_body: c.expected_body ?? "",
    command: c.command ?? "",
    interval_secs: String(c.interval_secs),
    timeout_secs: String(c.timeout_secs),
    retries: String(c.retries),
    start_period_secs: String(c.start_period_secs),
  };
}

function toConfig(v: FormValues): HealthcheckConfig {
  const int = (s: string, fallback: number) => {
    const n = parseInt(s, 10);
    return Number.isFinite(n) ? n : fallback;
  };
  return {
    enabled: v.enabled,
    check_type: v.check_type,
    method: v.method,
    scheme: v.scheme,
    host: v.host.trim() || "localhost",
    port: v.port.trim() === "" ? null : int(v.port, 0),
    path: v.path.trim() || "/",
    expected_status: int(v.expected_status, 200),
    expected_body: v.expected_body.trim() || null,
    command: v.command.trim() || null,
    interval_secs: int(v.interval_secs, 5),
    timeout_secs: int(v.timeout_secs, 5),
    retries: int(v.retries, 10),
    start_period_secs: int(v.start_period_secs, 5),
  };
}

function validate(v: FormValues): string | null {
  if (!v.enabled) return null;
  const inRange = (s: string, min: number, max: number) => {
    const n = Number(s);
    return s.trim() !== "" && Number.isInteger(n) && n >= min && n <= max;
  };
  if (v.check_type === "http") {
    if (!v.path.trim().startsWith("/")) return "Path must start with /";
    if (v.port.trim() !== "" && !inRange(v.port, 1, 65535)) return "Port must be between 1 and 65535";
    if (!inRange(v.expected_status, 100, 599)) return "Expected status code must be between 100 and 599";
  } else if (!v.command.trim()) {
    return "Enter a command";
  }
  if (!inRange(v.interval_secs, 1, 3600)) return "Interval must be between 1 and 3600 seconds";
  if (!inRange(v.timeout_secs, 1, 3600)) return "Timeout must be between 1 and 3600 seconds";
  if (!inRange(v.retries, 1, 100)) return "Retries must be between 1 and 100";
  if (!inRange(v.start_period_secs, 0, 3600)) return "Start period must be between 0 and 3600 seconds";
  return null;
}

function Field({
  id,
  label,
  hint,
  children,
}: {
  id: string;
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      {children}
      {hint && <p className="text-xs text-muted-foreground">{hint}</p>}
    </div>
  );
}

export default function AppHealthcheckSettings() {
  const { app } = useOutletContext<{ app: App }>();
  const queryClient = useQueryClient();

  const { data, isLoading } = useQuery({
    queryKey: ["app-healthcheck", app.id],
    queryFn: () => appConfigApi.getHealthcheck(app.id),
  });

  const initial = useMemo(() => toForm(data ?? DEFAULT_CONFIG), [data]);
  const form = useDirtyForm<FormValues>(initial);
  const v = form.values;
  const error = validate(v);

  const save = useMutation({
    mutationFn: () => appConfigApi.updateHealthcheck(app.id, toConfig(v)),
    onSuccess: (saved) => {
      queryClient.setQueryData(["app-healthcheck", app.id], saved);
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
      form.markSaved(toForm(saved));
      toast.success("Healthcheck saved. It applies on the next deployment.");
    },
    onError: (e: Error) => toast.error(e.message || "Failed to save healthcheck"),
  });

  const handleSave = () => {
    if (error) {
      toast.error(error);
      return;
    }
    save.mutate();
  };

  if (isLoading) {
    return <Skeleton className="h-96 w-full" />;
  }

  const disabled = !v.enabled;

  return (
    <div className="space-y-6 pb-24">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <HeartPulse className="h-5 w-5" />
            Healthcheck
          </CardTitle>
          <CardDescription>
            Rivetr runs this check after each deploy before switching traffic, and sets it as
            the container HEALTHCHECK. Changes apply on the next deployment.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="flex items-center justify-between rounded-lg border p-4">
            <div className="space-y-0.5">
              <Label htmlFor="hc-enabled" className="text-base">
                Enable healthcheck
              </Label>
              <p className="text-sm text-muted-foreground">
                When off, a deploy only waits for the port to accept connections.
              </p>
            </div>
            <Switch
              id="hc-enabled"
              checked={v.enabled}
              onCheckedChange={(checked) => form.setField("enabled", checked)}
            />
          </div>

          <fieldset disabled={disabled} className="space-y-6 disabled:opacity-60">
            <div className="grid gap-4 md:grid-cols-2">
              <Field id="hc-type" label="Check type">
                <Select
                  value={v.check_type}
                  onValueChange={(value) =>
                    form.setField("check_type", value as FormValues["check_type"])
                  }
                  disabled={disabled}
                >
                  <SelectTrigger id="hc-type">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="http">HTTP request</SelectItem>
                    <SelectItem value="command">Command</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
            </div>

            {v.check_type === "http" ? (
              <div className="grid gap-4 md:grid-cols-2">
                <Field id="hc-method" label="Method">
                  <Select
                    value={v.method}
                    onValueChange={(value) => form.setField("method", value as FormValues["method"])}
                    disabled={disabled}
                  >
                    <SelectTrigger id="hc-method">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {["GET", "HEAD", "POST", "OPTIONS"].map((m) => (
                        <SelectItem key={m} value={m}>
                          {m}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </Field>
                <Field id="hc-scheme" label="Scheme">
                  <Select
                    value={v.scheme}
                    onValueChange={(value) => form.setField("scheme", value as FormValues["scheme"])}
                    disabled={disabled}
                  >
                    <SelectTrigger id="hc-scheme">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="http">HTTP</SelectItem>
                      <SelectItem value="https">HTTPS</SelectItem>
                    </SelectContent>
                  </Select>
                </Field>
                <Field id="hc-host" label="Host" hint="Host inside the container, usually localhost.">
                  <Input
                    id="hc-host"
                    value={v.host}
                    onChange={(e) => form.setField("host", e.target.value)}
                    placeholder="localhost"
                  />
                </Field>
                <Field id="hc-port" label="Port" hint={`Leave empty to use the app port (${app.port}).`}>
                  <Input
                    id="hc-port"
                    type="number"
                    min={1}
                    max={65535}
                    value={v.port}
                    onChange={(e) => form.setField("port", e.target.value)}
                    placeholder={String(app.port)}
                  />
                </Field>
                <Field id="hc-path" label="Path">
                  <Input
                    id="hc-path"
                    value={v.path}
                    onChange={(e) => form.setField("path", e.target.value)}
                    placeholder="/health"
                    className="font-mono"
                  />
                </Field>
                <Field
                  id="hc-status"
                  label="Expected status code"
                  hint="200 accepts any 2xx response."
                >
                  <Input
                    id="hc-status"
                    type="number"
                    min={100}
                    max={599}
                    value={v.expected_status}
                    onChange={(e) => form.setField("expected_status", e.target.value)}
                  />
                </Field>
                <div className="md:col-span-2">
                  <Field
                    id="hc-body"
                    label="Expected response text"
                    hint="Optional. The response body must contain this text."
                  >
                    <Input
                      id="hc-body"
                      value={v.expected_body}
                      onChange={(e) => form.setField("expected_body", e.target.value)}
                      placeholder="ok"
                    />
                  </Field>
                </div>
              </div>
            ) : (
              <Field
                id="hc-command"
                label="Command"
                hint="Runs with sh -c inside the container. Exit code 0 means healthy."
              >
                <Textarea
                  id="hc-command"
                  value={v.command}
                  onChange={(e) => form.setField("command", e.target.value)}
                  placeholder="pg_isready -U postgres"
                  className="font-mono text-sm"
                  rows={3}
                />
              </Field>
            )}
          </fieldset>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Timer className="h-5 w-5" />
            Timing and retries
          </CardTitle>
          <CardDescription>All values are in seconds, except retries.</CardDescription>
        </CardHeader>
        <CardContent>
          <fieldset disabled={disabled} className="grid gap-4 md:grid-cols-2 disabled:opacity-60">
            <Field id="hc-interval" label="Interval" hint="Time between checks.">
              <Input
                id="hc-interval"
                type="number"
                min={1}
                value={v.interval_secs}
                onChange={(e) => form.setField("interval_secs", e.target.value)}
              />
            </Field>
            <Field id="hc-timeout" label="Timeout" hint="How long one check may take.">
              <Input
                id="hc-timeout"
                type="number"
                min={1}
                value={v.timeout_secs}
                onChange={(e) => form.setField("timeout_secs", e.target.value)}
              />
            </Field>
            <Field id="hc-retries" label="Retries" hint="Failed checks before the container counts as unhealthy.">
              <Input
                id="hc-retries"
                type="number"
                min={1}
                value={v.retries}
                onChange={(e) => form.setField("retries", e.target.value)}
              />
            </Field>
            <Field id="hc-start" label="Start period" hint="Grace time after start before checks count.">
              <Input
                id="hc-start"
                type="number"
                min={0}
                value={v.start_period_secs}
                onChange={(e) => form.setField("start_period_secs", e.target.value)}
              />
            </Field>
          </fieldset>
          {error && v.enabled && <p className="mt-4 text-sm text-destructive">{error}</p>}
        </CardContent>
      </Card>

      <UnsavedChangesBar
        show={form.isDirty}
        onSave={handleSave}
        onReset={form.reset}
        saving={save.isPending}
        saveDisabled={Boolean(error)}
      />
    </div>
  );
}
