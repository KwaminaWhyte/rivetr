import { cn } from "@/lib/utils";
import type { ServerSummary } from "@/lib/api/server-ops";

const STYLES: Record<ServerSummary["status"], { label: string; dot: string; pill: string }> = {
  reachable: {
    label: "Reachable",
    dot: "bg-green-500",
    pill: "border-green-500/30 text-green-700 dark:text-green-400",
  },
  attention: {
    label: "Attention required",
    dot: "bg-amber-500",
    pill: "border-amber-500/30 text-amber-700 dark:text-amber-400",
  },
  unreachable: {
    label: "Unreachable",
    dot: "bg-red-500",
    pill: "border-red-500/30 text-red-700 dark:text-red-400",
  },
  unknown: {
    label: "Unknown",
    dot: "bg-muted-foreground",
    pill: "border-border text-muted-foreground",
  },
};

/** Status pill used in the servers list and the server header. */
export function ServerStatusPill({
  status,
  className,
}: {
  status: ServerSummary["status"];
  className?: string;
}) {
  const s = STYLES[status] ?? STYLES.unknown;
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-xs font-medium",
        s.pill,
        className,
      )}
    >
      <span className={cn("h-2 w-2 rounded-full", s.dot)} />
      {s.label}
    </span>
  );
}
