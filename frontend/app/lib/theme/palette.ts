/**
 * Turns ThemeSettings into CSS: colour tokens for light and dark, plus the
 * data attributes that app.css keys radius, density, font and motion off.
 */
import { contrastRatio, formatOklch, hexToOklch, type Oklch } from "./color";
import type { AccentPreset, Neutral, ThemeSettings } from "./types";

interface AccentSpec {
  hue: number;
  /** Primary in light mode. */
  light: { l: number; c: number };
  /** Primary in dark mode. */
  dark: { l: number; c: number };
}

export const ACCENT_SPECS: Record<AccentPreset, AccentSpec> = {
  violet: { hue: 292, light: { l: 0.541, c: 0.235 }, dark: { l: 0.64, c: 0.2 } },
  blue: { hue: 259, light: { l: 0.546, c: 0.215 }, dark: { l: 0.64, c: 0.18 } },
  indigo: { hue: 277, light: { l: 0.511, c: 0.23 }, dark: { l: 0.62, c: 0.19 } },
  emerald: { hue: 163, light: { l: 0.56, c: 0.13 }, dark: { l: 0.7, c: 0.15 } },
  teal: { hue: 185, light: { l: 0.56, c: 0.1 }, dark: { l: 0.7, c: 0.12 } },
  amber: { hue: 65, light: { l: 0.72, c: 0.165 }, dark: { l: 0.78, c: 0.16 } },
  orange: { hue: 45, light: { l: 0.63, c: 0.2 }, dark: { l: 0.7, c: 0.18 } },
  rose: { hue: 12, light: { l: 0.586, c: 0.22 }, dark: { l: 0.66, c: 0.2 } },
  slate: { hue: 257, light: { l: 0.372, c: 0.044 }, dark: { l: 0.82, c: 0.03 } },
};

/** Neutral greys: base hue and how much of it to mix in (1 = zinc). */
const NEUTRAL_SPECS: Record<Neutral, { hue: number; k: number }> = {
  zinc: { hue: 286, k: 1 },
  slate: { hue: 257, k: 4 },
  stone: { hue: 56, k: 1.4 },
  neutral: { hue: 0, k: 0 },
  gray: { hue: 264, k: 2.2 },
};

/** The accent the user picked as a spec, deriving one from a custom hex. */
export function accentSpec(settings: Pick<ThemeSettings, "accent" | "customAccent">): AccentSpec {
  if (settings.accent !== "custom") return ACCENT_SPECS[settings.accent];
  const { l, c, h } = hexToOklch(settings.customAccent);
  const chroma = Math.min(c, 0.26);
  const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));
  return {
    hue: chroma < 0.02 ? 286 : h,
    light: { l: clamp(l, 0.4, 0.74), c: chroma },
    dark: { l: clamp(l, 0.6, 0.82), c: Math.min(chroma, 0.22) },
  };
}

const WHITE_ISH = (h: number): Oklch => ({ l: 0.99, c: 0.004, h });
const DARK_ISH = (h: number): Oklch => ({ l: 0.2, c: 0.02, h });

/** Whichever of near-white or near-black reads better on the colour. */
export function readableForeground(bg: Oklch): Oklch {
  const light = WHITE_ISH(bg.h);
  const dark = DARK_ISH(bg.h);
  return contrastRatio(bg, light) >= contrastRatio(bg, dark) ? light : dark;
}

type Tokens = Record<string, string>;

