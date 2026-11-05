import { useMemo, useState } from "react";
import { useNavigate } from "react-router";
import {
  Ban,
  FileText,
  GitCommitHorizontal,
  GitCompare,
  MoreHorizontal,
  RotateCcw,
  Rocket,
  Tag,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  DataTable,
  EmptyState,
  ResourceStatus,
  TablePagination,
  TableToolbar,
  formatDuration,
  relativeTime,
  usePagedRows,
  type DataColumn,
} from "@/components/page";
import type { Deployment, DeploymentStatus } from "@/types/api";
import { isString, useLocalPreference, usePersistedState } from "@/lib/persisted-state";

const ACTIVE: DeploymentStatus[] = ["pending", "cloning", "building", "starting", "checking"];

type StatusFilter = "all" | "running" | "active" | "failed" | "replaced" | "cancelled";

const STATUS_FILTERS: { value: StatusFilter; label: string }[] = [
  { value: "all", label: "All statuses" },
  { value: "running", label: "Running" },
  { value: "active", label: "In progress" },
  { value: "failed", label: "Failed" },
  { value: "replaced", label: "Replaced or stopped" },
  { value: "cancelled", label: "Cancelled" },
];

const SOURCE_LABEL: Record<string, string> = {
  manual: "Manual",
  webhook: "Webhook",
  api: "API",
  rollback: "Rollback",
  restart: "Restart",
  scheduled: "Scheduled",
};

function matchesStatus(d: Deployment, f: StatusFilter): boolean {
  switch (f) {
    case "all":
      return true;
    case "active":
      return ACTIVE.includes(d.status);
    case "replaced":
      return d.status === "replaced" || d.status === "stopped";
    default:
      return d.status === f;
  }
}

function sourceOf(d: Deployment): string {
  return d.trigger ?? "manual";
}

interface DeploymentHistoryTableProps {
  appId: string;
  deployments: Deployment[];
  /** Total on the server (may exceed the loaded list). */
  total: number;
  loading?: boolean;
  /** Name of the server the app runs on ("localhost" for the local runtime). */
  serverName: string;
  canRollback: (d: Deployment) => boolean;
  onRollback: (id: string) => void;
  onViewDiff: (id: string) => void;
  onCancel: (id: string) => void;
  busy?: boolean;
}

/**
 * Deployment history: search, status and source filters, short commit and
 * message, relative start time, duration, server, and per-row actions.
 */
