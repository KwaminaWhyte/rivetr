import { useState, useEffect, useRef } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { serversApi } from "@/lib/api/servers";
import type {
  Server,
  ServerHealthResponse,
  CreateServerRequest,
  PatchesResponse,
  SecurityCheckItem,
  SecurityCheckResponse,
} from "@/lib/api/servers";
import {
  Server as ServerIcon,
  Plus,
  Trash2,
  RefreshCw,
  Loader2,
  Cpu,
  MemoryStick,
  HardDrive,
  Terminal,
  ShieldCheck,
  PackageSearch,
  CheckCircle2,
  XCircle,
  AlertTriangle,
  HelpCircle,
  ChevronDown,
  ChevronUp,
  Container,
  Download,
} from "lucide-react";
import { ContainerTerminal } from "@/components/container-terminal";

export function meta() {
  return [
    { title: "Servers - Rivetr" },
    { name: "description", content: "Manage remote servers for multi-server deployments" },
  ];
}

function formatDate(dateString: string) {
  return new Date(dateString).toLocaleString();
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  }
  if (bytes >= 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
  return `${(bytes / 1024).toFixed(1)} KB`;
}

function StatusDot({ status }: { status: Server["status"] }) {
  const colorClass =
    status === "online"
      ? "bg-green-500"
      : status === "offline"
      ? "bg-red-500"
      : "bg-yellow-400";
  return (
    <span className={`inline-block w-2.5 h-2.5 rounded-full ${colorClass}`} />
  );
}

function StatusBadge({ status }: { status: Server["status"] }) {
  const variant =
    status === "online"
      ? "default"
      : status === "offline"
      ? "destructive"
      : "secondary";
  return (
    <Badge variant={variant} className="gap-1.5 capitalize">
      <StatusDot status={status} />
      {status}
    </Badge>
  );
}

// ---------------------------------------------------------------------------
// SSH Terminal component for servers
// ---------------------------------------------------------------------------

interface ServerTerminalProps {
  server: Server;
}

function ServerTerminal({ server }: ServerTerminalProps) {
  const wsUrl = serversApi.getTerminalWsUrl(server.id);
  // Re-use the xterm logic from ContainerTerminal but connect to the server WS.
  // We pass a synthetic "appId"-style prop by piggybacking on the existing
  // ContainerTerminal component's URL builder — instead we inline a minimal
  // version that accepts an explicit wsUrl.
  return <ServerTerminalInner wsUrl={wsUrl} label={`SSH: ${server.username}@${server.host}`} />;
}

interface ServerTerminalInnerProps {
  wsUrl: string;
  label: string;
}

