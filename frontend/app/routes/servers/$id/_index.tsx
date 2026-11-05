import { Link } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { Layers, ShieldCheck, Sparkles } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { serversApi } from "@/lib/api/servers";
import { relativeTime } from "@/components/servers/server-ops-utils";
import { useServerContext } from "@/components/servers/server-context";

export function meta() {
  return [{ title: "Server - Rivetr" }];
}

function Row({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="grid grid-cols-3 gap-4 py-2 text-sm">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="col-span-2 break-words">{value ?? "-"}</dd>
    </div>
  );
}

export default function ServerGeneralPage() {
  const { serverId, isLocal, summary } = useServerContext();
  const { data: server } = useQuery({
    queryKey: ["server", serverId],
    queryFn: () => serversApi.get(serverId),
    enabled: !isLocal,
  });

  const links = [
    { to: "images", icon: Layers, title: "Images", text: "See which images take space and remove unused ones." },
    { to: "docker-cleanup", icon: Sparkles, title: "Docker Cleanup", text: "Schedule cleanups and review past runs." },
    { to: "security", icon: ShieldCheck, title: "Security", text: "Package updates, automatic security updates and reboots." },
  ];

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>General</CardTitle>
        </CardHeader>
        <CardContent>
          <dl className="divide-y">
            {isLocal ? (
              <Row label="Runtime" value="Local container runtime used by Rivetr" />
            ) : (
              <>
                <Row label="Host" value={server ? `${server.username}@${server.host}:${server.port}` : summary?.host} />
                <Row label="Operating system" value={server?.os_info} />
                <Row label="Docker" value={server?.docker_version} />
                <Row label="Last seen" value={server?.last_seen_at ? relativeTime(server.last_seen_at) : "never"} />
              </>
            )}
            <Row label="Resources" value={summary?.resource_count ?? "-"} />
            <Row
              label="Patches"
              value={
                summary ? (
                  <span className="flex flex-wrap items-center gap-2">
                    {summary.last_patch_check_at ? (
                      <>
                        <Badge variant="outline">{summary.update_count} updates</Badge>
                        {summary.reboot_required && (
                          <Badge variant="destructive">Reboot required</Badge>
                        )}
                        <span className="text-xs text-muted-foreground">
                          checked {relativeTime(summary.last_patch_check_at)}
                        </span>
                      </>
                    ) : (
                      <span className="text-muted-foreground">Not checked yet</span>
                    )}
                  </span>
                ) : (
                  "-"
                )
              }
            />
          </dl>
        </CardContent>
      </Card>

      <div className="grid gap-4 sm:grid-cols-3">
        {links.map((l) => (
          <Link key={l.to} to={l.to} className="group">
            <Card className="h-full transition-colors group-hover:border-foreground/20">
              <CardContent className="space-y-1 pt-6">
                <div className="flex items-center gap-2 font-medium">
                  <l.icon className="h-4 w-4" />
                  {l.title}
                </div>
                <p className="text-sm text-muted-foreground">{l.text}</p>
              </CardContent>
            </Card>
          </Link>
        ))}
      </div>
    </div>
  );
}
