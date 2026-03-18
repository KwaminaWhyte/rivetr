import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { RefreshCw } from "lucide-react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { apiRequest } from "@/lib/api";

export function meta() {
  return [
    { title: "Proxy Logs - Rivetr" },
    { name: "description", content: "View reverse proxy access logs" },
  ];
}

interface ProxyLog {
  id: number;
  ts: string;
  host: string;
  method: string;
  path: string;
  status: number;
  response_ms: number;
  bytes_out: number;
  client_ip: string | null;
  user_agent: string | null;
}

function statusBadgeVariant(
  status: number
): "default" | "secondary" | "destructive" | "outline" {
  if (status >= 500) return "destructive";
  if (status >= 400) return "destructive";
  if (status >= 300) return "secondary";
  return "default";
}

function statusBadgeClass(status: number): string {
  if (status >= 500) return "bg-red-500/15 text-red-600 border-red-500/30";
  if (status >= 400) return "bg-orange-500/15 text-orange-600 border-orange-500/30";
  if (status >= 300) return "bg-yellow-500/15 text-yellow-600 border-yellow-500/30";
  return "bg-green-500/15 text-green-600 border-green-500/30";
}

function formatTimestamp(iso: string): string {
  try {
    return new Date(iso + "Z").toLocaleString();
  } catch {
    return iso;
  }
}

export default function ProxyLogsPage() {
  const [domain, setDomain] = useState("");
  const [statusFilter, setStatusFilter] = useState("all");
  const [limit, setLimit] = useState("100");
  const [autoRefresh, setAutoRefresh] = useState(false);
  const [appliedDomain, setAppliedDomain] = useState("");
  const [appliedStatus, setAppliedStatus] = useState("all");

  const buildQueryString = () => {
    const params = new URLSearchParams();
    if (appliedDomain) params.set("domain", appliedDomain);
    if (appliedStatus !== "all") params.set("status", appliedStatus);
    params.set("limit", limit);
    return params.toString();
  };

  const { data: logs = [], isLoading, isFetching, refetch } = useQuery<ProxyLog[]>({
    queryKey: ["proxy-logs", appliedDomain, appliedStatus, limit],
    queryFn: () => {
      const qs = buildQueryString();
      return apiRequest<ProxyLog[]>(`/proxy/logs${qs ? `?${qs}` : ""}`);
    },
    refetchInterval: autoRefresh ? 5000 : false,
  });

  const handleApplyFilters = (e: React.FormEvent) => {
    e.preventDefault();
    setAppliedDomain(domain);
    setAppliedStatus(statusFilter);
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Proxy Access Logs</h1>
          <p className="text-muted-foreground">
            Inspect HTTP requests handled by the built-in reverse proxy
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant={autoRefresh ? "default" : "outline"}
            size="sm"
            onClick={() => setAutoRefresh((v) => !v)}
          >
            <RefreshCw className={`h-4 w-4 mr-2 ${autoRefresh ? "animate-spin" : ""}`} />
            {autoRefresh ? "Auto-refresh on" : "Auto-refresh"}
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => refetch()}
            disabled={isFetching}
          >
            <RefreshCw className={`h-4 w-4 mr-2 ${isFetching ? "animate-spin" : ""}`} />
            Refresh
          </Button>
        </div>
      </div>

      {/* Filter bar */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-base">Filters</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleApplyFilters} className="flex flex-wrap items-end gap-4">
            <div className="space-y-1 min-w-[200px]">
              <Label htmlFor="domain-filter">Domain</Label>
              <Input
                id="domain-filter"
                placeholder="e.g. example.com"
                value={domain}
                onChange={(e) => setDomain(e.target.value)}
              />
            </div>

            <div className="space-y-1 min-w-[140px]">
              <Label htmlFor="status-filter">Status</Label>
              <Select value={statusFilter} onValueChange={setStatusFilter}>
                <SelectTrigger id="status-filter">
                  <SelectValue placeholder="All" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">All</SelectItem>
                  <SelectItem value="2xx">2xx Success</SelectItem>
                  <SelectItem value="3xx">3xx Redirect</SelectItem>
                  <SelectItem value="4xx">4xx Client Error</SelectItem>
                  <SelectItem value="5xx">5xx Server Error</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-1 min-w-[120px]">
              <Label htmlFor="limit-select">Rows</Label>
              <Select value={limit} onValueChange={setLimit}>
                <SelectTrigger id="limit-select">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="50">50</SelectItem>
                  <SelectItem value="100">100</SelectItem>
                  <SelectItem value="500">500</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <Button type="submit">Apply</Button>
            <Button
              type="button"
              variant="outline"
              onClick={() => {
                setDomain("");
                setStatusFilter("all");
                setAppliedDomain("");
                setAppliedStatus("all");
              }}
            >
              Clear
            </Button>
          </form>
        </CardContent>
      </Card>

      {/* Logs table */}
      <Card>
        <CardHeader>
          <CardTitle>
            Access Logs
            {logs.length > 0 && (
              <span className="ml-2 text-sm font-normal text-muted-foreground">
                ({logs.length} entries)
              </span>
            )}
          </CardTitle>
          <CardDescription>
            Most recent requests first. Enable auto-refresh to stream live traffic.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <p className="py-8 text-center text-sm text-muted-foreground">Loading…</p>
          ) : logs.length === 0 ? (
            <p className="py-8 text-center text-sm text-muted-foreground">
              No proxy log entries found. Traffic will appear here as requests are
              forwarded through the proxy.
            </p>
          ) : (
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="whitespace-nowrap">Timestamp</TableHead>
                    <TableHead>Host</TableHead>
                    <TableHead>Method</TableHead>
                    <TableHead>Path</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead className="whitespace-nowrap">Time (ms)</TableHead>
                    <TableHead className="whitespace-nowrap">Client IP</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {logs.map((log) => (
                    <TableRow key={log.id}>
                      <TableCell className="text-xs text-muted-foreground whitespace-nowrap font-mono">
                        {formatTimestamp(log.ts)}
                      </TableCell>
                      <TableCell className="text-sm font-medium max-w-[180px] truncate">
                        {log.host}
                      </TableCell>
                      <TableCell>
                        <span className="font-mono text-xs font-semibold uppercase">
                          {log.method}
                        </span>
                      </TableCell>
                      <TableCell className="text-xs font-mono max-w-[240px] truncate">
                        {log.path}
                      </TableCell>
                      <TableCell>
                        <Badge
                          variant="outline"
                          className={`text-xs font-mono ${statusBadgeClass(log.status)}`}
                        >
                          {log.status}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-sm tabular-nums">
                        {log.response_ms}
                      </TableCell>
                      <TableCell className="text-xs text-muted-foreground font-mono">
                        {log.client_ip ?? "—"}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