export function DeploymentHistoryTable({
  appId,
  deployments,
  total,
  loading,
  serverName,
  canRollback,
  onRollback,
  onViewDiff,
  onCancel,
  busy,
}: DeploymentHistoryTableProps) {
  const navigate = useNavigate();
  const [search, setSearch] = useState("");
  // Filters and page size are remembered per browser (rivetr:ui:table:deployments:*).
  const [status, setStatus] = useLocalPreference<StatusFilter>(
    "table:deployments:status",
    "all",
    STATUS_FILTERS.map((f) => f.value)
  );
  const [storedSource, setSource] = usePersistedState<string>("table:deployments:source", "all", {
    validate: isString,
  });

  const sources = useMemo(() => {
    const set = new Set<string>(["manual", "webhook", "rollback"]);
    for (const d of deployments) set.add(sourceOf(d));
    return [...set];
  }, [deployments]);

  const source = storedSource === "all" || sources.includes(storedSource) ? storedSource : "all";

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    return deployments.filter(
      (d) =>
        matchesStatus(d, status) &&
        (source === "all" || sourceOf(d) === source) &&
        (!q ||
          [d.id, d.commit_sha ?? "", d.commit_message ?? "", d.git_tag ?? "", d.image_tag ?? ""].some((t) =>
            t.toLowerCase().includes(q)
          ))
    );
  }, [deployments, search, status, source]);

  const { pageRows, page, pageSize, setPage, setPageSize } = usePagedRows(filtered, 10, "deployments");

  const columns: DataColumn<Deployment>[] = [
    {
      id: "status",
      header: "Status",
      cell: (d) => (
        <div className="flex flex-col items-start gap-1">
          <ResourceStatus status={d.status} size="sm" />
          {d.approval_status === "pending" && (
            <span className="text-[11px] text-amber-600 dark:text-amber-400">Awaiting approval</span>
          )}
        </div>
      ),
    },
    {
      id: "commit",
      header: "Commit",
      className: "max-w-[22rem]",
      cell: (d) => (
        <div className="min-w-0">
          <div className="flex items-center gap-1.5 font-mono text-xs">
            <GitCommitHorizontal className="size-3.5 shrink-0 text-muted-foreground" />
            {d.commit_sha ? d.commit_sha.slice(0, 7) : <span className="text-muted-foreground">latest</span>}
            {d.git_tag && (
              <span className="inline-flex items-center gap-0.5 rounded bg-muted px-1 py-px font-sans text-[11px] text-muted-foreground">
                <Tag className="size-3" />
                {d.git_tag}
              </span>
            )}
          </div>
          <p className="mt-0.5 truncate text-sm" title={d.commit_message ?? undefined}>
            {d.commit_message?.split("\n")[0] || (
              <span className="text-muted-foreground">{d.image_tag ? `Image ${d.image_tag}` : "No commit message"}</span>
            )}
          </p>
        </div>
      ),
    },
    {
      id: "source",
      header: "Source",
      showFrom: "md",
      cell: (d) => (
        <span className="text-sm text-muted-foreground">{SOURCE_LABEL[sourceOf(d)] ?? sourceOf(d)}</span>
      ),
    },
    {
      id: "started",
      header: "Started",
      cell: (d) => (
        <span className="whitespace-nowrap text-sm text-muted-foreground" title={new Date(d.started_at).toLocaleString()}>
          {relativeTime(d.started_at)}
        </span>
      ),
    },
    {
      id: "duration",
      header: "Duration",
      showFrom: "sm",
      cell: (d) => {
        const end = d.built_at ?? d.finished_at;
        if (ACTIVE.includes(d.status)) {
          return <span className="text-sm text-amber-600 dark:text-amber-400">{formatDuration(d.started_at, new Date().toISOString())}</span>;
        }
        return <span className="text-sm tabular-nums text-muted-foreground">{formatDuration(d.started_at, end)}</span>;
      },
    },
    {
      id: "server",
      header: "Server",
      showFrom: "lg",
      cell: () => <span className="text-sm text-muted-foreground">{serverName}</span>,
    },
    {
      id: "actions",
      header: <span className="sr-only">Actions</span>,
      className: "w-10 text-right",
      cell: (d) => (
        <div onClick={(e) => e.stopPropagation()}>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon-sm" aria-label="Deployment actions">
                <MoreHorizontal />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-48">
              <DropdownMenuItem onClick={() => navigate(`/apps/${appId}/deployments/${d.id}`)}>
                <FileText className="size-4" />
                View logs
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => onViewDiff(d.id)}>
                <GitCompare className="size-4" />
                Compare changes
              </DropdownMenuItem>
              {canRollback(d) && (
                <DropdownMenuItem disabled={busy} onClick={() => onRollback(d.id)}>
                  <RotateCcw className="size-4" />
                  Roll back to this
                </DropdownMenuItem>
              )}
              {ACTIVE.includes(d.status) && (
                <>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem variant="destructive" disabled={busy} onClick={() => onCancel(d.id)}>
                    <Ban className="size-4" />
                    Cancel deployment
                  </DropdownMenuItem>
                </>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      ),
    },
  ];

  return (
    <DataTable
      loading={loading}
      columns={columns}
      rows={pageRows}
      getRowKey={(d) => d.id}
      onRowClick={(d) => navigate(`/apps/${appId}/deployments/${d.id}`)}
      toolbar={
        <TableToolbar search={search} onSearchChange={setSearch} searchPlaceholder="Search commit, message or tag">
          <Select value={status} onValueChange={(v) => setStatus(v as StatusFilter)}>
            <SelectTrigger size="sm" className="w-40" aria-label="Filter by status">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {STATUS_FILTERS.map((f) => (
                <SelectItem key={f.value} value={f.value}>
                  {f.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Select value={source} onValueChange={setSource}>
            <SelectTrigger size="sm" className="w-36" aria-label="Filter by source">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All sources</SelectItem>
              {sources.map((s) => (
                <SelectItem key={s} value={s}>
                  {SOURCE_LABEL[s] ?? s}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </TableToolbar>
      }
      empty={
        deployments.length === 0 ? (
          <EmptyState
            compact
            icon={<Rocket />}
            title="No deployments yet"
            description="Deploy the app to see its history here."
          />
        ) : (
          <EmptyState compact title="No matches" description="No deployments match these filters." />
        )
      }
      footer={
        filtered.length > 0 ? (
          <div className="space-y-1">
            <TablePagination
              page={page}
              pageSize={pageSize}
              total={filtered.length}
              onPageChange={setPage}
              onPageSizeChange={setPageSize}
            />
            {total > deployments.length && (
              <p className="text-[11px] text-muted-foreground">
                Showing the latest {deployments.length} of {total} deployments.
              </p>
            )}
          </div>
        ) : undefined
      }
    />
  );
}
