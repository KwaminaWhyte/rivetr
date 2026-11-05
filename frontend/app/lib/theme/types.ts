import { isHexColor, normalizeHex } from "./color";

export const THEME_MODES = ["light", "dark", "system"] as const;
export const ACCENT_PRESETS = [
  "violet",
  "blue",
  "indigo",
  "emerald",
  "teal",
  "amber",
  "orange",
  "rose",
  "slate",
] as const;
export const ACCENTS = [...ACCENT_PRESETS, "custom"] as const;
export const NEUTRALS = ["zinc", "slate", "stone", "neutral", "gray"] as const;
export const RADII = ["none", "small", "medium", "large", "full"] as const;
export const DENSITIES = ["compact", "comfortable", "spacious"] as const;
export const FONTS = ["geist", "inter", "system", "plex"] as const;
export const FONT_SIZES = ["small", "default", "large"] as const;
export const SIDEBAR_STYLES = ["default", "inset", "floating", "accent"] as const;

export type ThemeMode = (typeof THEME_MODES)[number];
export type AccentPreset = (typeof ACCENT_PRESETS)[number];
export type Accent = (typeof ACCENTS)[number];
export type Neutral = (typeof NEUTRALS)[number];
export type Radius = (typeof RADII)[number];
export type Density = (typeof DENSITIES)[number];
export type FontFamily = (typeof FONTS)[number];
export type FontSize = (typeof FONT_SIZES)[number];
export type SidebarStyle = (typeof SIDEBAR_STYLES)[number];

/** Everything a user can customise about the dashboard look. Stored as JSON per user. */
export interface ThemeSettings {
  version: 1;
  mode: ThemeMode;
  accent: Accent;
  /** Hex colour used when `accent` is "custom". */
  customAccent: string;
  neutral: Neutral;
  radius: Radius;
  density: Density;
  font: FontFamily;
  fontSize: FontSize;
  sidebar: SidebarStyle;
  reducedMotion: boolean;
  highContrast: boolean;
}

export const DEFAULT_THEME: ThemeSettings = {
  version: 1,
  mode: "system",
  accent: "violet",
  customAccent: "#7c3aed",
  neutral: "zinc",
  radius: "medium",
  density: "comfortable",
  font: "geist",
  fontSize: "default",
  sidebar: "default",
  reducedMotion: false,
  highContrast: false,
};

function pick<T extends string>(value: unknown, allowed: readonly T[], fallback: T): T {
  return typeof value === "string" && (allowed as readonly string[]).includes(value)
    ? (value as T)
    : fallback;
}

/**
 * Turn anything (server JSON, an imported file, an old cache) into a valid
 * theme. Unknown or invalid fields fall back to `base` one by one, so a theme
 * saved by a newer dashboard still loads.
 */
export function normalizeTheme(input: unknown, base: ThemeSettings = DEFAULT_THEME): ThemeSettings {
  const raw = input && typeof input === "object" ? (input as Record<string, unknown>) : {};
  return {
    version: 1,
    mode: pick(raw.mode, THEME_MODES, base.mode),
    accent: pick(raw.accent, ACCENTS, base.accent),
    customAccent: isHexColor(raw.customAccent) ? normalizeHex(raw.customAccent) : base.customAccent,
    neutral: pick(raw.neutral, NEUTRALS, base.neutral),
    radius: pick(raw.radius, RADII, base.radius),
    density: pick(raw.density, DENSITIES, base.density),
    font: pick(raw.font, FONTS, base.font),
    fontSize: pick(raw.fontSize, FONT_SIZES, base.fontSize),
    sidebar: pick(raw.sidebar, SIDEBAR_STYLES, base.sidebar),
    reducedMotion: typeof raw.reducedMotion === "boolean" ? raw.reducedMotion : base.reducedMotion,
    highContrast: typeof raw.highContrast === "boolean" ? raw.highContrast : base.highContrast,
  };
}

export function themesEqual(a: ThemeSettings, b: ThemeSettings): boolean {
  return (Object.keys(a) as (keyof ThemeSettings)[]).every((k) => a[k] === b[k]);
}

export const LABELS = {
  mode: { light: "Light", dark: "Dark", system: "System" },
  accent: {
    violet: "Violet",
    blue: "Blue",
    indigo: "Indigo",
    emerald: "Emerald",
    teal: "Teal",
    amber: "Amber",
    orange: "Orange",
    rose: "Rose",
    slate: "Slate",
    custom: "Custom",
  },
  neutral: { zinc: "Zinc", slate: "Slate", stone: "Stone", neutral: "Neutral", gray: "Gray" },
  radius: { none: "None", small: "Small", medium: "Medium", large: "Large", full: "Full" },
  density: { compact: "Compact", comfortable: "Comfortable", spacious: "Spacious" },
  font: { geist: "Geist", inter: "Inter", system: "System UI", plex: "IBM Plex Sans" },
  fontSize: { small: "Small", default: "Default", large: "Large" },
  sidebar: { default: "Default", inset: "Inset", floating: "Floating", accent: "Solid accent" },
} as const;
