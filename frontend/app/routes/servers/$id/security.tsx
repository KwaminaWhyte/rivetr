import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  AlertTriangle,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Info,
  Loader2,
  PackageSearch,
  Power,
  RefreshCw,
  Save,
  ShieldCheck,
  XCircle,
} from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { QueryError } from "@/components/query-error";
import { serverOpsApi } from "@/lib/api/server-ops";
import type { PackageUpdate, PatchJob } from "@/lib/api/server-ops";
import { ConfirmActionDialog } from "@/components/servers/confirm-action-dialog";
import { relativeTime } from "@/components/servers/server-ops-utils";
import { useServerContext } from "@/components/servers/server-context";

export function meta() {
  return [{ title: "Security - Rivetr" }];
}

const JOB_LABELS: Record<PatchJob["kind"], string> = {
  check: "Check for updates",
  update_package: "Update package",
  update_all: "Update all packages",
  reboot: "Reboot",
  configure_auto_updates: "Configure automatic updates",
};

function JobStatusIcon({ status }: { status: PatchJob["status"] }) {
  if (status === "running") return <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />;
  if (status === "success") return <CheckCircle2 className="h-4 w-4 text-green-600" />;
  return <XCircle className="h-4 w-4 text-destructive" />;
}

function JobRow({ job }: { job: PatchJob }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="rounded-md border">
      <button
        type="button"
        className="flex w-full flex-wrap items-center gap-3 px-3 py-2 text-left text-sm hover:bg-muted/40"
        onClick={() => setOpen((o) => !o)}
        aria-expanded={open}
      >
        {open ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
        <JobStatusIcon status={job.status} />
        <span className="font-medium">{JOB_LABELS[job.kind] ?? job.kind}</span>
        {job.package && <span className="font-mono text-xs">{job.package}</span>}
        <span className="ml-auto text-muted-foreground">
          {job.finished_at ? `finished ${relativeTime(job.finished_at)}` : `started ${relativeTime(job.started_at)}`}
        </span>
      </button>
      {open && (
        <pre className="max-h-80 overflow-auto whitespace-pre-wrap border-t bg-muted/30 p-3 font-mono text-xs">
          {job.log || "No output yet."}
        </pre>
      )}
    </div>
  );
}

type Confirm =
  | { kind: "reboot" }
  | { kind: "update_all" }
  | { kind: "update"; pkg: PackageUpdate };

