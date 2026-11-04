import { useOutletContext, useSearchParams } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { DeploymentLogs, type LogEntry } from "@/components/deployment-logs";
import { api } from "@/lib/api";
import {
  ACTIVE_SERVICE_DEPLOYMENT_STATUSES,
  type Service,
  type ServiceDeployment,
  type ServiceImageState,
} from "@/types/api";
import { ArrowLeft, History } from "lucide-react";

export function meta() {
  return [
    { title: "Service Deployments - Rivetr" },
    { name: "description", content: "Deployment history and logs for this service" },
  ];
}

interface OutletContext {
  service: Service;
}

export function DeploymentStatusBadge({ status }: { status: string }) {
  switch (status) {
    case "succeeded":
      return <Badge className="bg-green-600 text-white">Succeeded</Badge>;
    case "failed":
      return <Badge variant="destructive">Failed</Badge>;
    case "building":
      return <Badge className="bg-amber-500 text-white">Building</Badge>;
    case "deploying":
      return <Badge className="bg-blue-600 text-white">Deploying</Badge>;
    default:
      return <Badge variant="outline">Pending</Badge>;
  }
}

function formatDuration(ms: number | null): string {
  if (ms == null) return "-";
  const s = Math.round(ms / 1000);
  if (s < 60) return `${s}s`;
  return `${Math.floor(s / 60)}m ${s % 60}s`;
}

function formatWhen(ts: string): string {
  const d = new Date(ts);
  return Number.isNaN(d.getTime()) ? ts : d.toLocaleString();
}

function shortDigest(state?: ServiceImageState): string {
  if (!state) return "-";
  const ref = state.digest?.split("@")[1] ?? state.image_id;
  if (!ref) return "unknown";
  return ref.replace("sha256:", "").slice(0, 12);
}

function isActive(d: ServiceDeployment) {
  return ACTIVE_SERVICE_DEPLOYMENT_STATUSES.includes(d.status);
}

