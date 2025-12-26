import { useEffect, useRef, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { api } from "@/lib/api";

interface LogMessage {
  type: "log" | "connected" | "end" | "error";
  message?: string;
  timestamp?: string;
  stream?: "stdout" | "stderr";
  container_id?: string;
}

interface RuntimeLogsProps {
  appId: string;
  appName?: string;
}

export function RuntimeLogs({ appId }: RuntimeLogsProps) {
  const [logs, setLogs] = useState<LogMessage[]>([]);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const wsRef = useRef<WebSocket | null>(null);
  const logsEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const wsUrl = api.getRuntimeLogsWsUrl(appId);
    const ws = new WebSocket(wsUrl);
    wsRef.current = ws;

    ws.onopen = () => {
      setConnected(true);
      setError(null);
    };

    ws.onmessage = (event) => {
      try {
        const msg: LogMessage = JSON.parse(event.data);
        if (msg.type === "connected") {
          setLogs([{ type: "connected", message: `Connected to container ${msg.container_id?.slice(0, 12)}` }]);
        } else if (msg.type === "error") {
          setError(msg.message || "Unknown error");
          setConnected(false);
        } else if (msg.type === "end") {
          setConnected(false);
          setLogs((prev) => [...prev, { type: "end", message: msg.message || "Stream ended" }]);
        } else if (msg.type === "log") {
          setLogs((prev) => [...prev.slice(-500), msg]); // Keep last 500 lines
        }
      } catch {
        // Ignore parse errors
      }
    };

    ws.onerror = () => {
      setError("WebSocket connection error");
      setConnected(false);
    };

    ws.onclose = () => {
      setConnected(false);
    };

    return () => {
      ws.close();
    };
  }, [appId]);

  useEffect(() => {
    if (autoScroll && logsEndRef.current) {
      logsEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [logs, autoScroll]);

  const handleClear = () => {
    setLogs([]);
  };

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between">
        <div className="flex items-center gap-2">
          <CardTitle>Runtime Logs</CardTitle>
          <Badge variant={connected ? "default" : "secondary"}>
            {connected ? "Live" : "Disconnected"}
          </Badge>
        </div>
        <div className="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => setAutoScroll(!autoScroll)}
          >
            {autoScroll ? "Pause Scroll" : "Resume Scroll"}
          </Button>
          <Button variant="outline" size="sm" onClick={handleClear}>
            Clear
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {error ? (
          <div className="text-sm text-red-500 py-4 text-center">{error}</div>
        ) : (
          <div className="bg-gray-900 text-gray-100 rounded-md p-4 font-mono text-xs overflow-auto max-h-96">
            {logs.length === 0 ? (
              <div className="text-gray-500">Waiting for logs...</div>
            ) : (
              logs.map((log, i) => (
                <div
                  key={i}
                  className={`${
                    log.stream === "stderr"
                      ? "text-red-400"
                      : log.type === "connected" || log.type === "end"
                      ? "text-blue-400"
                      : "text-gray-100"
                  }`}
                >
                  {log.timestamp && (
                    <span className="text-gray-500">
                      [{new Date(log.timestamp).toLocaleTimeString()}]{" "}
                    </span>
                  )}
                  {log.message}
                </div>
              ))
            )}
            <div ref={logsEndRef} />
          </div>
        )}
      </CardContent>
    </Card>
  );
}
