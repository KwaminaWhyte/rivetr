import { useState, useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";

export function meta() {
  return [
    { title: "Webhook Events - Rivetr" },
    { name: "description", content: "View incoming webhook event history and payloads" },
  ];
}
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { systemApi } from "@/lib/api/system";
import type { WebhookEvent } from "@/types/api";
import { RefreshCw, Webhook } from "lucide-react";

const PROVIDERS = ["github", "gitlab", "gitea", "bitbucket", "dockerhub"] as const;
const STATUSES = ["processed", "ignored", "error"] as const;

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString();
}

function formatBytes(bytes: number | null): string {
  if (bytes === null) return "-";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

function getProviderColor(
  provider: string
): "default" | "secondary" | "outline" {
  switch (provider) {
    case "github":
      return "default";
    case "gitlab":
      return "secondary";
    default:
      return "outline";
  }
}

function getStatusVariant(
  status: WebhookEvent["status"]
): "default" | "secondary" | "destructive" | "outline" {
  switch (status) {
    case "processed":
      return "default";
    case "ignored":
      return "secondary";
    case "error":
      return "destructive";
    default:
      return "outline";
  }
}

export default function SettingsWebhookEventsPage() {
  const [providerFilter, setProviderFilter] = useState<string>("all");
  const [statusFilter, setStatusFilter] = useState<string>("all");

  const { data: events = [], refetch, isLoading } = useQuery<WebhookEvent[]>({
    queryKey: ["webhookEvents", providerFilter, statusFilter],
    queryFn: () =>
      systemApi.listWebhookEvents({
        provider: providerFilter !== "all" ? providerFilter : undefined,
        status: statusFilter !== "all" ? statusFilter : undefined,
        limit: 100,
      }),
    refetchInterval: 30_000,
  });

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Webhook Events</h1>
          <p className="text-muted-foreground">
            Audit log of all incoming webhook events from Git providers
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => refetch()}
          disabled={isLoading}
        >
          <RefreshCw
            className={`h-4 w-4 mr-2 ${isLoading ? "animate-spin" : ""}`}
          />
          Refresh
        </Button>
      </div>

      {/* Filters */}
      <div className="flex items-center gap-4">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">Provider:</span>
          <Select value={providerFilter} onValueChange={setProviderFilter}>
            <SelectTrigger className="w-[140px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Providers</SelectItem>
              {PROVIDERS.map((p) => (
                <SelectItem key={p} value={p}>
                  {p.charAt(0).toUpperCase() + p.slice(1)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">Status:</span>
          <Select value={statusFilter} onValueChange={setStatusFilter}>
            <SelectTrigger className="w-[130px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Statuses</SelectItem>
              {STATUSES.map((s) => (
                <SelectItem key={s} value={s}>
                  {s.charAt(0).toUpperCase() + s.slice(1)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Recent Events</CardTitle>
          <CardDescription>
            Last {events.length} events — auto-refreshes every 30s
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-4">
              {Array.from({ length: 5 }).map((_, i) => (
                <Skeleton key={i} className="h-12 w-full" />
              ))}
            </div>
          ) : events.length === 0 ? (
            <div className="text-center py-12">
              <Webhook className="mx-auto h-12 w-12 text-muted-foreground/50" />
              <p className="mt-4 text-muted-foreground">
                No webhook events found.
              </p>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Received</TableHead>
                  <TableHead>Provider</TableHead>
                  <TableHead>Event</TableHead>
                  <TableHead>Repository</TableHead>
                  <TableHead>Branch</TableHead>
                  <TableHead>Apps Triggered</TableHead>
                  <TableHead>Size</TableHead>
                  <TableHead>Status</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {events.map((event) => (
                  <TableRow key={event.id}>
                    <TableCell className="whitespace-nowrap text-sm">
                      {formatDate(event.received_at)}
                    </TableCell>
                    <TableCell>
                      <Badge variant={getProviderColor(event.provider)}>
                        {event.provider}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <code className="text-xs bg-muted px-1 py-0.5 rounded">
                        {event.event_type}
                      </code>
                    </TableCell>
                    <TableCell className="max-w-[200px] truncate text-sm">
                      {event.repository ?? "-"}
                    </TableCell>
                    <TableCell className="text-sm">
                      {event.branch ?? "-"}
                    </TableCell>
                    <TableCell className="text-center">
                      <span
                        className={
                          event.apps_triggered > 0
                            ? "font-semibold text-green-600"
                            : "text-muted-foreground"
                        }
                      >
                        {event.apps_triggered}
                      </span>
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {formatBytes(event.payload_size)}
                    </TableCell>
                    <TableCell>
                      <Badge variant={getStatusVariant(event.status)}>
                        {event.status}
                      </Badge>
                      {event.error_message && (
                        <p className="text-xs text-destructive mt-1 max-w-[200px] truncate">
                          {event.error_message}
                        </p>
                      )}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