function ImageChanges({ deployment }: { deployment: ServiceDeployment }) {
  const names = Array.from(
    new Set([
      ...Object.keys(deployment.images_before ?? {}),
      ...Object.keys(deployment.images_after ?? {}),
    ])
  ).sort();
  if (names.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        No image information was recorded for this deployment.
      </p>
    );
  }
  return (
    <div className="overflow-x-auto">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Compose service</TableHead>
            <TableHead>Image</TableHead>
            <TableHead>Before</TableHead>
            <TableHead>After</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {names.map((name) => {
            const before = deployment.images_before?.[name];
            const after = deployment.images_after?.[name];
            const changed =
              !!after && (before?.image_id ?? null) !== (after.image_id ?? null);
            return (
              <TableRow key={name}>
                <TableCell className="font-medium">{name}</TableCell>
                <TableCell className="font-mono text-xs break-all">
                  {(after ?? before)?.image}
                </TableCell>
                <TableCell className="font-mono text-xs" title={before?.digest ?? before?.image_id}>
                  {shortDigest(before)}
                </TableCell>
                <TableCell className="font-mono text-xs" title={after?.digest ?? after?.image_id}>
                  <span className={changed ? "text-green-600 font-semibold" : undefined}>
                    {shortDigest(after)}
                  </span>
                  {changed && (
                    <Badge variant="outline" className="ml-2 text-[10px]">
                      updated
                    </Badge>
                  )}
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </div>
  );
}

function DeploymentDetail({
  service,
  deploymentId,
  onBack,
}: {
  service: Service;
  deploymentId: string;
  onBack: () => void;
}) {
  const { data: deployment } = useQuery<ServiceDeployment>({
    queryKey: ["service-deployment", service.id, deploymentId],
    queryFn: () => api.getServiceDeployment(service.id, deploymentId),
    refetchInterval: (q) => (q.state.data && !isActive(q.state.data) ? false : 2000),
  });

  if (!deployment) {
    return <div className="h-32 animate-pulse rounded-lg bg-muted" />;
  }

  const active = isActive(deployment);

  return (
    <div className="space-y-4">
      <Button variant="ghost" size="sm" className="gap-2" onClick={onBack}>
        <ArrowLeft className="h-4 w-4" />
        All deployments
      </Button>
      <Card>
        <CardHeader>
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="space-y-1">
              <CardTitle className="flex flex-wrap items-center gap-2">
                Deployment
                <span className="font-mono text-sm text-muted-foreground">
                  {deployment.id.slice(0, 8)}
                </span>
                <DeploymentStatusBadge status={deployment.status} />
              </CardTitle>
              <CardDescription>
                {deployment.trigger}
                {deployment.triggered_by_name ? ` by ${deployment.triggered_by_name}` : ""}
                {" - "}
                {formatWhen(deployment.started_at)}
                {" - "}
                {formatDuration(deployment.duration_ms)}
              </CardDescription>
            </div>
            {/* ROLLBACK_SLOT */}
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          {(deployment.commit_sha || deployment.image_tag) && (
            <div className="grid gap-1 text-sm">
              {deployment.commit_sha && (
                <div>
                  <span className="text-muted-foreground">Commit: </span>
                  <span className="font-mono">{deployment.commit_sha.slice(0, 12)}</span>
                  {deployment.commit_message && (
                    <span className="text-muted-foreground"> {deployment.commit_message}</span>
                  )}
                </div>
              )}
              {deployment.image_tag && (
                <div>
                  <span className="text-muted-foreground">Image tag: </span>
                  <span className="font-mono">{deployment.image_tag}</span>
                </div>
              )}
            </div>
          )}
          {deployment.rollback_of && (
            <p className="text-sm text-muted-foreground">
              Rollback to deployment{" "}
              <span className="font-mono">{deployment.rollback_of.slice(0, 8)}</span>
            </p>
          )}
          {deployment.error_message && (
            <pre className="whitespace-pre-wrap rounded-md bg-destructive/10 p-3 text-sm text-destructive">
              {deployment.error_message}
            </pre>
          )}
          <ImageChanges deployment={deployment} />
        </CardContent>
      </Card>
      <DeploymentLogs
        key={deployment.id}
        deploymentId={deployment.id}
        isActive={active}
        title="Deployment Logs"
        streamPath={`/services/${service.id}/deployments/${deployment.id}/logs/stream`}
        loadLogs={() =>
          api.getServiceDeploymentLogs(service.id, deployment.id) as Promise<LogEntry[]>
        }
      />
    </div>
  );
}

export default function ServiceDeploymentsTab() {
  const { service } = useOutletContext<OutletContext>();
  const [params, setParams] = useSearchParams();
  const selected = params.get("deployment");

  const { data: deployments = [], isLoading } = useQuery<ServiceDeployment[]>({
    queryKey: ["service-deployments", service.id],
    queryFn: () => api.getServiceDeployments(service.id),
    refetchInterval: (q) => (q.state.data?.some(isActive) ? 2000 : 15000),
  });

  if (selected) {
    return (
      <DeploymentDetail
        service={service}
        deploymentId={selected}
        onBack={() => setParams({})}
      />
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <History className="h-5 w-5" />
          Deployments
        </CardTitle>
        <CardDescription>
          Each deploy pulls (or builds) the images and runs <code>docker compose up -d</code>,
          which recreates only the containers that changed.
        </CardDescription>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="h-24 animate-pulse rounded bg-muted" />
        ) : deployments.length === 0 ? (
          <p className="py-8 text-center text-sm text-muted-foreground">
            No deployments yet. Use Deploy in the header to run the first one.
          </p>
        ) : (
          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Status</TableHead>
                  <TableHead>Trigger</TableHead>
                  <TableHead>Started</TableHead>
                  <TableHead>Duration</TableHead>
                  <TableHead>Version</TableHead>
                  <TableHead>Changed</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {deployments.map((d) => {
                  const changed = Object.entries(d.images_after ?? {}).filter(
                    ([name, after]) => d.images_before?.[name]?.image_id !== after.image_id
                  );
                  return (
                    <TableRow
                      key={d.id}
                      className="cursor-pointer"
                      onClick={() => setParams({ deployment: d.id })}
                    >
                      <TableCell>
                        <DeploymentStatusBadge status={d.status} />
                      </TableCell>
                      <TableCell className="text-sm">
                        <div className="capitalize">{d.trigger}</div>
                        {d.triggered_by_name && (
                          <div className="text-xs text-muted-foreground">
                            {d.triggered_by_name}
                          </div>
                        )}
                      </TableCell>
                      <TableCell className="whitespace-nowrap text-sm">
                        {formatWhen(d.started_at)}
                      </TableCell>
                      <TableCell className="text-sm">{formatDuration(d.duration_ms)}</TableCell>
                      <TableCell className="font-mono text-xs">
                        {d.image_tag ?? d.commit_sha?.slice(0, 7) ?? "-"}
                      </TableCell>
                      <TableCell className="text-sm">
                        {changed.length === 0 ? (
                          <span className="text-muted-foreground">-</span>
                        ) : (
                          changed.map(([name]) => name).join(", ")
                        )}
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