export default function ServerSecurityPage() {
  const { serverId, name, isLocal } = useServerContext();
  const queryClient = useQueryClient();
  const [confirm, setConfirm] = useState<Confirm | null>(null);
  const [checkCron, setCheckCron] = useState("");
  const [checkEnabled, setCheckEnabled] = useState(true);
  const [autoUpdates, setAutoUpdates] = useState(false);
  const [maintWindow, setMaintWindow] = useState("03:00");

  const { data, error, isLoading, refetch } = useQuery({
    queryKey: ["server-patching", serverId],
    queryFn: () => serverOpsApi.getPatching(serverId),
    refetchInterval: (q) => (q.state.data?.running_job ? 3000 : false),
    // Keep polling through errors: during a reboot of Rivetr's own host the API
    // is gone for a minute, and the last data (with the running job) stays shown.
    refetchIntervalInBackground: true,
  });

  const settings = data?.settings;
  useEffect(() => {
    if (!settings) return;
    setCheckCron(settings.patch_check_cron);
    setCheckEnabled(settings.patch_check_enabled);
    setAutoUpdates(settings.auto_security_updates);
    setMaintWindow(settings.maintenance_window);
  }, [settings]);

  // Refresh the servers summary when a job finishes.
  const wasRunning = useRef(false);
  useEffect(() => {
    const running = !!data?.running_job;
    if (wasRunning.current && !running) {
      queryClient.invalidateQueries({ queryKey: ["server-ops-summary"] });
    }
    wasRunning.current = running;
  }, [data?.running_job, queryClient]);

  const onStarted = (msg: string) => {
    toast.success(msg);
    setConfirm(null);
    queryClient.invalidateQueries({ queryKey: ["server-patching", serverId] });
  };
  const onError = (e: Error) => toast.error(e.message || "Request failed");

  const checkMutation = useMutation({
    mutationFn: () => serverOpsApi.checkUpdates(serverId),
    onSuccess: () => onStarted("Checking for updates"),
    onError,
  });
  const updateMutation = useMutation({
    mutationFn: (pkg?: string) => serverOpsApi.updatePackages(serverId, pkg),
    onSuccess: (_d, pkg) => onStarted(pkg ? `Updating ${pkg}` : "Updating all packages"),
    onError,
  });
  const rebootMutation = useMutation({
    mutationFn: () => serverOpsApi.reboot(serverId),
    onSuccess: () => onStarted("Reboot requested"),
    onError,
  });
  const saveMutation = useMutation({
    mutationFn: () =>
      serverOpsApi.updateSettings(serverId, {
        patch_check_cron: checkCron,
        patch_check_enabled: checkEnabled,
        auto_security_updates: autoUpdates,
        maintenance_window: maintWindow,
      }),
    onSuccess: () => {
      toast.success("Patching settings saved");
      queryClient.invalidateQueries({ queryKey: ["server-patching", serverId] });
    },
    onError,
  });

  // Keep showing the last known state while Rivetr is unreachable during a reboot
  // of its own host; polling continues and the page recovers when it is back.
  const rebooting = data?.running_job?.kind === "reboot";
  if (error && !data) {
    return <QueryError what="patching status" error={error} onRetry={() => refetch()} />;
  }
  if (isLoading || !data || !settings) {
    return (
      <div className="flex justify-center py-12">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  const state = data.state;
  const busy = !!data.running_job;
  const dirty =
    checkCron !== settings.patch_check_cron ||
    checkEnabled !== settings.patch_check_enabled ||
    autoUpdates !== settings.auto_security_updates ||
    maintWindow !== settings.maintenance_window;
  const packages = state?.packages ?? [];
  const dockerPackages = packages.filter((p) => p.restarts_docker);
  // Before the first check the package manager is unknown, so allow trying.
  const autoSupported = data.supports_auto_updates || !state;

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="space-y-1">
              <CardTitle className="flex items-center gap-2">
                <ShieldCheck className="h-5 w-5" />
                Server patching
              </CardTitle>
              <div className="flex flex-wrap items-center gap-2 text-sm text-muted-foreground">
                <span>Last checked: {relativeTime(settings.last_patch_check_at)}</span>
                {state?.package_manager && state.package_manager !== "unknown" && (
                  <Badge variant="outline">{state.package_manager}</Badge>
                )}
                {state?.booted_at ? (
                  <span>Up since {new Date(state.booted_at * 1000).toLocaleString()}</span>
                ) : null}
                {state?.reboot_required && <Badge variant="destructive">Reboot required</Badge>}
              </div>
            </div>
            <div className="flex flex-wrap gap-2">
              <Button
                variant="outline"
                className="border-destructive/50 text-destructive hover:text-destructive"
                disabled={busy}
                onClick={() => setConfirm({ kind: "reboot" })}
              >
                <Power className="h-4 w-4" />
                Reboot Server
              </Button>
              <Button disabled={busy || checkMutation.isPending} onClick={() => checkMutation.mutate()}>
                {data.running_job?.kind === "check" ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <RefreshCw className="h-4 w-4" />
                )}
                Check For Updates
              </Button>
            </div>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          {data.running_job && (
            <div className="rounded-md border border-primary/30 bg-primary/5 p-3 text-sm">
              <div className="flex items-center gap-2 font-medium">
                <Loader2 className="h-4 w-4 animate-spin" />
                {rebooting
                  ? "Rebooting. Waiting for the server to come back..."
                  : `${JOB_LABELS[data.running_job.kind]}${data.running_job.package ? ` ${data.running_job.package}` : ""} in progress`}
              </div>
              <p className="mt-1 text-muted-foreground">
                {rebooting
                  ? isLocal
                    ? "Rivetr runs on this server, so the dashboard is unreachable for a minute or two. This page reconnects by itself."
                    : "This usually takes one to three minutes. The job finishes when the server reports a new boot time."
                  : "This runs on the server itself, so it keeps going if you leave this page or Rivetr restarts."}
              </p>
              {error && (
                <p className="mt-1 text-muted-foreground">Rivetr is not reachable right now, retrying...</p>
              )}
              {!rebooting && data.running_job.log && (
                <pre className="mt-2 max-h-48 overflow-auto rounded bg-muted/60 p-2 font-mono text-xs leading-relaxed whitespace-pre-wrap">
                  {data.running_job.log.split("\n").slice(-15).join("\n")}
                </pre>
              )}
            </div>
          )}
          {state?.error && (
            <div className="flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">
              <XCircle className="mt-0.5 h-4 w-4 shrink-0" />
              <span>{state.error}</span>
            </div>
          )}
          <div className="grid gap-5 md:grid-cols-3">
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label htmlFor="check-cron">Check frequency (cron, UTC)</Label>
                <Switch
                  aria-label="Scheduled update checks"
                  checked={checkEnabled}
                  onCheckedChange={setCheckEnabled}
                />
              </div>
              <Input
                id="check-cron"
                className="font-mono"
                value={checkCron}
                disabled={!checkEnabled}
                onChange={(e) => setCheckCron(e.target.value)}
                placeholder="0 0 * * *"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="auto-updates">Automatic security updates</Label>
              <div className="flex h-8 items-center gap-3">
                <Switch
                  id="auto-updates"
                  checked={autoUpdates}
                  disabled={!autoSupported}
                  onCheckedChange={setAutoUpdates}
                />
                <span className="text-sm text-muted-foreground">
                  {autoUpdates ? "On" : "Off"}
                  {!autoSupported && " (needs apt or dnf)"}
                </span>
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="maint-window">Maintenance window</Label>
              <Input
                id="maint-window"
                type="time"
                value={maintWindow}
                onChange={(e) => setMaintWindow(e.target.value)}
              />
              <p className="text-xs text-muted-foreground">Server local time, used by automatic updates.</p>
            </div>
          </div>
          <div className="flex justify-end">
            <Button disabled={!dirty || saveMutation.isPending} onClick={() => saveMutation.mutate()}>
              {saveMutation.isPending ? <Loader2 className="h-4 w-4 animate-spin" /> : <Save className="h-4 w-4" />}
              Save
            </Button>
          </div>
          <div className="flex items-start gap-2 rounded-md border bg-muted/40 p-3 text-sm text-muted-foreground">
            <Info className="mt-0.5 h-4 w-4 shrink-0" />
            <p>
              Supported package managers: apk, apt, dnf (and yum), pacman, zypper. Automatic security
              updates are available for apt (unattended-upgrades) and dnf (dnf-automatic).
              {!isLocal && " Commands run over SSH; a non-root user needs passwordless sudo."}
            </p>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <CardTitle className="flex items-center gap-2">
                <PackageSearch className="h-5 w-5" />
                Package updates
              </CardTitle>
              {dockerPackages.length > 0 && (
                <CardDescription className="flex items-center gap-1.5 text-amber-700 dark:text-amber-400">
                  <AlertTriangle className="h-4 w-4" />
                  {dockerPackages.length} update{dockerPackages.length === 1 ? "" : "s"} restart Docker
                  and the running containers.
                </CardDescription>
              )}
            </div>
            <Button
              variant="outline"
              disabled={busy || packages.length === 0}
              onClick={() => setConfirm({ kind: "update_all" })}
            >
              Update All Packages
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {!state ? (
            <p className="py-8 text-center text-sm text-muted-foreground">
              Run "Check For Updates" to list pending package updates.
            </p>
          ) : packages.length === 0 ? (
            <p className="py-8 text-center text-sm text-muted-foreground">
              Everything is up to date.
            </p>
          ) : (
            <>
              <ul className="divide-y rounded-md border">
                {packages.map((p) => (
                  <li key={p.name} className="flex flex-wrap items-center gap-3 px-3 py-2.5">
                    <div className="min-w-0 flex-1">
                      <div className="flex flex-wrap items-center gap-2">
                        <span className="font-medium">{p.name}</span>
                        <span className="font-mono text-xs">{p.new_version}</span>
                        {p.security && (
                          <Badge variant="destructive" className="text-[10px]">
                            Security
                          </Badge>
                        )}
                        {p.held && (
                          <Badge variant="outline" className="text-[10px]" title={`Held with apt-mark hold. Run "sudo apt-mark unhold ${p.name}" on the server to allow updates.`}>
                            Held
                          </Badge>
                        )}
                      </div>
                      <div className="flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
                        {p.current_version && <span>Current: {p.current_version}</span>}
                        {p.restarts_docker && (
                          <span className="flex items-center gap-1 text-amber-700 dark:text-amber-400">
                            <AlertTriangle className="h-3 w-3" />
                            Restarts Docker and the running containers
                          </span>
                        )}
                      </div>
                    </div>
                    <Button
                      size="sm"
                      variant="outline"
                      disabled={busy || p.held}
                      title={p.held ? "Held on the server; unhold it to allow updates" : undefined}
                      onClick={() =>
                        p.restarts_docker
                          ? setConfirm({ kind: "update", pkg: p })
                          : updateMutation.mutate(p.name)
                      }
                    >
                      Update
                    </Button>
                  </li>
                ))}
              </ul>
              <p className="pt-3 text-sm text-muted-foreground">
                {packages.length} package update{packages.length === 1 ? "" : "s"}
                {state.packages.some((p) => p.security) &&
                  `, ${state.packages.filter((p) => p.security).length} security`}
              </p>
            </>
          )}
        </CardContent>
      </Card>

      {data.recent_jobs.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Recent activity</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            {data.recent_jobs.map((job) => (
              <JobRow key={job.id} job={job} />
            ))}
          </CardContent>
        </Card>
      )}

      <ConfirmActionDialog
        open={confirm?.kind === "reboot"}
        onOpenChange={(o) => !o && setConfirm(null)}
        title={`Reboot ${name}?`}
        description={
          <>
            <p>Every container on this server stops until the machine is back up.</p>
            {isLocal && (
              <p className="font-medium text-destructive">
                This is the machine Rivetr runs on. The dashboard is unavailable until it restarts.
              </p>
            )}
          </>
        }
        confirmLabel="Reboot Server"
        confirmText={name}
        pending={rebootMutation.isPending}
        onConfirm={() => rebootMutation.mutate()}
      />
      <ConfirmActionDialog
        open={confirm?.kind === "update_all"}
        onOpenChange={(o) => !o && setConfirm(null)}
        title="Update all packages?"
        description={
          <>
            <p>
              Installs {packages.length} package update{packages.length === 1 ? "" : "s"} on {name}.
            </p>
            {dockerPackages.length > 0 && (
              <p className="text-amber-600 dark:text-amber-400">
                This includes {dockerPackages.map((p) => p.name).join(", ")}, which restarts Docker
                and every running container.
              </p>
            )}
          </>
        }
        confirmLabel="Update All"
        confirmText={dockerPackages.length > 0 ? name : undefined}
        pending={updateMutation.isPending}
        onConfirm={() => updateMutation.mutate(undefined)}
      />
      <ConfirmActionDialog
        open={confirm?.kind === "update"}
        onOpenChange={(o) => !o && setConfirm(null)}
        title={confirm?.kind === "update" ? `Update ${confirm.pkg.name}?` : "Update package?"}
        description={
          <p>Updating this package restarts Docker and every running container on {name}.</p>
        }
        confirmLabel="Update"
        pending={updateMutation.isPending}
        onConfirm={() => confirm?.kind === "update" && updateMutation.mutate(confirm.pkg.name)}
      />
    </div>
  );
}
