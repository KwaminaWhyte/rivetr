/**
 * xterm.js theme built from the live dashboard tokens, so terminals follow the
 * user's mode, neutral base and accent colour. Tokens are oklch() in CSS, which
 * xterm cannot parse, so each one is resolved to rgb through a 1x1 canvas.
 */
import { useEffect, useState } from "react";
import { useTheme } from "./theme-provider";

export interface TerminalTheme {
  background: string;
  foreground: string;
  cursor: string;
  cursorAccent: string;
  selectionBackground: string;
  selectionForeground?: string;
  black: string;
  red: string;
  green: string;
  yellow: string;
  blue: string;
  magenta: string;
  cyan: string;
  white: string;
  brightBlack: string;
  brightRed: string;
  brightGreen: string;
  brightYellow: string;
  brightBlue: string;
  brightMagenta: string;
  brightCyan: string;
  brightWhite: string;
}

/**
 * ANSI colours tuned for contrast on each mode. They stay fixed so `ls` and log
 * colours keep their meaning; the accent drives the cursor and selection.
 */
const ANSI = {
  dark: {
    red: "#f87171",
    green: "#4ade80",
    yellow: "#facc15",
    blue: "#60a5fa",
    magenta: "#c084fc",
    cyan: "#22d3ee",
    brightRed: "#fca5a5",
    brightGreen: "#86efac",
    brightYellow: "#fde047",
    brightBlue: "#93c5fd",
    brightMagenta: "#d8b4fe",
    brightCyan: "#67e8f9",
  },
  light: {
    red: "#c81e1e",
    green: "#15803d",
    yellow: "#a16207",
    blue: "#1d4ed8",
    magenta: "#9333ea",
    cyan: "#0e7490",
    brightRed: "#dc2626",
    brightGreen: "#16a34a",
    brightYellow: "#ca8a04",
    brightBlue: "#2563eb",
    brightMagenta: "#a855f7",
    brightCyan: "#0891b2",
  },
} as const;

let ctx: CanvasRenderingContext2D | null = null;

/** Resolve any CSS colour (oklch, color-mix, var-free) to `rgb(r, g, b)` or `rgba(...)`. */
function toRgb(css: string, alpha = 1): string {
  if (typeof document === "undefined") return css;
  if (!ctx) {
    const canvas = document.createElement("canvas");
    canvas.width = canvas.height = 1;
    ctx = canvas.getContext("2d", { willReadFrequently: true });
  }
  if (!ctx) return css;
  ctx.clearRect(0, 0, 1, 1);
  ctx.fillStyle = "#000";
  ctx.fillStyle = css;
  ctx.fillRect(0, 0, 1, 1);
  const [r, g, b] = ctx.getImageData(0, 0, 1, 1).data;
  return alpha >= 1 ? `rgb(${r}, ${g}, ${b})` : `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

function token(styles: CSSStyleDeclaration, name: string, fallback: string): string {
  const v = styles.getPropertyValue(name).trim();
  return v || fallback;
}

/** Build the terminal theme from the tokens currently applied to `<html>`. */
export function buildTerminalTheme(mode: "light" | "dark"): TerminalTheme {
  const styles = getComputedStyle(document.documentElement);
  const background = toRgb(token(styles, "--card", mode === "dark" ? "#18181b" : "#ffffff"));
  const foreground = toRgb(token(styles, "--card-foreground", mode === "dark" ? "#fafafa" : "#18181b"));
  const muted = toRgb(token(styles, "--muted-foreground", "#71717a"));
  const border = toRgb(token(styles, "--border", "#3f3f46"));
  const primary = token(styles, "--primary", "#7c3aed");
  const ansi = ANSI[mode];

  return {
    background,
    foreground,
    cursor: toRgb(primary),
    cursorAccent: background,
    selectionBackground: toRgb(primary, mode === "dark" ? 0.35 : 0.22),
    black: mode === "dark" ? border : foreground,
    red: ansi.red,
    green: ansi.green,
    yellow: ansi.yellow,
    blue: ansi.blue,
    magenta: ansi.magenta,
    cyan: ansi.cyan,
    white: mode === "dark" ? foreground : muted,
    brightBlack: muted,
    brightRed: ansi.brightRed,
    brightGreen: ansi.brightGreen,
    brightYellow: ansi.brightYellow,
    brightBlue: ansi.brightBlue,
    brightMagenta: ansi.brightMagenta,
    brightCyan: ansi.brightCyan,
    brightWhite: mode === "dark" ? "rgb(255, 255, 255)" : foreground,
  };
}

/** The theme's monospace font stack, for xterm's `fontFamily`. */
export function terminalFontFamily(): string {
  if (typeof document === "undefined") return "monospace";
  const v = getComputedStyle(document.documentElement).getPropertyValue("--font-mono").trim();
  return v || '"Geist Mono Variable", ui-monospace, SFMono-Regular, Menlo, monospace';
}

/**
 * Terminal theme that rebuilds whenever the dashboard theme changes. Returns null
 * during SSR. Apply it with `term.options.theme = theme`.
 */
export function useTerminalTheme(): TerminalTheme | null {
  const { settings, resolvedMode } = useTheme();
  const [theme, setTheme] = useState<TerminalTheme | null>(null);
  useEffect(() => {
    // Wait a frame so the provider has written the new tokens to <html>.
    const id = requestAnimationFrame(() => setTheme(buildTerminalTheme(resolvedMode)));
    return () => cancelAnimationFrame(id);
  }, [settings, resolvedMode]);
  return theme;
}
