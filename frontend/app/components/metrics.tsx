import type { ReactNode } from "react";
import { Link } from "react-router";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";

export type Tone = "ok" | "warn" | "bad" | "neutral";

/** Map a utilisation percentage to a severity tone (75% warns, 90% is critical). */
export function toneForPercent(percent: number): Tone {
  if (percent >= 90) return "bad";
  if (percent >= 75) return "warn";
  return "ok";
}

const TONE_BAR: Record<Tone, string> = {
  ok: "bg-emerald-500",
  warn: "bg-amber-500",
  bad: "bg-red-500",
  neutral: "bg-primary",
};

const TONE_TEXT: Record<Tone, string> = {
  ok: "text-emerald-600 dark:text-emerald-400",
  warn: "text-amber-600 dark:text-amber-400",
  bad: "text-red-600 dark:text-red-400",
  neutral: "text-muted-foreground",
};

const TONE_PILL: Record<Tone, string> = {
  ok: "bg-emerald-500/10 text-emerald-700 dark:text-emerald-400",
  warn: "bg-amber-500/10 text-amber-700 dark:text-amber-400",
  bad: "bg-red-500/10 text-red-700 dark:text-red-400",
  neutral: "bg-muted text-muted-foreground",
};

const TONE_DOT: Record<Tone, string> = {
  ok: "bg-emerald-500",
  warn: "bg-amber-500",
  bad: "bg-red-500",
  neutral: "bg-muted-foreground/50",
};

export function toneTextClass(tone: Tone): string {
  return TONE_TEXT[tone];
}

/** Thin horizontal utilisation bar. Exposes its value to assistive tech. */
export function Meter({
  value,
  tone,
  label,
  className,
}: {
  value: number;
  tone?: Tone;
  label: string;
  className?: string;
}) {
  const clamped = Math.max(0, Math.min(100, Number.isFinite(value) ? value : 0));
  return (
    <div
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(clamped)}
      className={cn("h-1.5 w-full overflow-hidden rounded-full bg-muted", className)}
    >
      <div
        className={cn(
          "h-full rounded-full transition-[width] duration-500",
          TONE_BAR[tone ?? toneForPercent(clamped)]
        )}
        style={{ width: `${clamped}%` }}
      />
    </div>
  );
}

/**
 * Status chip. Always carries text, so state is never conveyed by colour alone.
 */
export function StatusPill({
  tone,
  children,
  className,
}: {
  tone: Tone;
  children: ReactNode;
  className?: string;
}) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium",
        TONE_PILL[tone],
        className
      )}
    >
      <span className={cn("h-1.5 w-1.5 rounded-full", TONE_DOT[tone])} aria-hidden="true" />
      {children}
    </span>
  );
}

/** Small status dot for dense lists. The label is read by screen readers and shown on hover. */
export function StatusDot({
  tone,
  label,
  className,
}: {
  tone: Tone;
  label: string;
  className?: string;
}) {
  return (
    <span
      title={label}
      className={cn("inline-block h-2 w-2 shrink-0 rounded-full", TONE_DOT[tone], className)}
    >
      <span className="sr-only">{label}</span>
    </span>
  );
}

export interface MetricTileProps {
  label: string;
  icon?: ReactNode;
  /** Primary value. Pass `undefined` with `loading` to show a skeleton. */
  value?: ReactNode;
  /** Small unit or qualifier rendered after the value. */
  unit?: string;
  /** One line of supporting text under the value. */
  detail?: ReactNode;
  /** 0-100. Renders a meter under the value when provided. */
  percent?: number;
  tone?: Tone;
  loading?: boolean;
  /** Makes the whole tile a link. */
  to?: string;
  className?: string;
}

function MetricTileBody({
  label,
  icon,
  value,
  unit,
  detail,
  percent,
  tone,
  loading,
}: MetricTileProps) {
  return (
    <div className="flex h-full flex-col gap-2 p-4">
      <div className="flex items-center justify-between gap-2 text-muted-foreground">
        <span className="truncate text-xs font-medium uppercase tracking-wide">{label}</span>
        {icon && <span className="shrink-0 [&_svg]:h-4 [&_svg]:w-4">{icon}</span>}
      </div>
      {loading ? (
        <>
          <Skeleton className="h-8 w-24" />
          <Skeleton className="h-3 w-32" />
        </>
      ) : (
        <>
          <div className="flex items-baseline gap-1.5">
            <span className="truncate text-2xl font-semibold leading-none tabular-nums tracking-tight">
              {value ?? "--"}
            </span>
            {unit && <span className="text-sm text-muted-foreground">{unit}</span>}
          </div>
          {percent !== undefined && <Meter value={percent} tone={tone} label={label} />}
          {detail && (
            <p className="truncate text-xs text-muted-foreground tabular-nums">{detail}</p>
          )}
        </>
      )}
    </div>
  );
}

export function MetricTile(props: MetricTileProps) {
  const { to, className } = props;
  const body = <MetricTileBody {...props} />;
  if (to) {
    return (
      <Link
        to={to}
        className={cn(
          "block bg-card outline-none transition-colors hover:bg-muted/40 focus-visible:ring-2 focus-visible:ring-ring",
          className
        )}
      >
        {body}
      </Link>
    );
  }
  return <div className={cn("bg-card", className)}>{body}</div>;
}

/** A label/value pair for a `<dl>` grid. Long values truncate rather than wrap. */
export function DetailField({
  label,
  mono,
  children,
  className,
}: {
  label: string;
  mono?: boolean;
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("min-w-0", className)}>
      <dt className="text-xs font-medium uppercase tracking-wide text-muted-foreground">{label}</dt>
      <dd className={cn("mt-1 truncate text-sm font-medium", mono && "font-mono")}>{children}</dd>
    </div>
  );
}

/**
 * A row of metric tiles drawn as one bordered surface with hairline dividers
 * (the gap shows the border colour), so uneven wraps never leave a hole.
 */
export function MetricStrip({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "grid gap-px overflow-hidden rounded-xl bg-border ring-1 ring-foreground/10",
        className
      )}
    >
      {children}
    </div>
  );
}
