import { useState } from "react";
import { Loader2, Activity, Filter, Calendar, ChevronLeft, ChevronRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
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
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type {
  TeamAuditLogPage,
  TeamAuditLogQuery,
  TeamAuditAction,
  TeamAuditResourceType,
} from "@/types/api";

interface AuditTabProps {
  teamId: string;
  isActiveTab: boolean;
}

const AUDIT_ACTION_OPTIONS: { value: TeamAuditAction; label: string }[] = [
  { value: "team_created", label: "Team Created" },
  { value: "team_updated", label: "Team Updated" },
  { value: "team_deleted", label: "Team Deleted" },
  { value: "member_invited", label: "Member Invited" },
  { value: "member_joined", label: "Member Joined" },
  { value: "member_removed", label: "Member Removed" },
  { value: "role_changed", label: "Role Changed" },
  { value: "invitation_created", label: "Invitation Created" },
  { value: "invitation_revoked", label: "Invitation Revoked" },
  { value: "invitation_accepted", label: "Invitation Accepted" },
  { value: "invitation_resent", label: "Invitation Resent" },
  { value: "app_created", label: "App Created" },
  { value: "app_updated", label: "App Updated" },
  { value: "app_deleted", label: "App Deleted" },
  { value: "project_created", label: "Project Created" },
  { value: "project_updated", label: "Project Updated" },
  { value: "project_deleted", label: "Project Deleted" },
  { value: "database_created", label: "Database Created" },
  { value: "database_deleted", label: "Database Deleted" },
  { value: "service_created", label: "Service Created" },
  { value: "service_deleted", label: "Service Deleted" },
  { value: "deployment_triggered", label: "Deployment Triggered" },
  { value: "deployment_rolled_back", label: "Deployment Rolled Back" },
];

const AUDIT_RESOURCE_TYPE_OPTIONS: { value: TeamAuditResourceType; label: string }[] = [
  { value: "team", label: "Team" },
  { value: "member", label: "Member" },
  { value: "invitation", label: "Invitation" },
  { value: "app", label: "App" },
  { value: "project", label: "Project" },
  { value: "database", label: "Database" },
  { value: "service", label: "Service" },
  { value: "deployment", label: "Deployment" },
];

function formatDateTime(dateStr: string): string {
  return new Date(dateStr).toLocaleString();
}

function formatActionLabel(action: string): string {
  return action
    .split("_")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}

function getActionBadgeVariant(
  action: string
): "default" | "secondary" | "outline" | "destructive" {
  if (action.includes("deleted") || action.includes("removed") || action.includes("revoked")) {
    return "destructive";
  }
  if (action.includes("created") || action.includes("joined") || action.includes("accepted")) {
    return "default";
  }
  if (action.includes("updated") || action.includes("changed")) {
    return "secondary";
  }
  return "outline";
}

export function AuditTab({ teamId, isActiveTab }: AuditTabProps) {
  const [auditActionFilter, setAuditActionFilter] = useState<string>("");
  const [auditResourceTypeFilter, setAuditResourceTypeFilter] = useState<string>("");
  const [auditStartDate, setAuditStartDate] = useState<string>("");
  const [auditEndDate, setAuditEndDate] = useState<string>("");
  const [auditPage, setAuditPage] = useState(1);
  const auditPerPage = 20;

  const auditLogQuery: TeamAuditLogQuery = {
    page: auditPage,
    per_page: auditPerPage,
    ...(auditActionFilter && { action: auditActionFilter }),
    ...(auditResourceTypeFilter && { resource_type: auditResourceTypeFilter }),
    ...(auditStartDate && { start_date: new Date(auditStartDate).toISOString() }),
    ...(auditEndDate && { end_date: new Date(auditEndDate + "T23:59:59").toISOString() }),
  };

  const { data: auditLogs, isLoading: isLoadingAuditLogs } = useQuery<TeamAuditLogPage>({
    queryKey: ["team-audit-logs", teamId, auditLogQuery],
    queryFn: () => api.getTeamAuditLogs(teamId, auditLogQuery),
    enabled: !!teamId && isActiveTab,
  });

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Activity className="h-5 w-5" />
              Team Activity
            </CardTitle>
            <CardDescription>
              View all actions and events that have occurred in this team.
            </CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Filters */}
        <div className="flex flex-wrap gap-4">
          <div className="flex items-center gap-2">
            <Filter className="h-4 w-4 text-muted-foreground" />
            <Select
              value={auditActionFilter || "all"}
              onValueChange={(value) => {
                setAuditActionFilter(value === "all" ? "" : value);
                setAuditPage(1);
              }}
            >
              <SelectTrigger className="w-48">
                <SelectValue placeholder="All Actions" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">All Actions</SelectItem>
                {AUDIT_ACTION_OPTIONS.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    {option.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div>
            <Select
              value={auditResourceTypeFilter || "all"}
              onValueChange={(value) => {
                setAuditResourceTypeFilter(value === "all" ? "" : value);
                setAuditPage(1);
              }}
            >
              <SelectTrigger className="w-40">
                <SelectValue placeholder="All Resources" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">All Resources</SelectItem>
                {AUDIT_RESOURCE_TYPE_OPTIONS.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    {option.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="flex items-center gap-2">
            <Calendar className="h-4 w-4 text-muted-foreground" />
            <Input
              type="date"
              value={auditStartDate}
              onChange={(e) => {
                setAuditStartDate(e.target.value);
                setAuditPage(1);
              }}
              className="w-36"
              placeholder="Start date"
            />
            <span className="text-muted-foreground">to</span>
            <Input
              type="date"
              value={auditEndDate}
              onChange={(e) => {
                setAuditEndDate(e.target.value);
                setAuditPage(1);
              }}
              className="w-36"
              placeholder="End date"
            />
          </div>

          {(auditActionFilter || auditResourceTypeFilter || auditStartDate || auditEndDate) && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                setAuditActionFilter("");
                setAuditResourceTypeFilter("");
                setAuditStartDate("");
                setAuditEndDate("");
                setAuditPage(1);
              }}
            >
              Clear Filters
            </Button>
          )}
        </div>

        {/* Audit Log Table */}
        {isLoadingAuditLogs ? (
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin" />
          </div>
        ) : !auditLogs || auditLogs.items.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground">
            No activity found
          </div>
        ) : (
          <>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-40">Timestamp</TableHead>
                  <TableHead className="w-36">User</TableHead>
                  <TableHead className="w-44">Action</TableHead>
                  <TableHead className="w-28">Resource</TableHead>
                  <TableHead>Details</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {auditLogs.items.map((log) => (
                  <TableRow key={log.id}>
                    <TableCell className="text-sm text-muted-foreground">
                      {formatDateTime(log.created_at)}
                    </TableCell>
                    <TableCell>
                      <div className="flex flex-col">
                        <span className="font-medium text-sm truncate max-w-32">
                          {log.user_name || "System"}
                        </span>
                        {log.user_email && (
                          <span className="text-xs text-muted-foreground truncate max-w-32">
                            {log.user_email}
                          </span>
                        )}
                      </div>
                    </TableCell>
                    <TableCell>
                      <Badge variant={getActionBadgeVariant(log.action)}>
                        {formatActionLabel(log.action)}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <span className="text-sm capitalize">{log.resource_type}</span>
                    </TableCell>
                    <TableCell>
                      {log.details && (
                        <span className="text-sm text-muted-foreground">
                          {Object.entries(log.details)
                            .filter(
                              ([key]) =>
                                !key.includes("id") ||
                                key === "old_role" ||
                                key === "new_role"
                            )
                            .map(([key, value]) => (
                              <span key={key} className="mr-3">
                                <span className="capitalize">
                                  {key.replace(/_/g, " ")}
                                </span>
                                : {String(value)}
                              </span>
                            ))}
                        </span>
                      )}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>

            {/* Pagination */}
            {auditLogs.total_pages > 1 && (
              <div className="flex items-center justify-between pt-4">
                <div className="text-sm text-muted-foreground">
                  Showing {(auditLogs.page - 1) * auditLogs.per_page + 1} to{" "}
                  {Math.min(auditLogs.page * auditLogs.per_page, auditLogs.total)} of{" "}
                  {auditLogs.total} entries
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setAuditPage((p) => Math.max(1, p - 1))}
                    disabled={auditLogs.page <= 1}
                  >
                    <ChevronLeft className="h-4 w-4" />
                    Previous
                  </Button>
                  <span className="text-sm text-muted-foreground">
                    Page {auditLogs.page} of {auditLogs.total_pages}
                  </span>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() =>
                      setAuditPage((p) => Math.min(auditLogs.total_pages, p + 1))
                    }
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
  );
}