function ServerTerminalInner({ wsUrl, label }: ServerTerminalInnerProps) {
  // Delegate to ContainerTerminal with a hack: pass the full WS URL via a
  // custom prop. Since ContainerTerminal builds the URL from appId, we instead
  // create a thin wrapper that overrides the ws URL via a ref.
  // For simplicity we just use ContainerTerminal with a server-flavoured appId
  // sentinel and intercept via the exported wsUrl.
  // The cleanest approach: re-export the terminal UI with a wsUrl prop.
  // We implement it inline here.
  const terminalRef = useRef<HTMLDivElement>(null);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const terminalInstance = useRef<any>(null);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const fitAddonRef = useRef<any>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const isInitializedRef = useRef(false);
  const [connected, setConnected] = useState(false);
  const [disconnected, setDisconnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    if (isInitializedRef.current || !terminalRef.current || typeof window === "undefined") return;
    isInitializedRef.current = true;

    let ws: WebSocket | null = null;
    let resizeObserver: ResizeObserver | null = null;

    const sendResize = () => {
      if (wsRef.current?.readyState === WebSocket.OPEN && terminalInstance.current) {
        const { cols, rows } = terminalInstance.current;
        wsRef.current.send(JSON.stringify({ type: "resize", cols, rows }));
      }
    };

    const handleResize = () => {
      if (fitAddonRef.current && terminalInstance.current) {
        fitAddonRef.current.fit();
        sendResize();
      }
    };

    const initTerminal = async () => {
      try {
        const [xtermModule, fitAddonModule] = await Promise.all([
          import("@xterm/xterm"),
          import("@xterm/addon-fit"),
        ]);
        await import("@xterm/xterm/css/xterm.css");

        const Terminal = xtermModule.Terminal;
        const FitAddon = fitAddonModule.FitAddon;

        const term = new Terminal({
          cursorBlink: true,
          cursorStyle: "block",
          fontFamily: '"Fira Code", "JetBrains Mono", Menlo, Monaco, "Courier New", monospace',
          fontSize: 14,
          lineHeight: 1.2,
          theme: {
            background: "#1a1a2e",
            foreground: "#e0e0e0",
            cursor: "#e0e0e0",
            cursorAccent: "#1a1a2e",
            selectionBackground: "#3b3b5c",
          },
        });

        const fit = new FitAddon();
        term.loadAddon(fit);
        term.open(terminalRef.current!);
        fit.fit();
        terminalInstance.current = term;
        fitAddonRef.current = fit;
        setIsLoading(false);

        term.writeln(`\x1b[1;34m[Rivetr SSH Terminal]\x1b[0m Connecting to ${label}...`);

        ws = new WebSocket(wsUrl);
        wsRef.current = ws;

        ws.onopen = () => {
          setConnected(true);
          setError(null);
          setTimeout(sendResize, 100);
        };

        ws.onmessage = (event) => {
          try {
            const msg = JSON.parse(event.data);
            if (msg.type === "connected") {
              term.writeln(`\x1b[1;32m[Connected]\x1b[0m ${msg.host}`);
              term.writeln("");
            } else if (msg.type === "data" && msg.data) {
              term.write(msg.data);
            } else if (msg.type === "error") {
              term.writeln(`\x1b[1;31m[Error]\x1b[0m ${msg.message}`);
              setError(msg.message || "Unknown error");
              setConnected(false);
            } else if (msg.type === "end") {
              term.writeln("");
              term.writeln(`\x1b[1;33m[Session Ended]\x1b[0m ${msg.message || ""}`);
              setConnected(false);
              setDisconnected(true);
            }
          } catch {
            term.write(event.data);
          }
        };

        ws.onerror = () => {
          setError("WebSocket connection error");
          setConnected(false);
          term.writeln("\x1b[1;31m[Error]\x1b[0m Failed to connect to server");
        };

        ws.onclose = () => {
          setConnected(false);
          setDisconnected(true);
          term.writeln("");
          term.writeln("\x1b[1;33m[Disconnected]\x1b[0m SSH session closed");
        };

        term.onData((data: string) => {
          if (ws?.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify({ type: "data", data }));
          }
        });

        resizeObserver = new ResizeObserver(() => handleResize());
        resizeObserver.observe(terminalRef.current!);
        window.addEventListener("resize", handleResize);
      } catch (err) {
        console.error("Failed to initialize terminal:", err);
        setError("Failed to load terminal");
        setIsLoading(false);
      }
    };

    initTerminal();

    return () => {
      isInitializedRef.current = false;
      resizeObserver?.disconnect();
      window.removeEventListener("resize", handleResize);
      ws?.close();
      terminalInstance.current?.dispose();
      terminalInstance.current = null;
      fitAddonRef.current = null;
      wsRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [wsUrl]);

  return (
    <div className="flex flex-col" style={{ height: "450px" }}>
      <div className="flex items-center justify-between px-4 py-2 bg-gray-800 border-b border-gray-700 rounded-t-lg flex-shrink-0">
        <div className="flex items-center gap-2">
          <div className="flex gap-1.5">
            <div className="w-3 h-3 rounded-full bg-red-500" />
            <div className="w-3 h-3 rounded-full bg-yellow-500" />
            <div className="w-3 h-3 rounded-full bg-green-500" />
          </div>
          <span className="text-sm text-gray-400 ml-2">{label}</span>
        </div>
        <span
          className={`inline-flex items-center px-2 py-1 text-xs font-medium rounded ${
            connected
              ? "bg-green-900/50 text-green-400"
              : error
              ? "bg-red-900/50 text-red-400"
              : disconnected
              ? "bg-yellow-900/50 text-yellow-400"
              : "bg-gray-700 text-gray-400"
          }`}
        >
          <span
            className={`w-2 h-2 rounded-full mr-1.5 ${
              connected ? "bg-green-400" : error ? "bg-red-400" : disconnected ? "bg-yellow-400" : "bg-gray-400"
            }`}
          />
          {connected ? "Connected" : error ? "Error" : disconnected ? "Disconnected" : isLoading ? "Loading..." : "Connecting..."}
        </span>
      </div>
      <div ref={terminalRef} className="flex-1 bg-[#1a1a2e] rounded-b-lg p-2 overflow-hidden">
        {isLoading && (
          <div className="flex items-center justify-center h-full text-gray-400">
            <div className="animate-pulse">Loading terminal...</div>
          </div>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main page component
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Patches Dialog
// ---------------------------------------------------------------------------

function PatchCountBadge({ count }: { count: number }) {
  if (count === 0) {
    return (
      <Badge variant="default" className="bg-green-600 hover:bg-green-600 text-white">
        0 security updates
      </Badge>
    );
  }
  if (count <= 10) {
    return (
      <Badge variant="secondary" className="bg-yellow-100 text-yellow-800 dark:bg-yellow-900/40 dark:text-yellow-300">
        {count} security update{count > 1 ? "s" : ""}
      </Badge>
    );
  }
  return (
    <Badge variant="destructive">
      {count} security updates
    </Badge>
  );
}

function PatchesDialogContent({ serverId }: { serverId: string }) {
  const [expanded, setExpanded] = useState(false);
  const {
    data,
    isLoading,
    error,
    refetch,
    isFetching,
  } = useQuery<PatchesResponse>({
    queryKey: ["server-patches", serverId],
    queryFn: () => serversApi.checkPatches(serverId),
    staleTime: 5 * 60 * 1000, // 5 min
    enabled: true,
  });

  return (
    <div className="space-y-4 py-2">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          {isLoading || isFetching ? (
            <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
          ) : data ? (
            <PatchCountBadge count={data.security_updates} />
          ) : null}
          {data && (
            <span className="text-sm text-muted-foreground">
              {data.total_updates} total upgradable package{data.total_updates !== 1 ? "s" : ""}
            </span>
          )}
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => refetch()}
          disabled={isFetching}
          className="gap-1"
        >
          {isFetching ? (
            <Loader2 className="h-3 w-3 animate-spin" />
          ) : (
            <RefreshCw className="h-3 w-3" />
          )}
          Refresh
        </Button>
      </div>

      {error && (
        <div className="rounded-md border border-destructive/40 bg-destructive/5 px-4 py-3 text-sm text-destructive">
          Failed to reach server. Ensure SSH access is configured.
        </div>
      )}

      {data && data.packages.length > 0 && (
        <div className="space-y-2">
          <button
            className="flex items-center gap-1 text-sm font-medium hover:underline"
            onClick={() => setExpanded((p) => !p)}
            type="button"
          >
            {expanded ? (
              <ChevronUp className="h-4 w-4" />
            ) : (
              <ChevronDown className="h-4 w-4" />
            )}
            {expanded ? "Hide" : "Show"} package list ({data.packages.length})
          </button>
          {expanded && (
            <div className="max-h-64 overflow-y-auto rounded-md border bg-muted/30 p-3">
              {data.packages.map((pkg, i) => (
                <div
                  key={i}
                  className={`py-1 text-xs font-mono border-b last:border-b-0 ${
                    pkg.includes("security")
                      ? "text-amber-600 dark:text-amber-400"
                      : "text-muted-foreground"
                  }`}
                >
                  {pkg}
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {data && data.packages.length === 0 && !isLoading && (
        <div className="flex items-center gap-2 rounded-md border border-green-200 bg-green-50 dark:border-green-800 dark:bg-green-950/30 px-4 py-3 text-sm text-green-700 dark:text-green-400">
          <CheckCircle2 className="h-4 w-4 flex-shrink-0" />
          System is up to date — no pending packages.
        </div>
      )}

      {data && (
        <p className="text-xs text-muted-foreground">
          Checked at {new Date(data.checked_at).toLocaleString()}
        </p>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Security Checklist Dialog
// ---------------------------------------------------------------------------

function SecurityStatusIcon({ status }: { status: string }) {
  switch (status) {
    case "pass":
      return <CheckCircle2 className="h-4 w-4 text-green-500 flex-shrink-0" />;
    case "fail":
      return <XCircle className="h-4 w-4 text-red-500 flex-shrink-0" />;
    case "warn":
      return <AlertTriangle className="h-4 w-4 text-yellow-500 flex-shrink-0" />;
    default:
      return <HelpCircle className="h-4 w-4 text-muted-foreground flex-shrink-0" />;
  }
}

function SecurityCheckDialogContent({ serverId }: { serverId: string }) {
  const {
    data,
    isLoading,
    error,
    refetch,
    isFetching,
  } = useQuery<SecurityCheckResponse>({
    queryKey: ["server-security", serverId],
    queryFn: () => serversApi.checkSecurity(serverId),
    staleTime: 5 * 60 * 1000,
    enabled: true,
  });

  const passCount = data?.items.filter((i) => i.status === "pass").length ?? 0;
  const failCount = data?.items.filter((i) => i.status === "fail").length ?? 0;
  const warnCount = data?.items.filter((i) => i.status === "warn").length ?? 0;

  return (
    <div className="space-y-4 py-2">
      <div className="flex items-center justify-between">
        {data && (
          <div className="flex items-center gap-2 text-sm">
            <span className="flex items-center gap-1 text-green-600 dark:text-green-400 font-medium">
              <CheckCircle2 className="h-3.5 w-3.5" />
              {passCount} passed
            </span>
            {warnCount > 0 && (
              <span className="flex items-center gap-1 text-yellow-600 dark:text-yellow-400 font-medium">
                <AlertTriangle className="h-3.5 w-3.5" />
                {warnCount} warnings
              </span>
            )}
            {failCount > 0 && (
              <span className="flex items-center gap-1 text-red-600 dark:text-red-400 font-medium">
                <XCircle className="h-3.5 w-3.5" />
                {failCount} failed
              </span>
            )}
          </div>
        )}
        <Button
          variant="outline"
          size="sm"
          onClick={() => refetch()}
          disabled={isFetching}
          className="gap-1 ml-auto"
        >
          {isFetching ? (
            <Loader2 className="h-3 w-3 animate-spin" />
          ) : (
            <ShieldCheck className="h-3 w-3" />
          )}
          Run Check
        </Button>
      </div>

      {(isLoading || isFetching) && (
        <div className="flex items-center gap-2 text-sm text-muted-foreground py-4">
          <Loader2 className="h-4 w-4 animate-spin" />
          Running security checks via SSH…
        </div>
      )}

      {error && (
        <div className="rounded-md border border-destructive/40 bg-destructive/5 px-4 py-3 text-sm text-destructive">
          Failed to reach server. Ensure SSH access is configured.
        </div>
      )}

      {data && (
        <div className="space-y-2">
          {data.items.map((item: SecurityCheckItem) => (
            <div
              key={item.id}
              className={`rounded-md border px-4 py-3 ${
                item.status === "fail"
                  ? "border-red-200 bg-red-50/50 dark:border-red-800 dark:bg-red-950/20"
                  : item.status === "warn"
                  ? "border-yellow-200 bg-yellow-50/50 dark:border-yellow-800 dark:bg-yellow-950/20"
                  : item.status === "pass"
                  ? "border-green-200 bg-green-50/50 dark:border-green-800 dark:bg-green-950/20"
                  : "border-border bg-muted/20"
              }`}
            >
              <div className="flex items-center gap-2">
                <SecurityStatusIcon status={item.status} />
                <span className="text-sm font-medium">{item.name}</span>
              </div>
              <p className="mt-0.5 pl-6 text-xs text-muted-foreground">{item.description}</p>
              {item.details && (
                <p
                  className={`mt-1 pl-6 text-xs font-mono ${
                    item.status === "fail"
                      ? "text-red-700 dark:text-red-400"
                      : item.status === "warn"
                      ? "text-yellow-700 dark:text-yellow-400"
                      : "text-green-700 dark:text-green-400"
                  }`}
                >
                  {item.details}
                </p>
              )}
            </div>
          ))}
        </div>
      )}

      {data && (
        <p className="text-xs text-muted-foreground">
          Checked at {new Date(data.checked_at).toLocaleString()}
        </p>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main page component
// ---------------------------------------------------------------------------

export default function ServersPage() {
  const queryClient = useQueryClient();
  const [addDialogOpen, setAddDialogOpen] = useState(false);
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const [checkingId, setCheckingId] = useState<string | null>(null);
  const [installingDockerIds, setInstallingDockerIds] = useState<Set<string>>(new Set());
  // Map from server id → last health check result (for docker status)
  const [dockerHealth, setDockerHealth] = useState<Record<string, Pick<ServerHealthResponse, "docker_installed" | "docker_running" | "compose_installed" | "compose_version">>>({});
  const [terminalServer, setTerminalServer] = useState<Server | null>(null);
  const [patchesServer, setPatchesServer] = useState<Server | null>(null);
  const [securityServer, setSecurityServer] = useState<Server | null>(null);

  // Form state
  const [formName, setFormName] = useState("");
  const [formHost, setFormHost] = useState("");
  const [formPort, setFormPort] = useState("22");
  const [formUsername, setFormUsername] = useState("root");
  const [formSshKey, setFormSshKey] = useState("");
  const [authMethod, setAuthMethod] = useState<"key" | "password">("key");
  const [sshPassword, setSshPassword] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);

  const { data: servers = [], isLoading } = useQuery<Server[]>({
    queryKey: ["servers"],
    queryFn: () => serversApi.list(),
  });

  const createMutation = useMutation({
    mutationFn: (data: CreateServerRequest) => serversApi.create(data),
    onSuccess: () => {
      toast.success("Server added successfully");
      queryClient.invalidateQueries({ queryKey: ["servers"] });
      resetForm();
      setAddDialogOpen(false);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to add server");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => serversApi.delete(id),
    onSuccess: () => {
      toast.success("Server removed");
      queryClient.invalidateQueries({ queryKey: ["servers"] });
      setDeleteId(null);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to remove server");
    },
  });

  const resetForm = () => {
    setFormName("");
    setFormHost("");
    setFormPort("22");
    setFormUsername("root");
    setFormSshKey("");
    setAuthMethod("key");
    setSshPassword("");
  };

  const handleAddServer = async () => {
    if (!formName.trim() || !formHost.trim()) {
      toast.error("Name and host are required");
      return;
    }
    setIsSubmitting(true);
    try {
      const port = parseInt(formPort, 10);
      await createMutation.mutateAsync({
        name: formName.trim(),
        host: formHost.trim(),
        port: isNaN(port) ? 22 : port,
        username: formUsername.trim() || "root",
        ssh_private_key: authMethod === "key" ? formSshKey.trim() || undefined : undefined,
        ssh_password: authMethod === "password" ? sshPassword : undefined,
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleCheckHealth = async (server: Server) => {
    setCheckingId(server.id);
    try {
      const result = await serversApi.check(server.id);
      toast.success(`Health check complete for ${server.name}`);
      queryClient.invalidateQueries({ queryKey: ["servers"] });
      // Store docker status fields from health check response
      setDockerHealth((prev) => ({
        ...prev,
        [server.id]: {
          docker_installed: result.docker_installed,
          docker_running: result.docker_running,
          compose_installed: result.compose_installed,
          compose_version: result.compose_version,
        },
      }));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Health check failed");
    } finally {
      setCheckingId(null);
    }
  };

  const handleInstallDocker = async (server: Server) => {
    setInstallingDockerIds((prev) => new Set(prev).add(server.id));
    try {
      const result = await serversApi.installDocker(server.id);
      if (result.success) {
        toast.success(`Docker installed successfully on ${server.name}`);
        // Trigger a health re-check to update docker status
        await handleCheckHealth(server);
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to install Docker");
    } finally {
      setInstallingDockerIds((prev) => {
        const next = new Set(prev);
        next.delete(server.id);
        return next;
      });
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Remote Servers</h1>
        <p className="text-muted-foreground">
          Register and manage remote servers for multi-server deployments.
        </p>
      </div>

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <ServerIcon className="h-5 w-5" />
                Servers
              </CardTitle>
              <CardDescription>
                Add remote servers accessible via SSH to deploy applications across your infrastructure.
              </CardDescription>
            </div>
            <Button onClick={() => setAddDialogOpen(true)} className="gap-2">
              <Plus className="h-4 w-4" />
              Add Server
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : servers.length === 0 ? (
            <div className="text-center py-12 space-y-4">
              <ServerIcon className="h-12 w-12 mx-auto text-muted-foreground/50" />
              <div>
                <p className="text-lg font-medium">No Servers Registered</p>
                <p className="text-sm text-muted-foreground">
                  Add a remote server to start deploying across multiple machines.
                </p>
              </div>
              <Button onClick={() => setAddDialogOpen(true)} className="gap-2">
                <Plus className="h-4 w-4" />
                Add Server
              </Button>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Host</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Resources</TableHead>
                  <TableHead>Last Seen</TableHead>
                  <TableHead className="w-[160px]"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {servers.map((server) => (
                  <TableRow key={server.id}>
                    <TableCell className="font-medium">{server.name}</TableCell>
                    <TableCell className="text-muted-foreground font-mono text-sm">
                      {server.host}:{server.port}
                    </TableCell>
                    <TableCell>
                      <StatusBadge status={server.status} />
                    </TableCell>
                    <TableCell>
                      {server.status === "online" ? (
                        <div className="flex items-center gap-3 text-xs text-muted-foreground">
                          {server.cpu_usage != null && (
                            <span className="flex items-center gap-1">
                              <Cpu className="h-3 w-3" />
                              {server.cpu_usage.toFixed(1)}%
                            </span>
                          )}
                          {server.memory_usage != null && (
                            <span className="flex items-center gap-1">
                              <MemoryStick className="h-3 w-3" />
                              {server.memory_usage.toFixed(1)}%
                            </span>
                          )}
                          {server.disk_usage != null && (
                            <span className="flex items-center gap-1">
                              <HardDrive className="h-3 w-3" />
                              {server.disk_usage.toFixed(1)}%
                            </span>
                          )}
                          {server.memory_total != null && (
                            <span className="text-muted-foreground/70">
                              {formatBytes(server.memory_total)} RAM
                            </span>
                          )}
                        </div>
                      ) : (
                        <span className="text-xs text-muted-foreground">—</span>
                      )}
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {server.last_seen_at ? formatDate(server.last_seen_at) : "Never"}
                    </TableCell>
                    <TableCell>
                      {/* Docker status badges (shown after a health check) */}
                      {dockerHealth[server.id] && (
                        <div className="flex items-center gap-1 flex-wrap mb-1.5">
                          <Badge
                            variant={dockerHealth[server.id].docker_installed ? "default" : "secondary"}
                            className={`text-xs gap-1 ${dockerHealth[server.id].docker_installed ? "bg-blue-600 hover:bg-blue-600 text-white" : "bg-muted text-muted-foreground"}`}
                            title="Docker CLI installed"
                          >
                            <Container className="h-3 w-3" />
                            {dockerHealth[server.id].docker_installed ? "Docker installed" : "Docker not installed"}
                          </Badge>
                          {dockerHealth[server.id].docker_installed && (
                            <Badge
                              variant={dockerHealth[server.id].docker_running ? "default" : "destructive"}
                              className={`text-xs gap-1 ${dockerHealth[server.id].docker_running ? "bg-green-600 hover:bg-green-600 text-white" : ""}`}
                              title="Docker daemon status"
                            >
                              {dockerHealth[server.id].docker_running ? "Daemon running" : "Daemon stopped"}
                            </Badge>
                          )}
                          {dockerHealth[server.id].compose_installed && (
                            <Badge
                              variant="outline"
                              className="text-xs"
                              title="Docker Compose available"
                            >
                              Compose {dockerHealth[server.id].compose_version ?? ""}
                            </Badge>
                          )}
                        </div>
                      )}
                      <div className="flex items-center gap-1 flex-wrap">
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => setTerminalServer(server)}
                          className="gap-1"
                          title="Open SSH terminal"
                        >
                          <Terminal className="h-3 w-3" />
                          Terminal
                        </Button>
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => setPatchesServer(server)}
                          className="gap-1"
                          title="Check for OS updates"
                        >
                          <PackageSearch className="h-3 w-3" />
                          Updates
                        </Button>
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => setSecurityServer(server)}
                          className="gap-1"
                          title="Run security checklist"
                        >
                          <ShieldCheck className="h-3 w-3" />
                          Security
                        </Button>
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => handleCheckHealth(server)}
                          disabled={checkingId === server.id || installingDockerIds.has(server.id)}
                          className="gap-1"
                        >
                          {checkingId === server.id ? (
                            <Loader2 className="h-3 w-3 animate-spin" />
                          ) : (
                            <RefreshCw className="h-3 w-3" />
                          )}
                          Check
                        </Button>
                        {/* Show Install Docker button when health check ran and docker is not installed */}
                        {dockerHealth[server.id] && !dockerHealth[server.id].docker_installed && (
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => handleInstallDocker(server)}
                            disabled={installingDockerIds.has(server.id) || checkingId === server.id}
                            className="gap-1 border-blue-300 text-blue-700 hover:bg-blue-50 dark:border-blue-700 dark:text-blue-400 dark:hover:bg-blue-950/30"
                            title="Install Docker via get.docker.com"
                          >
                            {installingDockerIds.has(server.id) ? (
                              <Loader2 className="h-3 w-3 animate-spin" />
                            ) : (
                              <Download className="h-3 w-3" />
                            )}
                            {installingDockerIds.has(server.id) ? "Installing…" : "Install Docker"}
                          </Button>
                        )}
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => setDeleteId(server.id)}
                          className="text-destructive hover:text-destructive hover:bg-destructive/10"
                        >
                          <Trash2 className="h-3 w-3" />
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Add Server Dialog */}
      <Dialog open={addDialogOpen} onOpenChange={(open) => { setAddDialogOpen(open); if (!open) resetForm(); }}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <ServerIcon className="h-5 w-5" />
              Add Remote Server
            </DialogTitle>
            <DialogDescription>
              Register a remote server accessible via SSH.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="grid grid-cols-2 gap-4">
              <div className="col-span-2 space-y-2">
                <Label htmlFor="server-name">Name</Label>
                <Input
                  id="server-name"
                  placeholder="production-us-east"
                  value={formName}
                  onChange={(e) => setFormName(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="server-host">Host / IP</Label>
                <Input
                  id="server-host"
                  placeholder="192.168.1.100"
                  value={formHost}
                  onChange={(e) => setFormHost(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="server-port">SSH Port</Label>
                <Input
                  id="server-port"
                  type="number"
                  placeholder="22"
                  value={formPort}
                  onChange={(e) => setFormPort(e.target.value)}
                />
              </div>
              <div className="col-span-2 space-y-2">
                <Label htmlFor="server-username">SSH Username</Label>
                <Input
                  id="server-username"
                  placeholder="root"
                  value={formUsername}
                  onChange={(e) => setFormUsername(e.target.value)}
                />
              </div>
              <div className="col-span-2 space-y-2">
                <Label>Authentication Method</Label>
                <Select value={authMethod} onValueChange={(v) => setAuthMethod(v as "key" | "password")}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="key">SSH Private Key</SelectItem>
                    <SelectItem value="password">Password</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {authMethod === "key" ? (
                <div className="col-span-2 space-y-2">
                  <Label htmlFor="server-ssh-key">SSH Private Key</Label>
                  <Textarea
                    id="server-ssh-key"
                    placeholder="Paste your private key here (PEM format)..."
                    className="font-mono text-xs h-32"
                    value={formSshKey}
                    onChange={(e) => setFormSshKey(e.target.value)}
                  />
                  <p className="text-xs text-muted-foreground">Optional. If not provided, uses the system default key.</p>
                </div>
              ) : (
                <div className="col-span-2 space-y-2">
                  <Label htmlFor="server-ssh-password">SSH Password</Label>
                  <Input
                    id="server-ssh-password"
                    type="password"
                    placeholder="SSH password"
                    value={sshPassword}
                    onChange={(e) => setSshPassword(e.target.value)}
                  />
                </div>
              )}
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => { setAddDialogOpen(false); resetForm(); }}
            >
              Cancel
            </Button>
            <Button
              onClick={handleAddServer}
              disabled={isSubmitting || !formName.trim() || !formHost.trim()}
              className="gap-2"
            >
              {isSubmitting ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Plus className="h-4 w-4" />
              )}
              Add Server
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* SSH Terminal Dialog */}
      <Dialog
        open={!!terminalServer}
        onOpenChange={(open) => { if (!open) setTerminalServer(null); }}
      >
        <DialogContent
          className="sm:max-w-4xl"
          onInteractOutside={(e) => e.preventDefault()}
        >
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Terminal className="h-5 w-5" />
              SSH Terminal — {terminalServer?.name}
            </DialogTitle>
            <DialogDescription>
              Interactive SSH session to {terminalServer?.username}@{terminalServer?.host}
            </DialogDescription>
          </DialogHeader>
          <div className="py-2">
            {terminalServer && <ServerTerminal server={terminalServer} />}
          </div>
        </DialogContent>
      </Dialog>

      {/* OS Patches Dialog */}
      <Dialog
        open={!!patchesServer}
        onOpenChange={(open) => { if (!open) setPatchesServer(null); }}
      >
        <DialogContent className="sm:max-w-2xl">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <PackageSearch className="h-5 w-5" />
              OS Updates — {patchesServer?.name}
            </DialogTitle>
            <DialogDescription>
              Pending package upgrades on {patchesServer?.host}
            </DialogDescription>
          </DialogHeader>
          {patchesServer && <PatchesDialogContent serverId={patchesServer.id} />}
        </DialogContent>
      </Dialog>

      {/* Security Checklist Dialog */}
      <Dialog
        open={!!securityServer}
        onOpenChange={(open) => { if (!open) setSecurityServer(null); }}
      >
        <DialogContent className="sm:max-w-2xl">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <ShieldCheck className="h-5 w-5" />
              Security Checklist — {securityServer?.name}
            </DialogTitle>
            <DialogDescription>
              Security best-practice audit for {securityServer?.host}
            </DialogDescription>
          </DialogHeader>
          {securityServer && <SecurityCheckDialogContent serverId={securityServer.id} />}
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <AlertDialog open={!!deleteId} onOpenChange={() => setDeleteId(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Remove Server</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to remove this server? All app assignments for this
              server will also be removed. This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                if (deleteId) {
                  deleteMutation.mutate(deleteId);
                }
              }}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Remove
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
