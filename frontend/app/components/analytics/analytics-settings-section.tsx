import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { SectionCard } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { analyticsApi } from "@/lib/api/analytics";
import { ServerAnalyticsToggle } from "./server-analytics-toggle";

/** Settings > Advanced: retention, optional GeoIP database and per-server switches. */
export function AnalyticsSettingsSection() {
  const queryClient = useQueryClient();
  const { data } = useQuery({
    queryKey: ["analytics-settings"],
    queryFn: analyticsApi.settings,
  });
  const [retention, setRetention] = useState("30");
  const [geoip, setGeoip] = useState("");

  useEffect(() => {
    if (data) {
      setRetention(String(data.retention_days));
      setGeoip(data.geoip_path ?? "");
    }
  }, [data]);

  const save = useMutation({
    mutationFn: () =>
      analyticsApi.updateSettings({
        retention_days: Number(retention),
        geoip_path: geoip.trim(),
      }),
    onSuccess: (saved) => {
      queryClient.setQueryData(["analytics-settings"], saved);
      toast.success("Traffic analytics settings saved");
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const dirty =
    !!data &&
    (String(data.retention_days) !== retention || (data.geoip_path ?? "") !== geoip.trim());

  return (
    <SectionCard
      id="traffic-analytics"
      title="Traffic analytics"
      description="Per-minute request rollups from the proxy. Visitor counts use salted hashes that rotate daily; raw IPs are never stored."
      footer={
        <Button size="sm" disabled={!dirty || save.isPending} onClick={() => save.mutate()}>
          Save
        </Button>
      }
    >
      <div className="grid gap-5 sm:grid-cols-2">
        <div className="space-y-2">
          <Label htmlFor="analytics-retention">Retention (days)</Label>
          <Input
            id="analytics-retention"
            type="number"
            min={1}
            max={365}
            value={retention}
            onChange={(e) => setRetention(e.target.value)}
          />
          <p className="text-xs text-muted-foreground">Older rollups are deleted every hour. 1 to 365.</p>
        </div>
        <div className="space-y-2">
          <Label htmlFor="analytics-geoip">GeoIP database path (optional)</Label>
          <Input
            id="analytics-geoip"
            value={geoip}
            onChange={(e) => setGeoip(e.target.value)}
            placeholder="/var/lib/rivetr/GeoLite2-Country.mmdb"
            className="font-mono text-xs"
          />
          <p className="text-xs text-muted-foreground">
            A MaxMind GeoLite2 or DB-IP country .mmdb file on this server. None is bundled; without it the
            country breakdown is hidden.
          </p>
          {data?.geoip_error && <p className="text-xs text-destructive">{data.geoip_error}</p>}
        </div>
      </div>
      {data && (
        <div className="mt-5 space-y-2 border-t pt-4">
          <p className="text-sm font-medium">Servers</p>
          <ul className="space-y-2">
            {data.servers.map((s) => (
              <li key={s.server_id} className="flex flex-wrap items-center justify-between gap-2">
                <span className="text-sm">{s.name}</span>
                <ServerAnalyticsToggle serverId={s.server_id} enabled={s.enabled} />
              </li>
            ))}
          </ul>
        </div>
      )}
    </SectionCard>
  );
}
