import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import type { Route } from "./+types/audit";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
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
import { api } from "@/lib/api";
import type { AuditLogListResponse, AuditLogQuery } from "@/types/api";
import {
  ChevronLeft,
  ChevronRight,
  History,
  Filter,
  RefreshCw,
} from "lucide-react";

function formatDate(dateStr: string): string {
  const date = new Date(dateStr);
  return date.toLocaleString();
}

function getActionBadgeVariant(
  action: string
): "default" | "secondary" | "destructive" | "outline" {
  if (action.includes("delete")) return "destructive";
  if (action.includes("create")) return "default";
  if (action.includes("update")) return "secondary";
  return "outline";
}

function formatAction(action: string): string {
  // Convert "app.create" to "App Create"
  return action
    .split(".")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function formatResourceType(resourceType: string): string {
  return resourceType.charAt(0).toUpperCase() + resourceType.slice(1);
}

export async function loader({ request }: Route.LoaderArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);

  // Fetch initial data
  const [auditLogs, actionTypes, resourceTypes] = await Promise.all([
    api.getAuditLogs({}, token).catch(() => ({
      items: [],
      total: 0,
      page: 1,
      per_page: 50,
      total_pages: 0,
    })),
    api.getAuditActionTypes(token).catch(() => []),
    api.getAuditResourceTypes(token).catch(() => []),
  ]);

  return { auditLogs, actionTypes, resourceTypes, token };
}

export default function SettingsAuditPage({
  loaderData,
}: Route.ComponentProps) {
  const [query, setQuery] = useState<AuditLogQuery>({
    page: 1,
    per_page: 50,
  });
  const [showFilters, setShowFilters] = useState(false);

  const { data: auditLogs, refetch, isLoading } = useQuery<AuditLogListResponse>({
    queryKey: ["auditLogs", query],
    queryFn: () => api.getAuditLogs(query, loaderData.token),
    initialData: loaderData.auditLogs,
  });

  const { data: actionTypes = [] } = useQuery<string[]>({
    queryKey: ["auditActionTypes"],
    queryFn: () => api.getAuditActionTypes(loaderData.token),
    initialData: loaderData.actionTypes,
  });

  const { data: resourceTypes = [] } = useQuery<string[]>({
    queryKey: ["auditResourceTypes"],
    queryFn: () => api.getAuditResourceTypes(loaderData.token),
    initialData: loaderData.resourceTypes,
  });

  const handleFilterChange = (key: keyof AuditLogQuery, value: string) => {
    setQuery((prev) => ({
      ...prev,
      [key]: value || undefined,
      page: 1, // Reset to first page when filtering
    }));
  };

  const clearFilters = () => {
    setQuery({ page: 1, per_page: 50 });
  };

  const goToPage = (page: number) => {
    setQuery((prev) => ({ ...prev, page }));
  };

  const hasFilters = query.action || query.resource_type || query.start_date || query.end_date;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Audit Log</h1>
          <p className="text-muted-foreground">
            View a history of all actions performed in your Rivetr instance
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => setShowFilters(!showFilters)}
          >
            <Filter className="h-4 w-4 mr-2" />
            Filters
            {hasFilters && (
              <Badge variant="secondary" className="ml-2">
                Active
              </Badge>
            )}
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => refetch()}
            disabled={isLoading}
          >
            <RefreshCw className={`h-4 w-4 mr-2 ${isLoading ? "animate-spin" : ""}`} />
            Refresh
          </Button>
        </div>
      </div>

      {/* Filters Panel */}
      {showFilters && (
        <Card>
          <CardHeader className="pb-4">
            <CardTitle className="text-lg">Filters</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
              <div className="space-y-2">
                <Label>Action Type</Label>
                <Select
                  value={query.action || ""}
                  onValueChange={(v) => handleFilterChange("action", v)}
                >
                  <SelectTrigger>
                    <SelectValue placeholder="All actions" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="">All actions</SelectItem>
                    {actionTypes.map((action) => (
                      <SelectItem key={action} value={action}>
                        {formatAction(action)}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2">
                <Label>Resource Type</Label>
                <Select
                  value={query.resource_type || ""}
                  onValueChange={(v) => handleFilterChange("resource_type", v)}
                >
                  <SelectTrigger>
                    <SelectValue placeholder="All resources" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="">All resources</SelectItem>
                    {resourceTypes.map((type) => (
                      <SelectItem key={type} value={type}>
                        {formatResourceType(type)}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2">
                <Label>Start Date</Label>
                <Input
                  type="datetime-local"
                  value={query.start_date || ""}
                  onChange={(e) => handleFilterChange("start_date", e.target.value)}
                />
              </div>

              <div className="space-y-2">
                <Label>End Date</Label>
                <Input
                  type="datetime-local"
                  value={query.end_date || ""}
                  onChange={(e) => handleFilterChange("end_date", e.target.value)}
                />
              </div>
            </div>
            {hasFilters && (
              <div className="mt-4">
                <Button variant="outline" size="sm" onClick={clearFilters}>
                  Clear Filters
                </Button>
              </div>
            )}
          </CardContent>
        </Card>
      )}

      <Card>
        <CardHeader>
          <CardTitle>Recent Activity</CardTitle>
          <CardDescription>
            Showing {auditLogs.items.length} of {auditLogs.total} log entries
          </CardDescription>
        </CardHeader>
        <CardContent>
          {auditLogs.items.length === 0 ? (
            <div className="text-center py-8">
              <History className="mx-auto h-12 w-12 text-muted-foreground/50" />
              <p className="mt-4 text-muted-foreground">
                No audit log entries found.
                {hasFilters && " Try adjusting your filters."}
              </p>
            </div>
          ) : (
            <>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Time</TableHead>
                    <TableHead>Action</TableHead>
                    <TableHead>Resource</TableHead>
                    <TableHead>User</TableHead>
                    <TableHead>IP Address</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {auditLogs.items.map((log) => (
                    <TableRow key={log.id}>
                      <TableCell className="whitespace-nowrap">
                        {formatDate(log.created_at)}
                      </TableCell>
                      <TableCell>
                        <Badge variant={getActionBadgeVariant(log.action)}>
                          {formatAction(log.action)}
                        </Badge>
                      </TableCell>
                      <TableCell>
                        <div className="flex flex-col">
                          <span className="font-medium">
                            {log.resource_name || log.resource_id || "-"}
                          </span>
                          <span className="text-xs text-muted-foreground">
                            {formatResourceType(log.resource_type)}
                          </span>
                        </div>
                      </TableCell>
                      <TableCell>
                        <span className="text-sm">
                          {log.user_id === "system" ? (
                            <Badge variant="outline">System</Badge>
                          ) : (
                            log.user_id || "-"
                          )}
                        </span>
                      </TableCell>
                      <TableCell>
                        <code className="text-xs bg-muted px-1 py-0.5 rounded">
                          {log.ip_address || "-"}
                        </code>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>

              {/* Pagination */}
              {auditLogs.total_pages > 1 && (
                <div className="flex items-center justify-between mt-4 pt-4 border-t">
                  <div className="text-sm text-muted-foreground">
                    Page {auditLogs.page} of {auditLogs.total_pages}
                  </div>
                  <div className="flex items-center gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => goToPage(auditLogs.page - 1)}
                      disabled={auditLogs.page <= 1}
                    >
                      <ChevronLeft className="h-4 w-4" />
                      Previous
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => goToPage(auditLogs.page + 1)}
                      disabled={auditLogs.page >= auditLogs.total_pages}
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