function neutralTokens(neutral: Neutral, dark: boolean, highContrast: boolean): Tokens {
  const { hue, k } = NEUTRAL_SPECS[neutral];
  const n = (l: number, c: number) => formatOklch({ l, c: c * k, h: hue });
  if (!dark) {
    return {
      "--background": n(0.974, 0.002),
      "--foreground": n(highContrast ? 0.1 : 0.18, 0.006),
      "--card": n(1, 0),
      "--card-foreground": n(highContrast ? 0.1 : 0.18, 0.006),
      "--popover": n(1, 0),
      "--popover-foreground": n(0.18, 0.006),
      "--secondary": n(0.955, 0.003),
      "--secondary-foreground": n(0.24, 0.008),
      "--muted": n(0.955, 0.003),
      "--muted-foreground": n(highContrast ? 0.38 : 0.52, 0.014),
      "--border": n(highContrast ? 0.75 : 0.912, 0.004),
      "--input": n(highContrast ? 0.68 : 0.9, 0.004),
      "--field": n(0.962, 0.003),
      "--frame": n(0.953, 0.003),
      "--sidebar": n(0.985, 0.001),
      "--sidebar-foreground": n(highContrast ? 0.12 : 0.22, 0.006),
      "--sidebar-accent": n(0.947, 0.004),
      "--sidebar-accent-foreground": n(0.18, 0.006),
      "--sidebar-border": n(highContrast ? 0.75 : 0.912, 0.004),
    };
  }
  return {
    "--background": n(0.155, 0.004),
    "--foreground": n(highContrast ? 1 : 0.96, 0.002),
    "--card": n(0.198, 0.005),
    "--card-foreground": n(highContrast ? 1 : 0.96, 0.002),
    "--popover": n(0.21, 0.006),
    "--popover-foreground": n(0.96, 0.002),
    "--secondary": n(0.26, 0.006),
    "--secondary-foreground": n(0.96, 0.002),
    "--muted": n(0.25, 0.006),
    "--muted-foreground": n(highContrast ? 0.84 : 0.7, 0.012),
    "--border": highContrast ? "oklch(1 0 0 / 24%)" : "oklch(1 0 0 / 9%)",
    "--input": highContrast ? "oklch(1 0 0 / 30%)" : "oklch(1 0 0 / 13%)",
    "--field": n(0.235, 0.006),
    "--frame": n(0.18, 0.005),
    "--sidebar": n(0.172, 0.005),
    "--sidebar-foreground": n(highContrast ? 1 : 0.93, 0.003),
    "--sidebar-accent": n(0.24, 0.006),
    "--sidebar-accent-foreground": n(0.97, 0.002),
    "--sidebar-border": highContrast ? "oklch(1 0 0 / 22%)" : "oklch(1 0 0 / 8%)",
  };
}

function accentTokens(spec: AccentSpec, dark: boolean): Tokens {
  const { hue } = spec;
  const p = dark ? spec.dark : spec.light;
  const primary: Oklch = { l: p.l, c: p.c, h: hue };
  const fg = readableForeground(primary);
  const cf = Math.min(1, p.c / 0.2); // how colourful the accent is, 0..1
  const ring: Oklch = dark ? { l: 0.6, c: 0.16 * cf, h: hue } : { l: 0.68, c: 0.16 * cf, h: hue };
  const charts = [
    { l: 0.72, c: 0.15 * cf, h: hue },
    { l: dark ? 0.62 : 0.6, c: Math.max(0.12, p.c), h: hue },
    { l: dark ? 0.68 : 0.65, c: 0.14, h: (hue + 150) % 360 },
    { l: dark ? 0.72 : 0.7, c: 0.15, h: (hue + 220) % 360 },
    { l: dark ? 0.78 : 0.75, c: 0.15, h: (hue + 75) % 360 },
  ];
  return {
    "--primary": formatOklch(primary),
    "--primary-foreground": formatOklch(fg),
    "--ring": formatOklch(ring),
    "--accent": dark
      ? formatOklch({ l: 0.29, c: 0.04 * cf, h: hue })
      : formatOklch({ l: 0.95, c: 0.012 * cf, h: hue }),
    "--accent-foreground": dark
      ? formatOklch({ l: 0.92, c: 0.04 * cf, h: hue })
      : formatOklch({ l: 0.3, c: 0.08 * cf, h: hue }),
    "--sidebar-primary": formatOklch(primary),
    "--sidebar-primary-foreground": formatOklch(fg),
    "--sidebar-ring": formatOklch(ring),
    ...Object.fromEntries(charts.map((c, i) => [`--chart-${i + 1}`, formatOklch(c)])),
  };
}

