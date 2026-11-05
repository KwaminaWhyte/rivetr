/**
 * Small OKLCH colour helpers: hex to OKLCH, OKLCH to sRGB and WCAG contrast.
 * Enough to derive an accent ramp from a custom hex and pick a readable
 * foreground, without pulling in a colour library.
 */

export interface Oklch {
  l: number;
  c: number;
  h: number;
}

const HEX_RE = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i;

export function isHexColor(value: unknown): value is string {
  return typeof value === "string" && HEX_RE.test(value.trim());
}

export function normalizeHex(value: string): string {
  let hex = value.trim().replace(/^#/, "").toLowerCase();
  if (hex.length === 3) hex = hex.split("").map((ch) => ch + ch).join("");
  return `#${hex}`;
}

function srgbToLinear(v: number): number {
  return v <= 0.04045 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
}

function linearToSrgb(v: number): number {
  return v <= 0.0031308 ? 12.92 * v : 1.055 * Math.pow(v, 1 / 2.4) - 0.055;
}

export function hexToOklch(hex: string): Oklch {
  const n = normalizeHex(hex);
  const r = srgbToLinear(parseInt(n.slice(1, 3), 16) / 255);
  const g = srgbToLinear(parseInt(n.slice(3, 5), 16) / 255);
  const b = srgbToLinear(parseInt(n.slice(5, 7), 16) / 255);

  const l_ = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m_ = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s_ = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);

  const L = 0.2104542553 * l_ + 0.793617785 * m_ - 0.0040720468 * s_;
  const A = 1.9779984951 * l_ - 2.428592205 * m_ + 0.4505937099 * s_;
  const B = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.808675766 * s_;

  const c = Math.sqrt(A * A + B * B);
  let h = (Math.atan2(B, A) * 180) / Math.PI;
  if (h < 0) h += 360;
  return { l: L, c, h };
}

/** OKLCH to linear-light sRGB, unclamped (values outside 0..1 are out of gamut). */
function oklchToLinearRgb({ l, c, h }: Oklch): [number, number, number] {
  const hr = (h * Math.PI) / 180;
  const A = c * Math.cos(hr);
  const B = c * Math.sin(hr);
  const l_ = l + 0.3963377774 * A + 0.2158037573 * B;
  const m_ = l - 0.1055613458 * A - 0.0638541728 * B;
  const s_ = l - 0.0894841775 * A - 1.291485548 * B;
  const L = l_ * l_ * l_;
  const M = m_ * m_ * m_;
  const S = s_ * s_ * s_;
  return [
    4.0767416621 * L - 3.3077115913 * M + 0.2309699292 * S,
    -1.2684380046 * L + 2.6097574011 * M - 0.3413193965 * S,
    -0.0041960863 * L - 0.7034186147 * M + 1.707614701 * S,
  ];
}

function inGamut(color: Oklch): boolean {
  return oklchToLinearRgb(color).every((v) => v >= -0.0005 && v <= 1.0005);
}

/** Reduce chroma until the colour fits in sRGB (keeps lightness and hue). */
export function clampToGamut(color: Oklch): Oklch {
  if (inGamut(color)) return color;
  let lo = 0;
  let hi = color.c;
  for (let i = 0; i < 20; i++) {
    const mid = (lo + hi) / 2;
    if (inGamut({ ...color, c: mid })) lo = mid;
    else hi = mid;
  }
  return { ...color, c: lo };
}

export function oklchToHex(color: Oklch): string {
  const rgb = oklchToLinearRgb(clampToGamut(color)).map((v) =>
    Math.round(Math.min(1, Math.max(0, linearToSrgb(Math.min(1, Math.max(0, v))))) * 255)
  );
  return `#${rgb.map((v) => v.toString(16).padStart(2, "0")).join("")}`;
}

/** WCAG relative luminance of an OKLCH colour. */
export function luminance(color: Oklch): number {
  const [r, g, b] = oklchToLinearRgb(clampToGamut(color)).map((v) => Math.min(1, Math.max(0, v)));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

export function contrastRatio(a: Oklch, b: Oklch): number {
  const la = luminance(a);
  const lb = luminance(b);
  const [hi, lo] = la > lb ? [la, lb] : [lb, la];
  return (hi + 0.05) / (lo + 0.05);
}

const round = (v: number, digits: number) => {
  const f = 10 ** digits;
  return Math.round(v * f) / f;
};

export function formatOklch(color: Oklch, alpha?: number): string {
  const c = clampToGamut(color);
  const body = `${round(c.l, 3)} ${round(c.c, 3)} ${round(c.h, 1)}`;
  return alpha === undefined ? `oklch(${body})` : `oklch(${body} / ${alpha}%)`;
}
