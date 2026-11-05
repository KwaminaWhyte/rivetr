import { Link, Outlet, useParams } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { ArrowLeft, Container } from "lucide-react";
import { ResourceSection } from "@/components/shell/resource-section";
import { serverOpsApi, LOCALHOST_SERVER_ID } from "@/lib/api/server-ops";
import type { ServerContext } from "@/components/servers/server-context";
import { ServerStatusPill } from "@/components/servers/server-status-pill";

export function meta() {
  return [
    { title: "Server - Rivetr" },
    { name: "description", content: "Server images, Docker cleanup and patching" },
  ];
}

export default function ServerLayout() {
  const { id = "" } = useParams<{ id: string }>();
  const isLocal = id === LOCALHOST_SERVER_ID;

  const { data: summaries } = useQuery({
    queryKey: ["server-ops-summary"],
    queryFn: () => serverOpsApi.summary(),
  });
  const summary = summaries?.find((s) => s.server_id === id);
  const name = summary?.name ?? (isLocal ? "localhost" : "Server");

  const basePath = `/servers/${id}`;

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="space-y-1">
          <Link
            to="/servers"
            className="inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground md:hidden"
          >
            <ArrowLeft className="h-3 w-3" />
            Servers
          </Link>
          <div className="flex items-center gap-3">
            <Container className="h-6 w-6 text-muted-foreground" />
            <h1 className="text-2xl font-semibold tracking-tight">{name}</h1>
            {summary && <ServerStatusPill status={summary.status} />}
          </div>
          <p className="text-sm text-muted-foreground">
            {isLocal ? "The machine this Rivetr instance runs on" : summary?.host}
          </p>
        </div>
      </div>

      <ResourceSection match={{ kind: "server", id, base: basePath }} />

      <div>
        <Outlet context={{ serverId: id, isLocal, name, summary } satisfies ServerContext} />
      </div>
    </div>
  );
}
