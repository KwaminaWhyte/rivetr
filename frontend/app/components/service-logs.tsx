import { useEffect, useRef, useState, useCallback } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { api } from "@/lib/api";
import type { ServiceLogEntry } from "@/types/api";

interface ServiceLogsProps {
  serviceId: string;
  serviceName?: string;
  serviceStatus?: string;
  token?: string;
}

export function ServiceLogs({ serviceId, serviceName, serviceStatus, token }: ServiceLogsProps) {
  const [logs, setLogs] = useState<ServiceLogEntry[]>([]);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isStopped, setIsStopped] = useState(false);
  const [autoScroll, setAutoScroll] = useState(true);
  const [isLoading, setIsLoading] = useState(true);
  const eventSourceRef = useRef<EventSource | null>(null);
  const logsEndRef = useRef<HTMLDivElement>(null);

  // Load initial logs
  const loadInitialLogs = useCallback(async () => {
    setIsLoading(true);
    setIsStopped(false);
    try {
      const initialLogs = await api.getServiceLogs(serviceId, 100, token);
      setLogs(initialLogs);
      setError(null);
    } catch (e) {
      const errorMessage = e instanceof Error ? e.message : "Failed to load logs";
      // Check if the error is due to container being stopped
      if (errorMessage.includes("Container is stopped") || errorMessage.includes("not running")) {
        setIsStopped(true);
        setError(null);
      } else {
        setError(errorMessage);
      }
    } finally {
      setIsLoading(false);
    }
  }, [serviceId, token]);

  // Connect to SSE stream
  const connectToStream = useCallback(() => {
    // Close existing connection
    if (eventSourceRef.current) {
      eventSourceRef.current.close();
    }

    const url = api.getServiceLogsStreamUrl(serviceId, token);
    const eventSource = new EventSource(url);
    eventSourceRef.current = eventSource;

    eventSource.onopen = () => {
      setConnected(true);
      setError(null);
    };

    eventSource.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);

        // Check for end or error messages
        if (data.type === "end") {
          setConnected(false);
          return;
        }
        if (data.type === "error") {
          setError(data.message);
          setConnected(false);
          return;
        }

        // It's a log entry
        const logEntry: ServiceLogEntry = {
          timestamp: data.timestamp,
          service: data.service,
          message: data.message,
        };

        setLogs((prev) => [...prev.slice(-500), logEntry]); // Keep last 500 lines
      } catch {
        // Ignore parse errors (might be keep-alive)
      }
    };

    eventSource.onerror = () => {
      setError("Connection lost. Retrying...");
      setConnected(false);

      // Attempt to reconnect after 3 seconds
      setTimeout(() => {
        if (eventSourceRef.current === eventSource) {
          connectToStream();
        }
      }, 3000);
    };

    return eventSource;
  }, [serviceId, token]);

  useEffect(() => {
    // Load initial logs first
    loadInitialLogs();

    // Then connect to stream
    const eventSource = connectToStream();

    return () => {
      eventSource.close();
    };
  }, [serviceId, loadInitialLogs, connectToStream]);

  useEffect(() => {
    if (autoScroll && logsEndRef.current) {
      logsEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [logs, autoScroll]);

  const handleClear = () => {
    setLogs([]);
  };

  const handleRefresh = () => {
    loadInitialLogs();
    // Reconnect to stream
    connectToStream();
  };

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between">
        <div className="flex items-center gap-2">
          <CardTitle>
            {serviceName ? `${serviceName} Logs` : "Service Logs"}
          </CardTitle>
          <Badge variant={connected ? "default" : "secondary"}>
            {connected ? "Live" : isLoading ? "Loading..." : "Disconnected"}
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
          <Button variant="outline" size="sm" onClick={handleRefresh}>
            Refresh
          </Button>
          <Button variant="outline" size="sm" onClick={handleClear}>
            Clear
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {error && (
          <div className="text-sm text-red-500 py-2 mb-2 text-center bg-red-50 dark:bg-red-950 rounded">
            {error}
          </div>
        )}
        {isStopped ? (
          <div className="bg-gray-900 text-gray-100 rounded-md p-8 text-center">
            <div className="text-gray-400 mb-2">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="48"
                height="48"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                className="mx-auto mb-4"
              >
                <rect x="6" y="4" width="4" height="16" rx="1" />
                <rect x="14" y="4" width="4" height="16" rx="1" />
              </svg>
            </div>
            <h3 className="text-lg font-medium text-gray-200 mb-2">Service is stopped</h3>
            <p className="text-gray-400">
              Start the service to view logs.
            </p>
          </div>
        ) : (
          <div className="bg-gray-900 text-gray-100 rounded-md p-4 font-mono text-xs overflow-auto max-h-96">
            {isLoading && logs.length === 0 ? (
              <div className="text-gray-500">Loading logs...</div>
            ) : logs.length === 0 ? (
              <div className="text-gray-500">No logs available</div>
            ) : (
              logs.map((log, i) => (
                <div key={i} className="hover:bg-gray-800 px-1 -mx-1 rounded">
                  <span className="text-gray-500">
                    [{new Date(log.timestamp).toLocaleTimeString()}]
                  </span>{" "}
                  <span className="text-blue-400">[{log.service}]</span>{" "}
                  <span className="text-gray-100">{log.message}</span>
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
