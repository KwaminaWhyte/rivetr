import { useEffect, useRef, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { api } from "@/lib/api";
import { getAuthToken } from "@/lib/auth";

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
  token?: string;
}

export function RuntimeLogs({ appId, token: propToken }: RuntimeLogsProps) {
  // Get token from props or localStorage
  const token = propToken || getAuthToken() || "";
  const [logs, setLogs] = useState<LogMessage[]>([]);
  const [connected, setConnected] = useState(false);
  const [reconnecting, setReconnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const eventSourceRef = useRef<EventSource | null>(null);
  const logsEndRef = useRef<HTMLDivElement>(null);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const reconnectAttemptsRef = useRef(0);
  const maxReconnectAttempts = 10;
  const isManualCloseRef = useRef(false);

  useEffect(() => {
    const connect = () => {
      // Use SSE endpoint with auth header via fetch
      const streamUrl = api.getRuntimeLogsStreamUrl(appId);

      // Create EventSource with credentials
      // Note: EventSource doesn't support custom headers, so we use a polyfill approach
      // by using fetch with the authorization header and reading the stream
      const abortController = new AbortController();

      const fetchStream = async () => {
        try {
          const response = await fetch(streamUrl, {
            method: 'GET',
            headers: {
              'Authorization': `Bearer ${token}`,
              'Accept': 'text/event-stream',
            },
            credentials: 'include',
            signal: abortController.signal,
          });

          if (!response.ok) {
            const errorText = await response.text();
            let errorMessage = "Connection failed";
            try {
              const errorJson = JSON.parse(errorText);
              errorMessage = errorJson.error || errorMessage;
            } catch {
              // Not JSON
            }
            setError(errorMessage);
            setConnected(false);
            if (!isManualCloseRef.current) {
              scheduleReconnect();
            }
            return;
          }

          setConnected(true);
          setReconnecting(false);
          setError(null);
          reconnectAttemptsRef.current = 0;

          const reader = response.body?.getReader();
          if (!reader) {
            setError("No response body");
            return;
          }

          const decoder = new TextDecoder();
          let buffer = "";

          while (true) {
            const { done, value } = await reader.read();
            if (done) {
              // Stream ended
              setConnected(false);
              if (!isManualCloseRef.current) {
                scheduleReconnect();
              }
              break;
            }

            buffer += decoder.decode(value, { stream: true });
            const lines = buffer.split("\n");
            buffer = lines.pop() || "";

            for (const line of lines) {
              if (line.startsWith("data: ")) {
                const data = line.slice(6);
                try {
                  const msg: LogMessage = JSON.parse(data);
                  if (msg.type === "connected") {
                    setLogs((prev) => {
                      const hasConnected = prev.some(l => l.type === "connected" && l.message?.includes(msg.container_id?.slice(0, 12) || ""));
                      if (hasConnected) return prev;
                      return [...prev, { type: "connected", message: `Connected to container ${msg.container_id?.slice(0, 12)}` }];
                    });
                  } else if (msg.type === "error") {
                    setError(msg.message || "Unknown error");
                    setConnected(false);
                  } else if (msg.type === "end") {
                    // Stream ended - schedule reconnect
                    setConnected(false);
                    if (!isManualCloseRef.current) {
                      scheduleReconnect();
                    }
                  } else if (msg.type === "log") {
                    setLogs((prev) => [...prev.slice(-500), msg]); // Keep last 500 lines
                  }
                } catch {
                  // Ignore parse errors
                }
              }
            }
          }
        } catch (err) {
          if (err instanceof Error && err.name === 'AbortError') {
            // Request was aborted, don't reconnect
            return;
          }
          setError("Connection error");
          setConnected(false);
          if (!isManualCloseRef.current) {
            scheduleReconnect();
          }
        }
      };

      fetchStream();

      // Store abort controller for cleanup
      eventSourceRef.current = { close: () => abortController.abort() } as unknown as EventSource;
    };

    const scheduleReconnect = () => {
      if (reconnectAttemptsRef.current >= maxReconnectAttempts) {
        setReconnecting(false);
        setError("Max reconnection attempts reached. Refresh to try again.");
        return;
      }

      setReconnecting(true);

      // Exponential backoff: 1s, 2s, 4s, 8s... max 30s
      const delay = Math.min(1000 * Math.pow(2, reconnectAttemptsRef.current), 30000);
      reconnectAttemptsRef.current++;

      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }

      reconnectTimeoutRef.current = setTimeout(() => {
        if (!isManualCloseRef.current) {
          connect();
        }
      }, delay);
    };

    isManualCloseRef.current = false;
    connect();

    return () => {
      isManualCloseRef.current = true;
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
      }
    };
  }, [appId, token]);

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
          <Badge variant={connected ? "default" : reconnecting ? "outline" : "secondary"}>
            {connected ? "Live" : reconnecting ? "Reconnecting..." : "Disconnected"}
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
