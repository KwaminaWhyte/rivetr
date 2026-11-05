import { useEffect, useRef, useState } from "react";
import { Minus, Plus } from "lucide-react";
import { usePersistedState } from "@/lib/persisted-state";

const MIN_FONT = 10;
const MAX_FONT = 24;
const isFontSize = (v: unknown): v is number =>
  Number.isInteger(v) && (v as number) >= MIN_FONT && (v as number) <= MAX_FONT;
import { api } from "@/lib/api";
import { getAuthToken } from "@/lib/auth";

interface TerminalMessage {
  type: "data" | "connected" | "end" | "error";
  data?: string;
  message?: string;
  container_id?: string;
  app_id?: string;
}

interface ContainerTerminalProps {
  appId?: string;
  token?: string;
  /** Build the WebSocket URL for another target (e.g. one service container). */
  wsUrlFor?: (token: string) => string;
}

export function ContainerTerminal({ appId, token: propToken, wsUrlFor }: ContainerTerminalProps) {
  // Get token from props or localStorage
  const token = propToken || getAuthToken() || "";
  const terminalRef = useRef<HTMLDivElement>(null);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const terminalInstance = useRef<any>(null);
  // Font size is remembered per browser (rivetr:ui:terminal:font-size).
  const [fontSize, setFontSize] = usePersistedState<number>("terminal:font-size", 14, { validate: isFontSize });
  const fontSizeRef = useRef(fontSize);
  fontSizeRef.current = fontSize;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const fitAddonRef = useRef<any>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const isInitializedRef = useRef(false);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    // Prevent re-initialization if already initialized with same appId
    if (isInitializedRef.current || !terminalRef.current || typeof window === "undefined") {
      return;
    }

    isInitializedRef.current = true;

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let term: any = null;
    let ws: WebSocket | null = null;
    let resizeObserver: ResizeObserver | null = null;

    // Send resize event to the server
    const sendResize = () => {
      if (wsRef.current?.readyState === WebSocket.OPEN && terminalInstance.current) {
        const { cols, rows } = terminalInstance.current;
        wsRef.current.send(JSON.stringify({ type: "resize", cols, rows }));
      }
    };

    // Handle terminal resize
    const handleResize = () => {
      if (fitAddonRef.current && terminalInstance.current) {
        fitAddonRef.current.fit();
        sendResize();
      }
    };

    const initTerminal = async () => {
      try {
        // Dynamically import xterm (browser-only)
        const [xtermModule, fitAddonModule] = await Promise.all([
          import("@xterm/xterm"),
          import("@xterm/addon-fit"),
        ]);

        // Import CSS
        await import("@xterm/xterm/css/xterm.css");

        const Terminal = xtermModule.Terminal;
        const FitAddon = fitAddonModule.FitAddon;

        // Create terminal instance
        term = new Terminal({
          cursorBlink: true,
          cursorStyle: "block",
          fontFamily: '"Fira Code", "JetBrains Mono", Menlo, Monaco, "Courier New", monospace',
          fontSize: fontSizeRef.current,
          lineHeight: 1.2,
          theme: {
            background: "#1a1a2e",
            foreground: "#e0e0e0",
            cursor: "#e0e0e0",
            cursorAccent: "#1a1a2e",
            selectionBackground: "#3b3b5c",
            black: "#1a1a2e",
            red: "#ff6b6b",
            green: "#4ecdc4",
            yellow: "#ffd93d",
            blue: "#6c5ce7",
            magenta: "#a29bfe",
            cyan: "#74b9ff",
            white: "#e0e0e0",
            brightBlack: "#545478",
            brightRed: "#ff8787",
            brightGreen: "#7dede4",
            brightYellow: "#ffe066",
            brightBlue: "#9d8eff",
            brightMagenta: "#c4b9ff",
            brightCyan: "#a3d9ff",
            brightWhite: "#ffffff",
          },
        });

        const fit = new FitAddon();
        term.loadAddon(fit);
        term.open(terminalRef.current!);
        fit.fit();

        terminalInstance.current = term;
        fitAddonRef.current = fit;
        setIsLoading(false);

        // Display welcome message
        term.writeln("\x1b[1;34m[Rivetr Terminal]\x1b[0m Connecting to container...");

        // Connect to WebSocket
        const wsUrl = wsUrlFor ? wsUrlFor(token) : api.getTerminalWsUrl(appId ?? "", token);
        ws = new WebSocket(wsUrl);
        wsRef.current = ws;

        ws.onopen = () => {
          setConnected(true);
          setError(null);
          // Send initial resize
          setTimeout(sendResize, 100);
        };

        ws.onmessage = (event) => {
          try {
            const msg: TerminalMessage = JSON.parse(event.data);
            if (msg.type === "connected") {
              term.writeln(`\x1b[1;32m[Connected]\x1b[0m Container: ${msg.container_id?.slice(0, 12)}`);
              term.writeln("");
            } else if (msg.type === "data" && msg.data) {
              term.write(msg.data);
            } else if (msg.type === "error") {
              term.writeln(`\x1b[1;31m[Error]\x1b[0m ${msg.message}`);
              setError(msg.message || "Unknown error");
              setConnected(false);
            } else if (msg.type === "end") {
              term.writeln("");
              term.writeln(`\x1b[1;33m[Session Ended]\x1b[0m ${msg.message || "Connection closed"}`);
              setConnected(false);
            }
          } catch {
            // If not JSON, treat as raw data
            term.write(event.data);
          }
        };

        ws.onerror = () => {
          setError("WebSocket connection error");
          setConnected(false);
          term.writeln("\x1b[1;31m[Error]\x1b[0m Failed to connect to container");
        };

        ws.onclose = () => {
          setConnected(false);
          term.writeln("");
          term.writeln("\x1b[1;33m[Disconnected]\x1b[0m Terminal session closed");
        };

        // Handle terminal input
        term.onData((data: string) => {
          if (ws?.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify({ type: "data", data }));
          }
        });

        // Handle window resize
        resizeObserver = new ResizeObserver(() => {
          handleResize();
        });
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
      if (resizeObserver) {
        resizeObserver.disconnect();
      }
      window.removeEventListener("resize", handleResize);
      if (ws) {
        ws.close();
      }
      if (term) {
        term.dispose();
      }
      terminalInstance.current = null;
      fitAddonRef.current = null;
      wsRef.current = null;
    };
  }, [appId, token]);

  // Apply font size changes to a live terminal and tell the server the new size.
  useEffect(() => {
    const term = terminalInstance.current;
    if (!term || term.options.fontSize === fontSize) return;
    term.options.fontSize = fontSize;
    fitAddonRef.current?.fit();
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({ type: "resize", cols: term.cols, rows: term.rows }));
    }
  }, [fontSize]);

  return (
    <div className="flex flex-col" style={{ height: "450px" }}>
      <div className="flex items-center justify-between px-4 py-2 bg-gray-800 border-b border-gray-700 rounded-t-lg flex-shrink-0">
        <div className="flex items-center gap-2">
          <div className="flex gap-1.5">
            <div className="w-3 h-3 rounded-full bg-red-500" />
            <div className="w-3 h-3 rounded-full bg-yellow-500" />
            <div className="w-3 h-3 rounded-full bg-green-500" />
          </div>
          <span className="text-sm text-gray-400 ml-2">Container Terminal</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="flex items-center rounded border border-gray-700 text-gray-400">
            <button
              type="button"
              className="px-1.5 py-1 hover:text-gray-100 disabled:opacity-40"
              onClick={() => setFontSize((s) => Math.max(MIN_FONT, s - 1))}
              disabled={fontSize <= MIN_FONT}
              aria-label="Smaller terminal font"
              title="Smaller font"
            >
              <Minus className="size-3" />
            </button>
            <span className="min-w-6 text-center text-[11px] tabular-nums" aria-live="polite">
              {fontSize}
            </span>
            <button
              type="button"
              className="px-1.5 py-1 hover:text-gray-100 disabled:opacity-40"
              onClick={() => setFontSize((s) => Math.min(MAX_FONT, s + 1))}
              disabled={fontSize >= MAX_FONT}
              aria-label="Larger terminal font"
              title="Larger font"
            >
              <Plus className="size-3" />
            </button>
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
      </div>
      <div
        ref={terminalRef}
        className="flex-1 bg-[#1a1a2e] rounded-b-lg p-2 overflow-hidden"
      >
        {isLoading && (
          <div className="flex items-center justify-center h-full text-gray-400">
            <div className="animate-pulse">Loading terminal...</div>
          </div>
        )}
      </div>
    </div>
  );
}
