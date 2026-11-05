import { StatusDot, StatusPill as TonePill, type Tone } from "@/components/metrics";
import { cn } from "@/lib/utils";

export type { Tone };
export { StatusDot };

const OK = new Set(["running", "success", "succeeded", "healthy", "online", "active", "completed", "reachable"]);
const WARN = new Set([
  "pending",
  "cloning",
  "building",
  "starting",
  "checking",
  "pulling",
  "deploying",
  "restarting",
  "queued",
  "scheduled",
  "unknown",
  "skipped",
  "frozen",
]);
const BAD = new Set(["failed", "error", "unhealthy", "offline", "unreachable", "crashed", "exited", "dead"]);
const ACTIVE = new Set(["pending", "cloning", "building", "starting", "checking", "pulling", "deploying", "restarting", "queued"]);

/** Map any app, database, service, deployment or server status onto a tone. */
export function statusTone(status?: string | null): Tone {
  const s = (status ?? "").toLowerCase();
  if (OK.has(s)) return "ok";
  if (BAD.has(s)) return "bad";
  if (s === "stopped" || s === "replaced" || s === "cancelled" || s === "not_deployed") return "neutral";
  if (WARN.has(s)) return "warn";
  return "neutral";
}

/** True for statuses that mean "work in progress" (pulsing dot). */
export function isActiveStatus(status?: string | null): boolean {
  return ACTIVE.has((status ?? "").toLowerCase());
}

/** "not_deployed" -> "Not deployed" */
export function statusLabel(status?: string | null): string {
  if (!status) return "Unknown";
  const s = status.replace(/_/g, " ");
  return s.charAt(0).toUpperCase() + s.slice(1);
}

const PILL: Record<Tone, string> = {
  ok: "bg-emerald-500/10 text-emerald-700 ring-emerald-600/15 dark:text-emerald-400",
  warn: "bg-amber-500/10 text-amber-700 ring-amber-600/15 dark:text-amber-400",
  bad: "bg-red-500/10 text-red-700 ring-red-600/15 dark:text-red-400",
  neutral: "bg-muted text-muted-foreground ring-foreground/10",
};
const DOT: Record<Tone, string> = {
  ok: "bg-emerald-500",
  warn: "bg-amber-500",
  bad: "bg-red-500",
  neutral: "bg-muted-foreground/60",
};

/**
 * Status chip with a coloured dot ("● Running"). Accepts a raw status string
 * and picks tone and label for it; pass `label` to override the text.
 * In-progress statuses get a pulsing dot.
 */
export function ResourceStatus({
  status,
  label,
  tone: toneOverride,
  size = "md",
  className,
}: {
  status?: string | null;
  label?: string;
  tone?: Tone;
  size?: "sm" | "md";
  className?: string;
}) {
  const tone = toneOverride ?? statusTone(status);
  const active = isActiveStatus(status);
  return (
    <span
      className={cn(
        "inline-flex shrink-0 items-center gap-1.5 rounded-full font-medium ring-1 ring-inset",
        size === "md" ? "px-2.5 py-0.5 text-xs" : "px-2 py-px text-[11px]",
        PILL[tone],
        className
      )}
    >
      <span className="relative flex size-1.5" aria-hidden="true">
        {active && (
          <span className={cn("absolute inline-flex size-full animate-ping rounded-full opacity-75", DOT[tone])} />
        )}
        <span className={cn("relative inline-flex size-1.5 rounded-full", DOT[tone])} />
      </span>
      {label ?? statusLabel(status)}
    </span>
  );
}

export { TonePill as StatusPill };
