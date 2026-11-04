import { useState } from "react";
import { useOutletContext } from "react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { ContainerTerminal } from "@/components/container-terminal";
import { api } from "@/lib/api";
import type { Service, ServiceContainer, ServiceContainerLogLine } from "@/types/api";
import { Boxes, FileText, RotateCw, SquareTerminal } from "lucide-react";

export function meta() {
  return [
    { title: "Service Containers - Rivetr" },
    { name: "description", content: "Containers of this service" },
  ];
}

interface OutletContext {
  service: Service;
}

function formatBytes(n: number | null | undefined): string {
  if (n == null) return "-";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let v = n;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v >= 100 || i === 0 ? 0 : 1)} ${units[i]}`;
}

function StateBadge({ c }: { c: ServiceContainer }) {
  if (c.state === "running") {
    return <Badge className="bg-green-600 text-white">running</Badge>;
  }
  if (c.state === "restarting") {
    return <Badge className="bg-amber-500 text-white">restarting</Badge>;
  }
  return (
    <Badge variant="secondary">
      {c.state}
      {c.state === "exited" ? ` (${c.exit_code})` : ""}
    </Badge>
  );
}

function HealthBadge({ health }: { health: string }) {
  if (!health) return <span className="text-xs text-muted-foreground">no check</span>;
  if (health === "healthy") {
    return (
      <Badge variant="outline" className="border-green-500 text-green-700 dark:text-green-400">
        healthy
      </Badge>
    );
  }
  if (health === "unhealthy") return <Badge variant="destructive">unhealthy</Badge>;
  return <Badge variant="outline">{health}</Badge>;
}

function LogsDialog({
  service,
  container,
  onClose,
}: {
  service: Service;
  container: ServiceContainer;
  onClose: () => void;
}) {
  const {
    data: lines = [],
    isFetching,
    refetch,
  } = useQuery<ServiceContainerLogLine[]>({
    queryKey: ["service-container-logs", service.id, container.id],
    queryFn: () => api.getServiceContainerLogs(service.id, container.id, 500),
  });
  return (
    <Dialog open onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="sm:max-w-5xl">
        <DialogHeader>
          <DialogTitle className="break-all">{container.name}</DialogTitle>
          <DialogDescription>Last 500 log lines.</DialogDescription>
        </DialogHeader>
        <div className="flex justify-end">
          <Button variant="outline" size="sm" onClick={() => refetch()} disabled={isFetching}>
            {isFetching ? "Loading..." : "Refresh"}
          </Button>
        </div>
        <div className="max-h-[60vh] overflow-auto rounded-lg bg-gray-900 p-3 font-mono text-xs text-gray-200">
          {lines.length === 0 ? (
            <div className="py-4 text-center text-gray-500">No logs</div>
          ) : (
            lines.map((l, i) => (
              <div key={i} className="flex gap-2">
                <span className="shrink-0 text-gray-500">
                  {l.timestamp ? new Date(l.timestamp).toLocaleTimeString() : ""}
                </span>
                <span
                  className={`whitespace-pre-wrap break-all ${
                    l.stream === "stderr" ? "text-amber-300" : ""
                  }`}
                >
                  {l.message}
                </span>
              </div>
            ))
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}

function TerminalDialog({
  service,
  container,
  onClose,
}: {
  service: Service;
  container: ServiceContainer;
  onClose: () => void;
}) {
  return (
    <Dialog open onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="sm:max-w-5xl">
        <DialogHeader>
          <DialogTitle className="break-all">Terminal: {container.name}</DialogTitle>
          <DialogDescription>/bin/sh inside the container.</DialogDescription>
        </DialogHeader>
        <ContainerTerminal
          key={container.id}
          wsUrlFor={(token) =>
            api.getServiceContainerTerminalWsUrl(service.id, container.id, token)
          }
        />
      </DialogContent>
    </Dialog>
  );
}

export default function ServiceContainersTab() {
  const { service } = useOutletContext<OutletContext>();
  const queryClient = useQueryClient();
  const [logsFor, setLogsFor] = useState<ServiceContainer | null>(null);
  const [terminalFor, setTerminalFor] = useState<ServiceContainer | null>(null);

  const {
    data: containers = [],
    isLoading,
    error,
  } = useQuery<ServiceContainer[]>({
    queryKey: ["service-containers", service.id],
    queryFn: () => api.getServiceContainers(service.id),
    refetchInterval: 5000,
  });

  const restartMutation = useMutation({
    mutationFn: (c: ServiceContainer) => api.restartServiceContainer(service.id, c.id),
    onSuccess: (_, c) => {
      toast.success(`Restarted ${c.name}`);
      queryClient.invalidateQueries({ queryKey: ["service-containers", service.id] });
    },
    onError: (e) => toast.error(e instanceof Error ? e.message : "Restart failed"),
  });

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Boxes className="h-5 w-5" />
            Containers
          </CardTitle>
          <CardDescription>
            Every container of this compose project, refreshed every 5 seconds.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="h-24 animate-pulse rounded bg-muted" />
          ) : error ? (
            <p className="text-sm text-destructive">
              {error instanceof Error ? error.message : "Could not list containers"}
            </p>
          ) : containers.length === 0 ? (
            <p className="py-8 text-center text-sm text-muted-foreground">
              No containers. Start or deploy the service first.
            </p>
          ) : (
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Service</TableHead>
                    <TableHead>State</TableHead>
                    <TableHead>Health</TableHead>
                    <TableHead>CPU</TableHead>
                    <TableHead>Memory</TableHead>
                    <TableHead>Ports</TableHead>
                    <TableHead className="text-right">Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {containers.map((c) => (
                    <TableRow key={c.id}>
                      <TableCell>
                        <div className="font-medium">{c.service}</div>
                        <div className="break-all font-mono text-xs text-muted-foreground">
                          {c.name}
                        </div>
                        <div className="break-all font-mono text-xs text-muted-foreground">
                          {c.image}
                        </div>
                      </TableCell>
                      <TableCell>
                        <StateBadge c={c} />
                        <div className="mt-1 text-xs text-muted-foreground">{c.status}</div>
                      </TableCell>
                      <TableCell>
                        <HealthBadge health={c.health} />
                      </TableCell>
                      <TableCell className="text-sm">
                        {c.cpu_percent == null ? "-" : `${c.cpu_percent.toFixed(1)}%`}
                      </TableCell>
                      <TableCell className="whitespace-nowrap text-sm">
                        {formatBytes(c.memory_usage)}
                        {c.memory_limit ? (
                          <span className="text-muted-foreground">
                            {" "}
                            / {formatBytes(c.memory_limit)}
                          </span>
                        ) : null}
                      </TableCell>
                      <TableCell className="font-mono text-xs">
                        {c.ports
                          .filter((p) => p.PublishedPort)
                          .map((p) => `${p.URL || "0.0.0.0"}:${p.PublishedPort}->${p.TargetPort}`)
                          .join(", ") || "-"}
                      </TableCell>
                      <TableCell>
                        <div className="flex justify-end gap-1">
                          <Button
                            variant="ghost"
                            size="sm"
                            title="Logs"
                            aria-label={`Logs of ${c.name}`}
                            onClick={() => setLogsFor(c)}
                          >
                            <FileText className="h-4 w-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            title="Terminal"
                            aria-label={`Terminal in ${c.name}`}
                            disabled={c.state !== "running"}
                            onClick={() => setTerminalFor(c)}
                          >
                            <SquareTerminal className="h-4 w-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            title="Restart"
                            aria-label={`Restart ${c.name}`}
                            disabled={restartMutation.isPending}
                            onClick={() => {
                              if (window.confirm(`Restart ${c.name}?`)) restartMutation.mutate(c);
                            }}
                          >
                            <RotateCw className="h-4 w-4" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </CardContent>
      </Card>
      {logsFor && (
        <LogsDialog service={service} container={logsFor} onClose={() => setLogsFor(null)} />
      )}
      {terminalFor && (
        <TerminalDialog
          service={service}
          container={terminalFor}
          onClose={() => setTerminalFor(null)}
        />
      )}
    </>
  );
}
