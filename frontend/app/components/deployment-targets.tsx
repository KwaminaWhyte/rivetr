import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { ChevronDown, ChevronRight, Server as ServerIcon } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { placementApi, type DeploymentTarget } from "@/lib/api/placement";

const STATUS_VARIANT: Record<string, "default" | "secondary" | "destructive" | "outline"> = {
  healthy: "default",
  failed: "destructive",
  rolled_back: "outline",
  replaced: "secondary",
  stopped: "secondary",
};

const STATUS_LABEL: Record<string, string> = {
  pending: "Pending",
  distributing: "Receiving image",
  starting: "Starting",
  checking: "Health checking",
  healthy: "Healthy",
  failed: "Failed",
  rolled_back: "Rolled back",
  replaced: "Replaced",
  stopped: "Stopped",
};

function TargetRow({ target }: { target: DeploymentTarget }) {
  const [open, setOpen] = useState(target.status === "failed");
  return (
    <div className="rounded-md border">
      <button
        type="button"
        className="flex w-full flex-wrap items-center gap-3 px-3 py-2 text-left text-sm"
        onClick={() => setOpen(!open)}
        aria-expanded={open}
      >
        {open ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
        <ServerIcon className="h-4 w-4 text-muted-foreground" />
        <span className="font-medium">{target.server_name || target.server_id}</span>
        <span className="text-muted-foreground">#{target.replica_index}</span>
        <span className="text-xs text-muted-foreground">batch {target.batch + 1}</span>
        <Badge variant={STATUS_VARIANT[target.status] ?? "secondary"} className="text-xs">
          {STATUS_LABEL[target.status] ?? target.status}
        </Badge>
        {target.upstream && (
          <span className="ml-auto font-mono text-xs text-muted-foreground">{target.upstream}</span>
        )}
      </button>
      {open && (
        <div className="border-t px-3 py-2 space-y-2">
          {target.error_message && (
            <p className="text-sm text-destructive break-words">{target.error_message}</p>
          )}
          {target.container_name && (
            <p className="text-xs text-muted-foreground font-mono break-all">
              {target.container_name}
            </p>
          )}
          <pre className="max-h-64 overflow-auto rounded bg-muted p-2 text-xs whitespace-pre-wrap break-words">
            {target.log || "No log lines yet."}
          </pre>
        </div>
      )}
    </div>
  );
}

/** Per-server status and logs of a multi-server deployment. Renders nothing for single-host deploys. */
export function DeploymentTargets({
  deploymentId,
  isActive,
}: {
  deploymentId: string;
  isActive: boolean;
}) {
  const { data: targets = [] } = useQuery({
    queryKey: ["deployment-targets", deploymentId],
    queryFn: () => placementApi.targets(deploymentId),
    refetchInterval: isActive ? 2000 : false,
  });
  if (targets.length === 0) return null;
  const healthy = targets.filter((t) => t.status === "healthy").length;
  const servers = new Set(targets.map((t) => t.server_id)).size;
  return (
    <Card>
      <CardHeader className="pb-3">
        <CardTitle className="text-base">
          Servers ({servers}) · {healthy} of {targets.length} containers healthy
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-2">
        {targets.map((t) => (
          <TargetRow key={t.id} target={t} />
        ))}
      </CardContent>
    </Card>
  );
}
