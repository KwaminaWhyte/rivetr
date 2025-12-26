import { useState, useEffect, useRef, useCallback } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";

interface LogEntry {
  id: number;
  deployment_id: string;
  level: string;
  message: string;
  timestamp: string;
}

interface DeploymentLogsProps {
  deploymentId: string;
  isActive: boolean;
}

const levelColors: Record<string, string> = {
  info: "bg-blue-500",
  warn: "bg-yellow-500",
  error: "bg-red-500",
  debug: "bg-gray-500",
};

export function DeploymentLogs({ deploymentId, isActive }: DeploymentLogsProps) {
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

  // Connect to WebSocket
  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) return;

    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const wsUrl = `${protocol}//${window.location.host}/api/deployments/${deploymentId}/logs/stream`;

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
          return;
        }

        // It's a log entry
        setLogs((prev) => {
          // Avoid duplicates
          if (prev.some((l) => l.id === data.id)) return prev;
          return [...prev, data as LogEntry];
        });
      } catch (e) {
        console.error("Failed to parse log message:", e);
      }
    };

    ws.onerror = () => {
      setConnected(false);
    };

    ws.onclose = () => {
      setConnected(false);
    };
  }, [deploymentId]);

  // Auto-connect when component mounts if deployment is active
  useEffect(() => {
    if (isActive) {
      connect();
    }

    return () => {
      if (wsRef.current) {
        wsRef.current.close();
      }
    };
  }, [isActive, connect]);

  const formatTimestamp = (ts: string) => {
    return new Date(ts).toLocaleTimeString();
  };

  if (!isActive && logs.length === 0) {
    return null;
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
                    {log.message}
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
