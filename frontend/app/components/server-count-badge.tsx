import { useQuery } from "@tanstack/react-query";
import { Server as ServerIcon } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { placementApi } from "@/lib/api/placement";
import { cn } from "@/lib/utils";

/** `{app_id: server_count}` for apps placed on several servers (one shared query). */
export function usePlacementCounts() {
  return useQuery({
    queryKey: ["placement-counts"],
    queryFn: () => placementApi.counts(),
    staleTime: 30_000,
  });
}

/** "N servers" for an app with a multi-server placement; nothing otherwise. */
export function ServerCountBadge({ appId, compact = false }: { appId: string; compact?: boolean }) {
  const { data } = usePlacementCounts();
  const n = data?.[appId];
  if (!n) return null;
  const label = `${n} ${n === 1 ? "server" : "servers"}`;
  if (compact) {
    return (
      <span
        className="inline-flex shrink-0 items-center gap-1 text-[11px] text-muted-foreground"
        title={`Runs on ${label}`}
      >
        <ServerIcon className="size-3" />
        {n}
      </span>
    );
  }
  return (
    <Badge variant="outline" className={cn("gap-1")} title={`Runs on ${label} behind the proxy`}>
      <ServerIcon className="h-3 w-3" />
      {label}
    </Badge>
  );
}
