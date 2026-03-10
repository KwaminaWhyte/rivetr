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
import { serversApi } from "@/lib/api/servers";
import type { Server, CreateServerRequest } from "@/lib/api/servers";
import { Server as ServerIcon, Plus, Trash2, RefreshCw, Loader2, Cpu, MemoryStick, HardDrive, Terminal } from "lucide-react";
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
              : "bg-gray-700 text-gray-400"
          }`}
        >
          <span
            className={`w-2 h-2 rounded-full mr-1.5 ${
              connected ? "bg-green-400" : error ? "bg-red-400" : "bg-gray-400"
            }`}
          />
          {connected ? "Connected" : error ? "Error" : isLoading ? "Loading..." : "Connecting..."}
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

export default function ServersPage() {
  const queryClient = useQueryClient();
  const [addDialogOpen, setAddDialogOpen] = useState(false);
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const [checkingId, setCheckingId] = useState<string | null>(null);
  const [terminalServer, setTerminalServer] = useState<Server | null>(null);

  // Form state
  const [formName, setFormName] = useState("");
  const [formHost, setFormHost] = useState("");
  const [formPort, setFormPort] = useState("22");
  const [formUsername, setFormUsername] = useState("root");
  const [formSshKey, setFormSshKey] = useState("");
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
        ssh_private_key: formSshKey.trim() || undefined,
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleCheckHealth = async (server: Server) => {
    setCheckingId(server.id);
    try {
      await serversApi.check(server.id);
      toast.success(`Health check complete for ${server.name}`);
      queryClient.invalidateQueries({ queryKey: ["servers"] });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Health check failed");
    } finally {
      setCheckingId(null);
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
                      <div className="flex items-center gap-1">
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
                          onClick={() => handleCheckHealth(server)}
                          disabled={checkingId === server.id}
                          className="gap-1"
                        >
                          {checkingId === server.id ? (
                            <Loader2 className="h-3 w-3 animate-spin" />
                          ) : (
                            <RefreshCw className="h-3 w-3" />
                          )}
                          Check
                        </Button>
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
                <Label htmlFor="server-ssh-key">
                  SSH Private Key{" "}
                  <span className="text-muted-foreground font-normal">(optional)</span>
                </Label>
                <Textarea
                  id="server-ssh-key"
                  placeholder="-----BEGIN RSA PRIVATE KEY-----&#10;...&#10;-----END RSA PRIVATE KEY-----"
                  rows={5}
                  value={formSshKey}
                  onChange={(e) => setFormSshKey(e.target.value)}
                  className="font-mono text-xs"
                />
                <p className="text-xs text-muted-foreground">
                  The private key will be encrypted with AES-256-GCM before storage.
                  Leave empty to use the system default SSH key.
                </p>
              </div>
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
        <DialogContent className="sm:max-w-4xl">
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
