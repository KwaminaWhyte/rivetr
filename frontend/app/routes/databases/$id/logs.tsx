import { useState, useEffect, useRef } from "react";
import { useOutletContext } from "react-router";
import { useQuery } from "@tanstack/react-query";
import type { ManagedDatabase, DatabaseLogEntry } from "@/types/api";
import { api } from "@/lib/api";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { RefreshCw, Download, Terminal, AlertCircle } from "lucide-react";

interface OutletContext {
  database: ManagedDatabase;
  token: string;
}

export default function DatabaseLogsTab() {
  const { database, token } = useOutletContext<OutletContext>();
  const [lines, setLines] = useState(100);
  const [autoScroll, setAutoScroll] = useState(true);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const scrollRef = useRef<HTMLDivElement>(null);
  const [isClient, setIsClient] = useState(false);

  // Ensure we only run the query on the client
  useEffect(() => {
    setIsClient(true);
  }, []);

  const {
    data: logs = [],
    isLoading,
    isError,
    error,
    refetch,
  } = useQuery<DatabaseLogEntry[]>({
    queryKey: ["databaseLogs", database.id, lines, token],
    queryFn: () => api.getDatabaseLogs(database.id, lines, token),
    enabled:
      isClient &&
      database.status === "running" &&
      !!database.container_id &&
      !!token,
    refetchInterval: autoRefresh ? 5000 : false,
  });

  // Auto-scroll to bottom when new logs arrive
  useEffect(() => {
    if (autoScroll && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs, autoScroll]);

  const downloadLogs = () => {
    const logText = logs
      .map((log) => `[${log.timestamp}] [${log.stream}] ${log.message}`)
      .join("\n");
    const blob = new Blob([logText], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${database.name}-logs-${new Date().toISOString()}.log`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  const formatTimestamp = (timestamp: string) => {
    try {
      const date = new Date(timestamp);
      return date.toLocaleTimeString();
    } catch {
      return timestamp;
    }
  };

  if (!database.container_id || database.status !== "running") {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Terminal className="h-5 w-5" />
            Container Logs
          </CardTitle>
          <CardDescription>View database container output</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex flex-col items-center justify-center py-12 text-center">
            <AlertCircle className="h-12 w-12 text-muted-foreground mb-4" />
            <h3 className="text-lg font-medium">No logs available</h3>
            <p className="text-muted-foreground mt-1">
              {database.status === "stopped"
                ? "The database is stopped. Start it to view logs."
                : database.status === "failed"
                  ? "The database failed to start. Check the General tab for error details."
                  : "The database container is not running."}
            </p>
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-6">
      {/* Controls Card */}
      <Card>
        <CardHeader className="pb-3">
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Terminal className="h-5 w-5" />
                Container Logs
              </CardTitle>
              <CardDescription>View database container output</CardDescription>
            </div>
            <div className="flex items-center gap-2">
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
              <Button
                variant="outline"
                size="sm"
                onClick={downloadLogs}
                disabled={logs.length === 0}
              >
                <Download className="h-4 w-4 mr-2" />
                Download
              </Button>
            </div>
          </div>
        </CardHeader>
        <CardContent className="pt-0">
          <div className="flex flex-wrap items-center gap-6">
            <div className="flex items-center gap-2">
              <Label htmlFor="lines" className="text-sm">
                Lines:
              </Label>
              <Select
                value={String(lines)}
                onValueChange={(v) => setLines(Number(v))}
              >
                <SelectTrigger className="w-24">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="50">50</SelectItem>
                  <SelectItem value="100">100</SelectItem>
                  <SelectItem value="500">500</SelectItem>
                  <SelectItem value="1000">1000</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="flex items-center gap-2">
              <Switch
                id="auto-refresh"
                checked={autoRefresh}
                onCheckedChange={setAutoRefresh}
              />
              <Label htmlFor="auto-refresh" className="text-sm">
                Auto-refresh (5s)
              </Label>
            </div>
            <div className="flex items-center gap-2">
              <Switch
                id="auto-scroll"
                checked={autoScroll}
                onCheckedChange={setAutoScroll}
              />
              <Label htmlFor="auto-scroll" className="text-sm">
                Auto-scroll
              </Label>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Logs Output */}
      <Card>
        <CardContent className="p-0">
          {isError ? (
            <div className="p-6 text-center">
              <AlertCircle className="h-8 w-8 text-destructive mx-auto mb-2" />
              <p className="text-sm text-destructive">
                {error instanceof Error && error.message.includes("Container is stopped")
                  ? "The database container is not running. Start the database to view logs."
                  : `Failed to load logs: ${error instanceof Error ? error.message : "Unknown error"}`}
              </p>
            </div>
          ) : (
            <ScrollArea
              ref={scrollRef}
              className="h-[500px] w-full rounded-md bg-black"
            >
              <div className="p-4 font-mono text-sm">
                {isLoading ? (
                  <p className="text-gray-500">Loading logs...</p>
                ) : logs.length === 0 ? (
                  <p className="text-gray-500">No logs available</p>
                ) : (
                  logs.map((log, idx) => (
                    <LogLine
                      key={idx}
                      log={log}
                      formatTimestamp={formatTimestamp}
                    />
                  ))
                )}
              </div>
            </ScrollArea>
          )}
        </CardContent>
      </Card>

      {/* Log Stats */}
      <div className="flex items-center justify-between text-sm text-muted-foreground">
        <div className="flex items-center gap-4">
          <span>
            Showing {logs.length} log{logs.length !== 1 ? "s" : ""}
          </span>
          <div className="flex items-center gap-2">
            <Badge
              variant="outline"
              className="bg-green-50 text-green-700 border-green-200"
            >
              stdout: {logs.filter((l) => l.stream === "stdout").length}
            </Badge>
            <Badge
              variant="outline"
              className="bg-red-50 text-red-700 border-red-200"
            >
              stderr: {logs.filter((l) => l.stream === "stderr").length}
            </Badge>
          </div>
        </div>
        {autoRefresh && (
          <span className="flex items-center gap-1">
            <span className="relative flex h-2 w-2">
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-400 opacity-75"></span>
              <span className="relative inline-flex h-2 w-2 rounded-full bg-green-500"></span>
            </span>
            Live updating
          </span>
        )}
      </div>
    </div>
  );
}

// Individual log line component
function LogLine({
  log,
  formatTimestamp,
}: {
  log: DatabaseLogEntry;
  formatTimestamp: (ts: string) => string;
}) {
  const isStderr = log.stream === "stderr";

  return (
    <div
      className={`flex gap-2 py-0.5 hover:bg-gray-900 ${isStderr ? "text-red-400" : "text-gray-300"}`}
    >
      <span className="text-gray-500 select-none shrink-0">
        {formatTimestamp(log.timestamp)}
      </span>
      <span
        className={`shrink-0 ${isStderr ? "text-red-500" : "text-green-500"}`}
      >
        [{log.stream}]
      </span>
      <span className="break-all">{log.message}</span>
    </div>
  );
}
