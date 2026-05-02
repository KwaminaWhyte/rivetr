import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link } from "react-router";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Copy, Download, ExternalLink } from "lucide-react";
import { toast } from "sonner";
import { getAuthToken } from "@/lib/auth";
import { api } from "@/lib/api";
import { useDeployPanel } from "@/lib/deploy-panel-context";
import type { DeployPanelTarget } from "@/lib/deploy-panel-context";

/**
 * Shape of a normalized log line shown in the panel. Both the app deployment
 * stream and the service/database start stream are converted into this.
 */
interface PanelLogEntry {
  /** Stable unique id for dedupe (string-coerced for safety across kinds). */
  uid: string;
  /** Sortable numeric or RFC3339 string. */
  sortKey: string;
  /** Coarse phase used by the badge. */
  phase: string;
  /** Log level. */
  level: string;
  /** Free-form message. */
  message: string;
  /** RFC3339 timestamp (or empty if not provided). */
  timestamp: string;
}

const phaseColors: Record<string, string> = {
  pulling: "bg-blue-500",
  starting: "bg-purple-500",
  running: "bg-green-500",
  failed: "bg-red-500",
  info: "bg-gray-500",
};

const levelColors: Record<string, string> = {
  info: "bg-blue-500",
  warn: "bg-yellow-500",
  error: "bg-red-500",
  debug: "bg-gray-500",
};

/**
 * Root-level side panel. Subscribes to the global deploy panel context and
 * renders a Sheet streaming live logs whenever a target is set. The Sheet
 * stays mounted across navigation so logs keep flowing while the user
 * browses the dashboard underneath.
 */
export function DeploySidePanel() {
  const { target, open, setOpen } = useDeployPanel();

  return (
    <Sheet open={open} onOpenChange={setOpen}>
      <SheetContent
        side="right"
        // Wider than the default sm:max-w-sm so logs are readable.
        className="w-full sm:max-w-2xl flex flex-col gap-0 p-0"
        // Keep the page interactive underneath — user can navigate while
        // logs continue to stream.
        onInteractOutside={(e) => e.preventDefault()}
      >
        {target ? <PanelContent target={target} /> : <EmptyPanel />}
      </SheetContent>
    </Sheet>
  );
}

function EmptyPanel() {
  return (
    <SheetHeader>
      <SheetTitle>Deploy Logs</SheetTitle>
      <SheetDescription>No active deployment.</SheetDescription>
    </SheetHeader>
  );
}

