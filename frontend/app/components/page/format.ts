/** "just now", "5m ago", "3h ago", "2d ago", then a short date. */
export function relativeTime(input: string | number | Date | null | undefined): string {
  if (input === null || input === undefined) return "-";
  const date = input instanceof Date ? input : new Date(input);
  const t = date.getTime();
  if (Number.isNaN(t)) return "-";
  const secs = Math.round((Date.now() - t) / 1000);
  if (secs < 0) return date.toLocaleString();
  if (secs < 10) return "just now";
  if (secs < 60) return `${secs}s ago`;
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d ago`;
  return date.toLocaleDateString();
}

/** Seconds (or a start/end pair) as "45s", "3m 12s", "1h 4m". */
export function formatDuration(
  startOrSeconds: string | number | null | undefined,
  end?: string | null
): string {
  let secs: number;
  if (typeof startOrSeconds === "number") {
    secs = startOrSeconds;
  } else {
    if (!startOrSeconds || !end) return "-";
    secs = (new Date(end).getTime() - new Date(startOrSeconds).getTime()) / 1000;
  }
  if (!Number.isFinite(secs) || secs < 0) return "-";
  secs = Math.round(secs);
  if (secs < 60) return `${secs}s`;
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m ${secs % 60}s`;
  const hours = Math.floor(mins / 60);
  return `${hours}h ${mins % 60}m`;
}
