import { useEffect, useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Plus, Trash2, Server as ServerIcon, ShieldAlert, HardDrive } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Badge } from "@/components/ui/badge";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { QueryError } from "@/components/query-error";
import { api } from "@/lib/api";
import {
  DEFAULT_MULTI_SERVER_SETTINGS,
  placementApi,
  type ImageDistribution,
  type LbPolicy,
  type MultiServerSettings,
  type PlacementInput,
} from "@/lib/api/placement";
import type { App } from "@/types/api";

const LOCAL = "localhost";

const DISTRIBUTION_HELP: Record<ImageDistribution, string> = {
  auto: "Registry pull when this app pushes to a registry, otherwise SSH streaming.",
  registry:
    "Push once, every server pulls. Fast for many servers and uses layer caching, but needs a registry (Build settings > registry push).",
  ssh: "docker save | ssh docker load to each server. No registry needed, but the full image crosses the network once per server on every deploy.",
};

/** Placement editor: which servers run this app and how a deploy rolls across them. */
export function ServerPlacementCard({ app }: { app: App }) {
  const queryClient = useQueryClient();
  const { data, error, refetch } = useQuery({
    queryKey: ["placement", app.id],
    queryFn: () => placementApi.get(app.id),
  });
  const { data: servers = [] } = useQuery({
    queryKey: ["servers"],
    queryFn: () => api.getServers(),
  });

  const [rows, setRows] = useState<PlacementInput[]>([]);
  const [settings, setSettings] = useState<MultiServerSettings>(DEFAULT_MULTI_SERVER_SETTINGS);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!data) return;
    setRows(data.placements.map((p) => ({ server_id: p.server_id, replicas: p.replicas })));
    setSettings(data.settings);
  }, [data]);

  const serverOptions = useMemo(
    () => [
      { id: LOCAL, name: "Rivetr host (this server)" },
      ...servers.map((s) => ({ id: s.id, name: `${s.name} (${s.host})` })),
    ],
    [servers],
  );
  const unused = serverOptions.filter((o) => !rows.some((r) => r.server_id === o.id));
  const totalContainers = rows.reduce((n, r) => n + r.replicas, 0);
  const hasRemote = rows.some((r) => r.server_id !== LOCAL);
  const publicServers = (data?.placements ?? []).filter((p) => !p.is_local && !p.private_bind);

  const save = async (next: PlacementInput[]) => {
    setSaving(true);
    try {
      await placementApi.update(app.id, next, settings);
      toast.success(
        next.length === 0
          ? "Multi-server placement removed. The next deploy runs on a single host."
          : "Placement saved. It applies on the next deployment.",
      );
      await refetch();
      queryClient.invalidateQueries({ queryKey: ["placement-counts"] });
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to save placement");
    } finally {
      setSaving(false);
    }
  };

  const update = (i: number, patch: Partial<PlacementInput>) =>
    setRows(rows.map((r, j) => (j === i ? { ...r, ...patch } : r)));

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-2">
          <CardTitle>Servers</CardTitle>
          {data?.enabled && (
            <Badge variant="secondary" className="text-xs">
              {data.server_count} {data.server_count === 1 ? "server" : "servers"}
            </Badge>
          )}
        </div>
        <CardDescription>
          Run this app on several servers behind the Rivetr proxy. The image is built once,
          sent to every server, and rolled out in batches with a health check per container.
          Leave the list empty to deploy on a single host as before.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {error ? (
          <QueryError what="placement" error={error} onRetry={() => refetch()} />
        ) : (
          <>
            <div className="space-y-3">
              {rows.length === 0 && (
                <p className="text-sm text-muted-foreground">
                  No placement. Deploys use the single-host pipeline.
                </p>
              )}
              {rows.map((row, i) => {
                const options = serverOptions.filter(
                  (o) => o.id === row.server_id || !rows.some((r) => r.server_id === o.id),
                );
                const view = data?.placements.find((p) => p.server_id === row.server_id);
                return (
                  <div key={row.server_id} className="flex flex-wrap items-end gap-3 rounded-md border p-3">
                    <div className="space-y-1.5 min-w-56 flex-1">
                      <Label htmlFor={`placement-server-${i}`}>Server</Label>
                      <Select
                        value={row.server_id}
                        onValueChange={(v) => update(i, { server_id: v })}
                      >
                        <SelectTrigger id={`placement-server-${i}`} aria-label="Server">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          {options.map((o) => (
                            <SelectItem key={o.id} value={o.id}>
                              {o.name}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                      {view?.address && (
                        <p className="text-xs text-muted-foreground">
                          Proxy reaches it at {view.address}
                          {view.private_bind ? " (private)" : " (public)"}
                        </p>
                      )}
                    </div>
                    <div className="space-y-1.5 w-28">
                      <Label htmlFor={`placement-replicas-${i}`}>Containers</Label>
                      <Input
                        id={`placement-replicas-${i}`}
                        type="number"
                        min={1}
                        max={10}
                        value={row.replicas}
                        onChange={(e) =>
                          update(i, {
                            replicas: Math.max(1, Math.min(10, parseInt(e.target.value) || 1)),
                          })
                        }
                      />
                    </div>
                    <Button
                      variant="ghost"
                      size="sm"
                      aria-label="Remove server from placement"
                      className="text-destructive hover:text-destructive"
                      onClick={() => setRows(rows.filter((_, j) => j !== i))}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                );
              })}
              <Button
                variant="outline"
                size="sm"
                className="gap-2"
                disabled={unused.length === 0}
                onClick={() => setRows([...rows, { server_id: unused[0].id, replicas: 1 }])}
              >
                <Plus className="h-4 w-4" />
                Add server
              </Button>
            </div>

            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-1.5">
                <Label htmlFor="ms-batch">Batch size (max unavailable)</Label>
                <Input
                  id="ms-batch"
                  type="number"
                  min={1}
                  max={50}
                  value={settings.max_unavailable}
                  onChange={(e) =>
                    setSettings({
                      ...settings,
                      max_unavailable: Math.max(1, Math.min(50, parseInt(e.target.value) || 1)),
                    })
                  }
                />
                <p className="text-xs text-muted-foreground">
                  Containers replaced at a time. Old containers keep serving until their
                  replacement passes its health check.
                </p>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="ms-threshold">Roll back when failed (%)</Label>
                <Input
                  id="ms-threshold"
                  type="number"
                  min={0}
                  max={100}
                  value={settings.failure_threshold_pct}
                  onChange={(e) =>
                    setSettings({
                      ...settings,
                      failure_threshold_pct: Math.max(
                        0,
                        Math.min(100, parseInt(e.target.value) || 0),
                      ),
                    })
                  }
                />
                <p className="text-xs text-muted-foreground">
                  The whole deployment rolls back once this share of {totalContainers || "the"}{" "}
                  containers fail. 0 rolls back on any failure.
                </p>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="ms-distribution">Image distribution</Label>
                <Select
                  value={settings.distribution}
                  onValueChange={(v) =>
                    setSettings({ ...settings, distribution: v as ImageDistribution })
                  }
                >
                  <SelectTrigger id="ms-distribution" aria-label="Image distribution">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="auto">Automatic</SelectItem>
                    <SelectItem value="registry">Registry push and pull</SelectItem>
                    <SelectItem value="ssh">Stream over SSH</SelectItem>
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  {DISTRIBUTION_HELP[settings.distribution]}
                  {data && !data.registry_configured && settings.distribution !== "ssh" && (
                    <> No registry is configured for this app, so images are streamed over SSH.</>
                  )}
                </p>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="ms-lb">Load balancing</Label>
                <Select
                  value={settings.lb_policy}
                  onValueChange={(v) => setSettings({ ...settings, lb_policy: v as LbPolicy })}
                >
                  <SelectTrigger id="ms-lb" aria-label="Load balancing policy">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="round_robin">Round robin</SelectItem>
                    <SelectItem value="least_conn">Least connections</SelectItem>
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  A container that refuses connections is skipped for 10 seconds.
                </p>
              </div>
            </div>

            <div className="grid gap-3 sm:grid-cols-2">
              <div className="flex items-center justify-between gap-4 rounded-lg border p-3">
                <div>
                  <Label htmlFor="ms-sticky">Sticky sessions</Label>
                  <p className="text-xs text-muted-foreground">
                    Pin each browser to one container with a cookie.
                  </p>
                </div>
                <Switch
                  id="ms-sticky"
                  checked={settings.sticky_sessions}
                  onCheckedChange={(v) => setSettings({ ...settings, sticky_sessions: v })}
                />
              </div>
              <div className="flex items-center justify-between gap-4 rounded-lg border p-3">
                <div>
                  <Label htmlFor="ms-private">Bind to private address</Label>
                  <p className="text-xs text-muted-foreground">
                    Publish ports on the server's Tailscale address when it has one.
                  </p>
                </div>
                <Switch
                  id="ms-private"
                  checked={settings.bind_private}
                  onCheckedChange={(v) => setSettings({ ...settings, bind_private: v })}
                />
              </div>
            </div>

            {hasRemote && (
              <Alert>
                <ShieldAlert className="h-4 w-4" />
                <AlertTitle>Remote containers publish a port</AlertTitle>
                <AlertDescription>
                  The proxy on this host connects to each remote container on a published port.
                  Put servers on Tailscale or a private network and keep "Bind to private
                  address" on.
                  {publicServers.length > 0 && (
                    <>
                      {" "}
                      {publicServers.map((p) => p.server_name).join(", ")}{" "}
                      {publicServers.length === 1 ? "has" : "have"} no private address, so the
                      port is open on every interface: allow it only from the Rivetr host in the
                      server firewall.
                    </>
                  )}
                </AlertDescription>
              </Alert>
            )}
            {rows.length > 0 && (
              <Alert>
                <HardDrive className="h-4 w-4" />
                <AlertTitle>Volumes are per server</AlertTitle>
                <AlertDescription>
                  Each container mounts the app's volumes from its own server. Data is not
                  shared or copied between servers; use a managed database or object storage
                  for shared state. Env vars are applied on every server.
                </AlertDescription>
              </Alert>
            )}

            {data?.enabled && data.upstreams.length > 0 && (
              <div className="space-y-1.5">
                <p className="text-sm font-medium">Live upstreams</p>
                <div className="flex flex-wrap gap-2">
                  {data.upstreams.map((u) => (
                    <Badge key={u} variant="outline" className="font-mono text-xs gap-1">
                      <ServerIcon className="h-3 w-3" />
                      {u}
                    </Badge>
                  ))}
                </div>
              </div>
            )}

            <div className="flex flex-wrap justify-end gap-2">
              {data?.enabled && (
                <Button variant="outline" disabled={saving} onClick={() => save([])}>
                  Use a single host
                </Button>
              )}
              <Button disabled={saving || rows.length === 0} onClick={() => save(rows)}>
                {saving ? "Saving..." : "Save placement"}
              </Button>
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
}
