import { buildThemeOutput, THEME_STYLE_ID, type ThemeOutput } from "./palette";
import type { FontFamily, ThemeMode, ThemeSettings } from "./types";

export function systemPrefersDark(): boolean {
  if (typeof window === "undefined" || !window.matchMedia) return false;
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

export function resolveMode(mode: ThemeMode): "light" | "dark" {
  if (mode === "system") return systemPrefersDark() ? "dark" : "light";
  return mode;
}

const loadedFonts = new Set<FontFamily>(["geist", "system"]);

/** Fetch the webfont CSS for Inter / IBM Plex only when someone picks it. */
export function ensureFont(font: FontFamily) {
  if (loadedFonts.has(font)) return;
  loadedFonts.add(font);
  const load =
    font === "inter"
      ? import("@fontsource-variable/inter")
      : import("@fontsource-variable/ibm-plex-sans");
  load.catch(() => loadedFonts.delete(font));
}

/** Write the theme onto <html>: colour stylesheet, data attributes and the dark class. */
export function applyTheme(settings: ThemeSettings, output: ThemeOutput = buildThemeOutput(settings)) {
  if (typeof document === "undefined") return;
  const root = document.documentElement;

  let style = document.getElementById(THEME_STYLE_ID) as HTMLStyleElement | null;
  if (!style) {
    style = document.createElement("style");
    style.id = THEME_STYLE_ID;
    document.head.appendChild(style);
  }
  if (style.textContent !== output.css) style.textContent = output.css;

  for (const [key, value] of Object.entries(output.attrs)) {
    if (root.getAttribute(key) !== value) root.setAttribute(key, value);
  }

  root.classList.toggle("dark", resolveMode(settings.mode) === "dark");
  ensureFont(settings.font);
}
