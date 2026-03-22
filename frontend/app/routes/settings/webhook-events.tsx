import { useState } from "react";
import { useSearchParams } from "react-router";
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
import type { WebhookEvent, WebhookEventListResponse } from "@/types/api";
import { ChevronLeft, ChevronRight, RefreshCw, Webhook } from "lucide-react";

const PROVIDERS = ["github", "gitlab", "gitea", "bitbucket", "dockerhub"] as const;
const STATUSES = ["processed", "ignored", "error"] as const;
const PER_PAGE = 50;

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
  const [searchParams, setSearchParams] = useSearchParams();

  const providerFilter = searchParams.get("provider") || "all";
  const statusFilter = searchParams.get("status") || "all";
  const page = Math.max(1, parseInt(searchParams.get("page") || "1"));

  const updateParams = (updates: Record<string, string | undefined>) => {
    const newParams = new URLSearchParams(searchParams);
    Object.entries(updates).forEach(([key, value]) => {
      if (value !== undefined && value !== "") {
        newParams.set(key, value);
      } else {
        newParams.delete(key);
      }
    });
    setSearchParams(newParams);
  };

  const handleFilterChange = (key: string, value: string) => {
    updateParams({ [key]: value === "all" ? undefined : value, page: undefined });
  };

  const goToPage = (newPage: number) => {
    updateParams({ page: newPage > 1 ? String(newPage) : undefined });
  };

  const { data, refetch, isLoading } = useQuery<WebhookEventListResponse>({
    queryKey: ["webhookEvents", providerFilter, statusFilter, page],
    queryFn: () =>
      systemApi.listWebhookEvents({
        provider: providerFilter !== "all" ? providerFilter : undefined,
        status: statusFilter !== "all" ? statusFilter : undefined,
        page,
        per_page: PER_PAGE,
      }),
    refetchInterval: 30_000,
  });

  const events = data?.items ?? [];
  const total = data?.total ?? 0;
  const totalPages = data?.total_pages ?? 1;
  const currentPage = data?.page ?? page;

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
          <Select
            value={providerFilter}
            onValueChange={(v) => handleFilterChange("provider", v)}
          >
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
          <Select
            value={statusFilter}
            onValueChange={(v) => handleFilterChange("status", v)}
          >
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
            {total > 0
              ? `Showing ${events.length} of ${total} events — auto-refreshes every 30s`
              : "Auto-refreshes every 30s"}
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
            <>
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

              {/* Pagination */}
              {totalPages > 1 && (
                <div className="flex items-center justify-between mt-4 pt-4 border-t">
                  <div className="text-sm text-muted-foreground">
                    Page {currentPage} of {totalPages}
                  </div>
                  <div className="flex items-center gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => goToPage(currentPage - 1)}
                      disabled={currentPage <= 1}
                    >
                      <ChevronLeft className="h-4 w-4" />
                      Previous
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => goToPage(currentPage + 1)}
                      disabled={currentPage >= totalPages}
                    >
                      Next
                      <ChevronRight className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              )}
            </>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
