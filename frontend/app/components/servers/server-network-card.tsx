/**
 * How Rivetr reaches a remote server over SSH: its public host, or a
 * Tailscale IP / MagicDNS name. Includes a connectivity check (tailscaled on
 * the Rivetr host, tailscale ping, SSH port). Rivetr does not manage tailnet
 * keys; the Rivetr host has to be logged in to the tailnet already.
 */
import { useEffect, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { CheckCircle2, CircleSlash, Loader2, XCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { SectionCard, FormGrid, FormField } from "@/components/page";
import {
  serversApi,
  type NetworkCheckResponse,
  type Server,
  type ServerNetworkMode,
} from "@/lib/api/servers";

const TS_ADDRESS = /^[A-Za-z0-9.:-]+$/;

export function ServerNetworkCard({ server }: { server: Server }) {
  const queryClient = useQueryClient();
  const [mode, setMode] = useState<ServerNetworkMode>(server.network_mode ?? "public");
  const [tsHost, setTsHost] = useState(server.tailscale_host ?? "");
  const [result, setResult] = useState<NetworkCheckResponse | null>(null);

  useEffect(() => {
    setMode(server.network_mode ?? "public");
    setTsHost(server.tailscale_host ?? "");
  }, [server.network_mode, server.tailscale_host]);

  const tsError =
    mode === "tailscale" && !tsHost.trim()
      ? "Required for Tailscale"
      : mode === "tailscale" && (!TS_ADDRESS.test(tsHost.trim()) || tsHost.trim().startsWith("-"))
        ? "Use a Tailscale IP (100.x.y.z) or MagicDNS name"
        : null;

  const dirty =
    mode !== (server.network_mode ?? "public") || tsHost.trim() !== (server.tailscale_host ?? "");

  const save = useMutation({
    mutationFn: () =>
      serversApi.update(server.id, { network_mode: mode, tailscale_host: tsHost.trim() }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["server", server.id] });
      queryClient.invalidateQueries({ queryKey: ["servers"] });
      setResult(null);
      toast.success("Network settings saved");
    },
    onError: (e) => toast.error(e instanceof Error ? e.message : "Failed to save"),
  });

  const check = useMutation({
    mutationFn: () => serversApi.networkCheck(server.id),
    onSuccess: setResult,
    onError: (e) => toast.error(e instanceof Error ? e.message : "Check failed"),
  });

  return (
    <SectionCard
      id="network"
      title="Network"
      description="How Rivetr reaches this server over SSH."
      footer={
        <div className="flex flex-wrap gap-2">
          <Button onClick={() => save.mutate()} disabled={!dirty || !!tsError || save.isPending}>
            {save.isPending ? "Saving..." : "Save"}
          </Button>
          <Button
            variant="outline"
            onClick={() => check.mutate()}
            disabled={dirty || check.isPending}
            title={dirty ? "Save first" : undefined}
          >
            {check.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            Check connectivity
          </Button>
        </div>
      }
    >
      <div className="space-y-4">
        <FormGrid>
          <FormField label="Connect via">
            <Select value={mode} onValueChange={(v) => setMode(v as ServerNetworkMode)}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="public">Public IP / hostname ({server.host})</SelectItem>
                <SelectItem value="tailscale">Tailscale</SelectItem>
              </SelectContent>
            </Select>
          </FormField>
          {mode === "tailscale" && (
            <FormField
              label="Tailscale address"
              htmlFor="server-tailscale-host"
              error={tsError}
              hint="Tailscale IP or MagicDNS name, e.g. web-1.tail1234.ts.net"
            >
              <Input
                id="server-tailscale-host"
                value={tsHost}
                onChange={(e) => setTsHost(e.target.value)}
                placeholder="100.64.0.2"
              />
            </FormField>
          )}
        </FormGrid>
        {mode === "tailscale" && (
          <p className="text-xs text-muted-foreground">
            The Rivetr host must run tailscaled and be logged in to the same tailnet
            (<code className="font-mono">sudo tailscale up</code>). Rivetr does not manage
            tailnet keys or ACLs. See docs/reference/remote-servers.md.
          </p>
        )}
        {result && (
          <ul className="space-y-1.5 text-sm">
            {result.checks.map((c) => (
              <li key={c.name} className="flex items-start gap-2">
                {c.status === "pass" ? (
                  <CheckCircle2 className="mt-0.5 h-4 w-4 shrink-0 text-green-600" />
                ) : c.status === "fail" ? (
                  <XCircle className="mt-0.5 h-4 w-4 shrink-0 text-destructive" />
                ) : (
                  <CircleSlash className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
                )}
                <span>
                  <span className="font-medium">{c.name}</span>
                  <span className="text-muted-foreground">: {c.detail}</span>
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </SectionCard>
  );
}