function PanelContent({ target }: { target: DeployPanelTarget }) {
  const token = getAuthToken() ?? "";
  const [logs, setLogs] = useState<PanelLogEntry[]>([]);
  const [connected, setConnected] = useState(false);
  const [ended, setEnded] = useState(false);
  const [phase, setPhase] = useState<string>(
    target.kind === "deployment" ? "info" : "starting",
  );
  const [startedAt] = useState<number>(Date.now());
  const [, forceTick] = useState(0);
  const wsRef = useRef<WebSocket | null>(null);
  const logsEndRef = useRef<HTMLDivElement>(null);

  // Reset state when target changes (e.g. user kicks off a new start)
  useEffect(() => {
    setLogs([]);
    setConnected(false);
    setEnded(false);
    setPhase(target.kind === "deployment" ? "info" : "starting");
  }, [target.kind, target.id]);

  // Tick once a second to keep the elapsed timer fresh while running
  useEffect(() => {
    if (ended) return;
    const t = setInterval(() => forceTick((n) => n + 1), 1000);
    return () => clearInterval(t);
  }, [ended]);

  // Merge incoming logs deduped by uid
  const mergeLogs = useCallback((incoming: PanelLogEntry[]) => {
    if (!incoming.length) return;
    setLogs((prev) => {
      const seen = new Set(prev.map((l) => l.uid));
      const fresh = incoming.filter((l) => !seen.has(l.uid));
      if (!fresh.length) return prev;
      return [...prev, ...fresh].sort((a, b) =>
        a.sortKey.localeCompare(b.sortKey, undefined, { numeric: true }),
      );
    });
    // Keep the latest phase from the most recent event so the badge follows
    // the actual progression.
    const latest = incoming[incoming.length - 1];
    if (latest?.phase) setPhase(latest.phase);
  }, []);

  // Auto-scroll to bottom on new logs
  useEffect(() => {
    logsEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  // Endpoint helpers
  const restPath = useMemo(() => {
    switch (target.kind) {
      case "deployment":
        return `/api/deployments/${target.id}/logs`;
      case "service":
        return `/api/services/${target.id}/start-events`;
      case "database":
        return `/api/databases/${target.id}/start-events`;
    }
  }, [target]);

  const wsPath = useMemo(() => {
    switch (target.kind) {
      case "deployment":
        return `/api/deployments/${target.id}/logs/stream`;
      case "service":
        return `/api/services/${target.id}/start-stream`;
      case "database":
        return `/api/databases/${target.id}/start-stream`;
    }
  }, [target]);

  // REST snapshot fetch (used as fallback + initial paint)
  const fetchSnapshot = useCallback(async () => {
    try {
      if (target.kind === "deployment") {
        const result = await api.getDeploymentLogs(target.id, token || undefined);
        const normalized: PanelLogEntry[] = (result ?? []).map((l) => ({
          uid: `dep:${l.id}`,
          sortKey: String(l.id).padStart(12, "0"),
          phase: "info",
          level: l.level,
          message: l.message,
          timestamp: l.timestamp,
        }));
        mergeLogs(normalized);
      } else {
        // Plain fetch — we keep the api object lean and don't need a typed wrapper here.
        const headers: Record<string, string> = {};
        if (token) headers.Authorization = `Bearer ${token}`;
        const res = await fetch(restPath, { headers });
        if (!res.ok) return;
        const events = (await res.json()) as Array<{
          id: number;
          level: string;
          phase: string;
          message: string;
          timestamp: string;
        }>;
        const prefix = target.kind === "service" ? "svc" : "db";
        const normalized: PanelLogEntry[] = events.map((e) => ({
          uid: `${prefix}:${target.id}:${e.id}`,
          sortKey: String(e.id).padStart(12, "0"),
          phase: e.phase,
          level: e.level,
          message: e.message,
          timestamp: e.timestamp,
        }));
        mergeLogs(normalized);
      }
    } catch {
      // ignore — snapshot is best effort
    }
  }, [restPath, target, token, mergeLogs]);

  // Initial load
  useEffect(() => {
    fetchSnapshot();
  }, [fetchSnapshot]);

  // Polling fallback every 3s while not ended (in case WS is broken)
  useEffect(() => {
    if (ended) return;
    const t = setInterval(fetchSnapshot, 3000);
    return () => clearInterval(t);
  }, [ended, fetchSnapshot]);

  // WebSocket connection
  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) return;
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const url = `${protocol}//${window.location.host}${wsPath}?token=${encodeURIComponent(token)}`;
    const ws = new WebSocket(url);
    wsRef.current = ws;

    ws.onopen = () => {
      setConnected(true);
      setEnded(false);
    };

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data as string);

        if (data.type === "end") {
          setEnded(true);
          ws.close();
          fetchSnapshot();
          return;
        }
        if (data.type === "lag") {
          // server-side broadcast lag notice — ignore in UI, snapshot will catch up
          return;
        }

        if (target.kind === "deployment") {
          const entry: PanelLogEntry = {
            uid: `dep:${data.id}`,
            sortKey: String(data.id).padStart(12, "0"),
            phase: "info",
            level: data.level || "info",
            message: data.message || "",
            timestamp: data.timestamp || new Date().toISOString(),
          };
          mergeLogs([entry]);
        } else {
          const prefix = target.kind === "service" ? "svc" : "db";
          const entry: PanelLogEntry = {
            uid: `${prefix}:${target.id}:${data.id}`,
            sortKey: String(data.id).padStart(12, "0"),
            phase: data.phase || "info",
            level: data.level || "info",
            message: data.message || "",
            timestamp: data.timestamp || new Date().toISOString(),
          };
          mergeLogs([entry]);
          // If the server signals a terminal phase, auto-mark ended after a
          // short delay so the user still sees the last few lines flow.
          if (data.phase === "running" || data.phase === "failed") {
            // Soft-end: don't actually close the socket — server sends an
            // explicit `end` event; but flip the phase badge immediately.
            setPhase(data.phase);
          }
        }
      } catch (err) {
        console.error("Failed to parse log message", err);
      }
    };

    ws.onerror = () => {
      setConnected(false);
      fetchSnapshot();
    };

    ws.onclose = () => {
      setConnected(false);
      fetchSnapshot();
    };
  }, [wsPath, token, target, mergeLogs, fetchSnapshot]);

  useEffect(() => {
    connect();
    return () => {
      wsRef.current?.close();
      wsRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [target.kind, target.id]);

  const elapsedSeconds = Math.floor((Date.now() - startedAt) / 1000);
  const elapsedLabel = formatElapsed(elapsedSeconds);

  const copyAll = () => {
    const text = logs
      .map((l) => `[${l.timestamp}] ${l.level.toUpperCase()} ${l.message}`)
      .join("\n");
    navigator.clipboard.writeText(text);
    toast.success("Logs copied to clipboard");
  };

  const downloadAll = () => {
    const text = logs
      .map((l) => `[${l.timestamp}] ${l.level.toUpperCase()} ${l.message}`)
      .join("\n");
    const blob = new Blob([text], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${target.kind}-${target.id}-logs.txt`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  const phaseLabel = phase.charAt(0).toUpperCase() + phase.slice(1);
  const phaseColor = phaseColors[phase] ?? phaseColors.info;

  return (
    <>
      <SheetHeader className="border-b">
        <div className="flex items-start justify-between gap-3 pr-8">
          <div className="min-w-0 flex-1">
            <SheetTitle className="flex items-center gap-2 truncate">
              <span className="truncate">{target.title}</span>
              <Badge className={`${phaseColor} text-white`}>{phaseLabel}</Badge>
              {connected && !ended && (
                <span className="flex items-center gap-1.5 text-xs font-normal text-green-600">
                  <span className="relative flex h-2 w-2">
                    <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-400 opacity-75" />
                    <span className="relative inline-flex h-2 w-2 rounded-full bg-green-500" />
                  </span>
                  Live
                </span>
              )}
              {ended && (
                <Badge variant="secondary" className="text-xs">
                  Completed
                </Badge>
              )}
            </SheetTitle>
            <SheetDescription className="flex items-center gap-2 text-xs">
              <span>{target.subtitle ?? defaultSubtitle(target)}</span>
              <span aria-hidden>•</span>
              <span>Elapsed {elapsedLabel}</span>
              <span aria-hidden>•</span>
              <span>{logs.length} lines</span>
            </SheetDescription>
          </div>
        </div>
        <div className="flex items-center gap-2 pt-1">
          <Button variant="outline" size="sm" onClick={copyAll} className="gap-1">
            <Copy className="h-3.5 w-3.5" />
            Copy
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={downloadAll}
            className="gap-1"
          >
            <Download className="h-3.5 w-3.5" />
            Download
          </Button>
          {target.href && (
            <Button variant="outline" size="sm" asChild className="gap-1">
              <Link to={target.href}>
                <ExternalLink className="h-3.5 w-3.5" />
                Open detail
              </Link>
            </Button>
          )}
          {!connected && !ended && (
            <Button variant="ghost" size="sm" onClick={connect}>
              Reconnect
            </Button>
          )}
        </div>
      </SheetHeader>
      <div className="flex-1 overflow-hidden bg-gray-950 p-3">
        <div className="h-full overflow-y-auto rounded-md bg-gray-900 p-3 font-mono text-xs">
          {logs.length === 0 ? (
            <div className="py-8 text-center text-gray-500">
              {connected ? "Waiting for logs..." : "Connecting..."}
            </div>
          ) : (
            <div className="space-y-1">
              {logs.map((l) => (
                <div key={l.uid} className="flex gap-2 text-gray-300">
                  <span className="flex-shrink-0 text-gray-500">
                    {formatTimestamp(l.timestamp)}
                  </span>
                  <span
                    className={`flex-shrink-0 rounded px-1.5 py-0.5 text-[10px] uppercase text-white ${
                      levelColors[l.level] ?? "bg-gray-500"
                    }`}
                  >
                    {l.level}
                  </span>
                  <span className="break-all whitespace-pre-wrap">{l.message}</span>
                </div>
              ))}
              <div ref={logsEndRef} />
            </div>
          )}
        </div>
      </div>
    </>
  );
}

function defaultSubtitle(target: DeployPanelTarget): string {
  switch (target.kind) {
    case "deployment":
      return "App deployment";
    case "service":
      return "Service start";
    case "database":
      return "Database start";
  }
}

function formatTimestamp(ts: string): string {
  if (!ts) return "";
  try {
    return new Date(ts).toLocaleTimeString();
  } catch {
    return ts;
  }
}

function formatElapsed(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${m}m ${s}s`;
}
