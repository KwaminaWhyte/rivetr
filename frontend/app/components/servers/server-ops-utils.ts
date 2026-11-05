/** Formatting helpers for the server operations pages. */

/** Decimal byte sizes, matching what docker prints (1 GB = 1000^3 bytes). */
export function formatBytes(bytes: number): string {
  if (!bytes || bytes < 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let v = bytes;
  let i = 0;
  while (v >= 1000 && i < units.length - 1) {
    v /= 1000;
    i++;
  }
  return i === 0 ? `${bytes} B` : `${v.toFixed(v >= 100 ? 0 : v >= 10 ? 1 : 2)} ${units[i]}`;
}

/** "3 minutes ago", "in 2 hours", "just now". */
export function relativeTime(iso: string | null | undefined): string {
  if (!iso) return "never";
  const t = new Date(iso).getTime();
  if (Number.isNaN(t)) return iso;
  const diff = (Date.now() - t) / 1000;
  const abs = Math.abs(diff);
  const steps: [number, string][] = [
    [60, "second"],
    [60, "minute"],
    [24, "hour"],
    [30, "day"],
    [12, "month"],
    [Number.POSITIVE_INFINITY, "year"],
  ];
  let value = abs;
  let unit = "second";
  for (const [size, name] of steps) {
    unit = name;
    if (value < size) break;
    value /= size;
  }
  const n = Math.floor(value);
  if (unit === "second" && n < 10) return "just now";
  const label = `${n} ${unit}${n === 1 ? "" : "s"}`;
  return diff >= 0 ? `${label} ago` : `in ${label}`;
}

/** "850 ms", "12.4 s", "3m 05s". */
export function formatDuration(ms: number | null | undefined): string {
  if (ms == null) return "-";
  if (ms < 1000) return `${ms} ms`;
  const s = ms / 1000;
  if (s < 60) return `${s.toFixed(1)} s`;
  const m = Math.floor(s / 60);
  const rest = Math.round(s % 60);
  return `${m}m ${String(rest).padStart(2, "0")}s`;
}
