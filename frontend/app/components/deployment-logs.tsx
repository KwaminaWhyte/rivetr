import { useState, useEffect, useRef, useCallback } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { getAuthToken } from "@/lib/auth";
import { api } from "@/lib/api";

interface LogEntry {
  id: string;
  deployment_id: string;
  level: string;
  message: string;
  timestamp: string;
}

// Strip ANSI escape sequences (color/style codes) from log messages so they
// render as plain text instead of leaking sequences like `\x1b[33m` into the
// UI (U10). Catches the common SGR set: \x1b[<digits;digits>m.
const ANSI_ESCAPE_RE = /\x1b\[[0-9;?]*[a-zA-Z]/g;
function stripAnsi(message: string): string {
  return message.replace(ANSI_ESCAPE_RE, "");
}

interface DeploymentLogsProps {
  deploymentId: string;
  isActive: boolean;
  token?: string;
}

const levelColors: Record<string, string> = {
  info: "bg-blue-500",
  warn: "bg-yellow-500",
  error: "bg-red-500",
  debug: "bg-gray-500",
};

export function DeploymentLogs({
  deploymentId,
  isActive,
  token: propToken,
}: DeploymentLogsProps) {
  const token = propToken || getAuthToken() || "";
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [connected, setConnected] = useState(false);
  const [ended, setEnded] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);
  const logsEndRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);

  // Scroll to bottom when new logs arrive (if autoScroll enabled)
  useEffect(() => {
    if (autoScroll && logsEndRef.current) {
      logsEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [logs, autoScroll]);

  // Merge new logs into state: deduplicate by id, keep sorted by id
  const mergeLogs = useCallback((incoming: LogEntry[]) => {
    if (!incoming || incoming.length === 0) return;
    setLogs((prev) => {
      const existingIds = new Set(prev.map((l) => l.id));
      const fresh = incoming.filter((l) => !existingIds.has(l.id));
      if (fresh.length === 0) return prev;
      return [...prev, ...fresh].sort((a, b) => a.timestamp.localeCompare(b.timestamp));
    });
  }, []);

  // Fetch all logs from REST API and merge (never replaces, always merges)
  const fetchLogs = useCallback(async () => {
    try {
      const result = await api.getDeploymentLogs(deploymentId, token || undefined);
      if (result && result.length > 0) {
        mergeLogs(result as LogEntry[]);
      }
    } catch {
      // ignore fetch errors
    }
  }, [deploymentId, token, mergeLogs]);

  // Initial load on mount
  useEffect(() => {
    fetchLogs();
    if (!isActive) {
      setEnded(true);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [deploymentId]); // Only re-run if deploymentId changes

  // REST polling as WebSocket fallback: poll every 3s while active,
  // do one final fetch when deployment completes
  useEffect(() => {
    if (!isActive) {
      fetchLogs();
      setEnded(true);
      return;
    }
    const poll = setInterval(fetchLogs, 3000);
    return () => clearInterval(poll);
  }, [isActive, fetchLogs]);

  // Connect to WebSocket for real-time streaming
  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) return;

    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const wsUrl = `${protocol}//${window.location.host}/api/deployments/${deploymentId}/logs/stream?token=${encodeURIComponent(token)}`;

    const ws = new WebSocket(wsUrl);
    wsRef.current = ws;

    ws.onopen = () => {
      setConnected(true);
      setEnded(false);
    };

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);

        if (data.type === "end") {
          setEnded(true);
          ws.close();
          // Final REST fetch to catch any logs written after the WS check
          fetchLogs();
          return;
        }

        mergeLogs([data as LogEntry]);
      } catch (e) {
        console.error("Failed to parse log message:", e);
      }
    };

    ws.onerror = () => {
      setConnected(false);
      // Fetch latest logs in case WS missed any
      fetchLogs();
    };

    ws.onclose = () => {
      setConnected(false);
      // Fetch latest logs in case WS missed any
      fetchLogs();
    };
  }, [deploymentId, token, mergeLogs, fetchLogs]);

  // Auto-connect WebSocket when active
  useEffect(() => {
    if (isActive) {
      connect();
    }

    return () => {
      wsRef.current?.close();
    };
  }, [isActive, connect]);

  const formatTimestamp = (ts: string) => {
    return new Date(ts).toLocaleTimeString();
  };

  if (logs.length === 0 && ended) {
    return (
      <Card>
        <CardContent className="py-8 text-center text-muted-foreground text-sm">
          No logs available for this deployment.
        </CardContent>
      </Card>
    );
  }

  if (logs.length === 0 && !isActive) {
    return (
      <Card>
        <CardContent className="py-8 text-center text-muted-foreground text-sm">
          Loading logs…
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            Build Logs
            {connected && !ended && (
              <span className="flex items-center gap-1.5 text-sm font-normal text-green-600">
                <span className="relative flex h-2 w-2">
                  <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-400 opacity-75"></span>
                  <span className="relative inline-flex h-2 w-2 rounded-full bg-green-500"></span>
                </span>
                Live
              </span>
            )}
            {ended && (
              <Badge variant="secondary" className="text-xs">
                Completed
              </Badge>
            )}
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setAutoScroll(!autoScroll)}
              className="text-xs"
            >
              {autoScroll ? "Disable" : "Enable"} auto-scroll
            </Button>
            {!connected && !ended && isActive && (
              <Button variant="outline" size="sm" onClick={connect}>
                Reconnect
              </Button>
            )}
          </div>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="bg-gray-900 rounded-lg p-4 max-h-96 overflow-y-auto font-mono text-sm">
          {logs.length === 0 ? (
            <div className="text-gray-500 text-center py-4">
              {connected ? "Waiting for logs..." : "No logs yet"}
            </div>
          ) : (
            <div className="space-y-1">
              {logs.map((log) => (
                <div key={log.id} className="flex gap-2 text-gray-300">
                  <span className="text-gray-500 flex-shrink-0">
                    {formatTimestamp(log.timestamp)}
                  </span>
                  <span
                    className={`px-1.5 py-0.5 rounded text-xs text-white flex-shrink-0 ${
                      levelColors[log.level] || "bg-gray-500"
                    }`}
                  >
                    {log.level.toUpperCase()}
                  </span>
                  <span className="whitespace-pre-wrap break-all">
                    {stripAnsi(log.message)}
                  </span>
                </div>
              ))}
              <div ref={logsEndRef} />
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