/**
 * "Solid accent" sidebar: a deep accent surface. Tokens are re-declared on the
 * sidebar element so everything inside (muted text, borders, icon tiles)
 * follows without touching the components.
 */
function accentSidebarTokens(spec: AccentSpec): Tokens {
  const h = spec.hue;
  const c = Math.min(spec.light.c, 0.16);
  const surface: Oklch = { l: 0.3, c: c * 0.75, h };
  return {
    "--sidebar": formatOklch(surface),
    "--sidebar-foreground": formatOklch({ l: 0.97, c: 0.01, h }),
    "--sidebar-accent": formatOklch({ l: 0.38, c: c * 0.8, h }),
    "--sidebar-accent-foreground": formatOklch({ l: 0.99, c: 0.005, h }),
    "--sidebar-border": "oklch(1 0 0 / 12%)",
    "--foreground": formatOklch({ l: 0.97, c: 0.01, h }),
    "--muted-foreground": formatOklch({ l: 0.82, c: 0.04, h }),
    "--card": formatOklch({ l: 0.35, c: c * 0.8, h }),
    "--field": formatOklch({ l: 0.26, c: c * 0.7, h }),
    "--border": "oklch(1 0 0 / 14%)",
    "--primary": formatOklch({ l: 0.85, c: 0.1, h }),
    "--muted": formatOklch({ l: 0.36, c: c * 0.8, h }),
  };
}

const block = (selector: string, tokens: Tokens) =>
  `${selector}{${Object.entries(tokens)
    .map(([k, v]) => `${k}:${v}`)
    .join(";")}}`;

export interface ThemeOutput {
  /** Stylesheet text for <style id="rivetr-theme-vars">. */
  css: string;
  /** Data attributes for <html>. */
  attrs: Record<string, string>;
}

export const THEME_STYLE_ID = "rivetr-theme-vars";

/**
 * Selectors use `html:root` / `html.dark` so they beat the defaults in
 * app.css regardless of stylesheet order.
 */
/** All colour tokens for one mode, e.g. for a scoped preview via inline style. */
export function themeTokens(settings: ThemeSettings, dark: boolean): Tokens {
  const spec = accentSpec(settings);
  const tokens = { ...neutralTokens(settings.neutral, dark, settings.highContrast), ...accentTokens(spec, dark) };
  if (settings.sidebar === "accent") {
    const side = accentSidebarTokens(spec);
    for (const key of Object.keys(side)) if (key.startsWith("--sidebar")) tokens[key] = side[key];
  }
  return tokens;
}

export function buildThemeOutput(settings: ThemeSettings): ThemeOutput {
  const spec = accentSpec(settings);
  const light = { ...neutralTokens(settings.neutral, false, settings.highContrast), ...accentTokens(spec, false) };
  const dark = { ...neutralTokens(settings.neutral, true, settings.highContrast), ...accentTokens(spec, true) };
  let css = block("html:root", light) + block("html.dark", dark);
  if (settings.sidebar === "accent") {
    css += block('html [data-sidebar="sidebar"]', accentSidebarTokens(spec));
  }
  return {
    css,
    attrs: {
      "data-accent": settings.accent,
      "data-neutral": settings.neutral,
      "data-radius": settings.radius,
      "data-density": settings.density,
      "data-font": settings.font,
      "data-font-size": settings.fontSize,
      "data-sidebar-style": settings.sidebar,
      "data-motion": settings.reducedMotion ? "reduce" : "full",
      "data-contrast": settings.highContrast ? "high" : "normal",
    },
  };
}

/** Swatch colour for a preset or custom accent, as rendered in light mode. */
export function accentSwatch(settings: Pick<ThemeSettings, "accent" | "customAccent">): string {
  const spec = accentSpec(settings);
  return formatOklch({ l: spec.light.l, c: spec.light.c, h: spec.hue });
}

export function neutralSwatch(neutral: Neutral, l = 0.55): string {
  const { hue, k } = NEUTRAL_SPECS[neutral];
  return formatOklch({ l, c: 0.014 * k, h: hue });
}
