import type { ReactNode } from "react";
import { Link } from "react-router";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { BarChart3 } from "lucide-react";
import { EmptyState, SectionCard, SectionSkeleton } from "@/components/page";
import { QueryError } from "@/components/query-error";
import { cn } from "@/lib/utils";
import type { AnalyticsRange, AnalyticsReport, Ranked } from "@/lib/api/analytics";

export const RANGES: { value: AnalyticsRange; label: string }[] = [
  { value: "1h", label: "1h" },
  { value: "24h", label: "24h" },
  { value: "7d", label: "7d" },
  { value: "30d", label: "30d" },
];

const STATUS_SERIES = [
  { key: "status_2xx", label: "2xx", color: "var(--chart-4)" },
  { key: "status_3xx", label: "3xx", color: "var(--chart-3)" },
  { key: "status_4xx", label: "4xx", color: "var(--chart-5)" },
  { key: "status_5xx", label: "5xx", color: "var(--destructive)" },
] as const;

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = n / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v < 10 ? 1 : 0)} ${units[i]}`;
}

export function formatCount(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 10_000) return `${(n / 1000).toFixed(0)}k`;
  if (n >= 1_000) return `${(n / 1000).toFixed(1)}k`;
  return String(n);
}

function formatMs(v: number | null): string {
  if (v === null) return "-";
  return v >= 1000 ? `${(v / 1000).toFixed(2)} s` : `${Math.round(v)} ms`;
}

function tickLabel(ts: number, range: AnalyticsRange): string {
  const d = new Date(ts * 1000);
  if (range === "1h" || range === "24h") {
    return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }
  return d.toLocaleDateString([], { month: "short", day: "numeric" });
}

/** Segmented 1h / 24h / 7d / 30d picker. */
export function RangePicker({
  value,
  onChange,
}: {
  value: AnalyticsRange;
  onChange: (v: AnalyticsRange) => void;
}) {
  return (
    <div role="radiogroup" aria-label="Time range" className="inline-flex rounded-lg border bg-card p-0.5 shadow-xs">
      {RANGES.map((r) => (
        <button
          key={r.value}
          type="button"
          role="radio"
          aria-checked={value === r.value}
          onClick={() => onChange(r.value)}
          className={cn(
            "rounded-md px-2.5 py-1 text-xs font-medium tabular-nums transition-colors",
            value === r.value
              ? "bg-muted text-foreground"
              : "text-muted-foreground hover:text-foreground"
          )}
        >
          {r.label}
        </button>
      ))}
    </div>
  );
}

function Stat({ label, value, hint }: { label: string; value: string; hint?: string }) {
  return (
    <div className="rounded-lg border bg-card px-4 py-3 shadow-xs">
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="mt-1 text-2xl font-semibold tabular-nums tracking-tight">{value}</p>
      {hint && <p className="mt-0.5 text-xs text-muted-foreground">{hint}</p>}
    </div>
  );
}

function ChartTooltip({
  active,
  payload,
  label,
  range,
  unit,
}: {
  active?: boolean;
  payload?: Array<{ dataKey: string; name: string; value: number | null; color: string }>;
  label?: number;
  range: AnalyticsRange;
  unit: "count" | "ms";
}) {
  if (!active || !payload?.length || label === undefined) return null;
  return (
    <div className="rounded-lg border bg-popover px-3 py-2 text-xs shadow-lg">
      <p className="mb-1 font-medium text-muted-foreground">{tickLabel(label, range)}</p>
      {payload.map((e) => (
        <div key={e.dataKey} className="flex items-center gap-2">
          <span className="inline-block size-2 rounded-full" style={{ background: e.color }} />
          <span className="text-foreground">{e.name}</span>
          <span className="ml-auto pl-4 tabular-nums text-foreground">
            {unit === "ms" ? formatMs(e.value) : (e.value ?? 0).toLocaleString()}
          </span>
        </div>
      ))}
    </div>
  );
}

function Legend({ items }: { items: { label: string; color: string }[] }) {
  return (
    <div className="flex flex-wrap items-center gap-3">
      {items.map((i) => (
        <span key={i.label} className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
          <span className="inline-block size-2 rounded-full" style={{ background: i.color }} aria-hidden="true" />
          {i.label}
        </span>
      ))}
    </div>
  );
}

function RankedList({
  title,
  rows,
  empty,
  render,
}: {
  title: string;
  rows: Ranked[];
  empty: string;
  render?: (value: string) => ReactNode;
}) {
  const max = Math.max(1, ...rows.map((r) => r.count));
  return (
    <SectionCard title={title} flush>
      {rows.length === 0 ? (
        <p className="px-4 py-6 text-center text-sm text-muted-foreground">{empty}</p>
      ) : (
        <ul className="space-y-1 p-2">
          {rows.map((r) => (
            <li key={r.value} className="relative overflow-hidden rounded-md px-2 py-1.5 text-sm">
              <span
                aria-hidden="true"
                className="absolute inset-y-0 left-0 rounded-md bg-[var(--chart-1)] opacity-15"
                style={{ width: `${(r.count / max) * 100}%` }}
              />
              <span className="relative flex items-center justify-between gap-3">
                <span className="min-w-0 truncate font-mono text-xs">{render ? render(r.value) : r.value}</span>
                <span className="shrink-0 tabular-nums text-muted-foreground">{formatCount(r.count)}</span>
              </span>
            </li>
          ))}
        </ul>
      )}
    </SectionCard>
  );
}

/** Country flag from an ISO code (regional indicator symbols). */
function flag(code: string): string {
  if (!/^[A-Z]{2}$/.test(code)) return "";
  return String.fromCodePoint(...[...code].map((c) => 0x1f1a5 + c.charCodeAt(0)));
}

/**
 * Full analytics view for one report: stats, requests and latency charts and
 * top-N lists. Shared by the app, server and team pages.
 */
export function AnalyticsReportView({
  report,
  isLoading,
  error,
  onRetry,
  range,
  disabledHint,
  showHosts = false,
}: {
  report: AnalyticsReport | undefined;
  isLoading: boolean;
  error: unknown;
  onRetry: () => void;
  range: AnalyticsRange;
  /** Shown when analytics is off: how to turn it on. */
  disabledHint: ReactNode;
  showHosts?: boolean;
}) {
  if (error && !report) return <QueryError what="traffic analytics" error={error} onRetry={onRetry} />;
  if (isLoading || !report) {
    return (
      <div className="space-y-6">
        <SectionSkeleton rows={2} />
        <SectionSkeleton rows={4} />
      </div>
    );
  }

  const t = report.totals;
  if (t.requests === 0) {
    return report.enabled ? (
      <EmptyState
        bordered
        icon={<BarChart3 />}
        title="No traffic in this range"
        description="Requests through the Rivetr proxy show up here within about a minute."
      />
    ) : (
      <EmptyState bordered icon={<BarChart3 />} title="Traffic analytics is off" description={disabledHint} />
    );
  }

  const errors = t.status_4xx + t.status_5xx;
  const errorRate = t.requests > 0 ? (errors / t.requests) * 100 : 0;
  const data = report.series;

  return (
    <div className="space-y-6">
      {!report.enabled && (
        <p className="rounded-lg border border-dashed px-4 py-3 text-sm text-muted-foreground">
          Analytics is turned off now, so new requests are not counted. {disabledHint}
        </p>
      )}
      <div className="grid grid-cols-2 gap-3 lg:grid-cols-5">
        <Stat label="Requests" value={formatCount(t.requests)} />
        <Stat label="Visitors" value={formatCount(t.visitors)} hint="Daily unique" />
        <Stat
          label="Error rate"
          value={`${errorRate.toFixed(errorRate < 10 ? 1 : 0)}%`}
          hint={`${formatCount(t.status_5xx)} server errors`}
        />
        <Stat label="Latency" value={formatMs(t.p50_ms)} hint={`p95 ${formatMs(t.p95_ms)}`} />
        <Stat label="Bandwidth" value={formatBytes(t.bytes_out)} hint={`${formatBytes(t.bytes_in)} in`} />
      </div>

      <SectionCard
        title="Requests"
        actions={<Legend items={STATUS_SERIES.map((s) => ({ label: s.label, color: s.color }))} />}
      >
        <div className="h-56 w-full min-w-[200px]">
          <ResponsiveContainer width="100%" height="100%">
            <BarChart data={data} margin={{ top: 4, right: 4, left: 0, bottom: 0 }}>
              <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" vertical={false} />
              <XAxis
                dataKey="ts"
                tickFormatter={(v: number) => tickLabel(v, range)}
                tick={{ fontSize: 11, fill: "var(--muted-foreground)" }}
                tickLine={false}
                axisLine={false}
                minTickGap={32}
              />
              <YAxis
                tickFormatter={(v: number) => formatCount(v)}
                tick={{ fontSize: 11, fill: "var(--muted-foreground)" }}
                tickLine={false}
                axisLine={false}
                width={40}
                allowDecimals={false}
              />
              <Tooltip
                cursor={{ fill: "var(--muted)", opacity: 0.5 }}
                content={<ChartTooltip range={range} unit="count" />}
              />
              {STATUS_SERIES.map((s) => (
                <Bar key={s.key} dataKey={s.key} name={s.label} stackId="status" fill={s.color} isAnimationActive={false} />
              ))}
            </BarChart>
          </ResponsiveContainer>
        </div>
      </SectionCard>

      <SectionCard
        title="Latency"
        actions={
          <Legend
            items={[
              { label: "p50", color: "var(--chart-1)" },
              { label: "p95", color: "var(--chart-2)" },
            ]}
          />
        }
      >
        <div className="h-48 w-full min-w-[200px]">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={data} margin={{ top: 4, right: 4, left: 0, bottom: 0 }}>
              <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" vertical={false} />
              <XAxis
                dataKey="ts"
                tickFormatter={(v: number) => tickLabel(v, range)}
                tick={{ fontSize: 11, fill: "var(--muted-foreground)" }}
                tickLine={false}
                axisLine={false}
                minTickGap={32}
              />
              <YAxis
                tickFormatter={(v: number) => formatMs(v)}
                tick={{ fontSize: 11, fill: "var(--muted-foreground)" }}
                tickLine={false}
                axisLine={false}
                width={56}
              />
              <Tooltip content={<ChartTooltip range={range} unit="ms" />} />
              <Line type="monotone" dataKey="p50_ms" name="p50" stroke="var(--chart-1)" strokeWidth={2} dot={false} connectNulls isAnimationActive={false} />
              <Line type="monotone" dataKey="p95_ms" name="p95" stroke="var(--chart-2)" strokeWidth={2} dot={false} connectNulls isAnimationActive={false} />
            </LineChart>
          </ResponsiveContainer>
        </div>
      </SectionCard>

      <div className="grid gap-6 lg:grid-cols-2">
        {showHosts && (
          <SectionCard title="Top domains" flush>
            <ul className="divide-y text-sm">
              {report.top_hosts.map((h) => (
                <li key={h.host} className="flex items-center justify-between gap-3 px-4 py-2">
                  <span className="min-w-0 truncate">
                    <span className="font-mono text-xs">{h.host}</span>
                    {h.app_id && h.app_name && (
                      <Link to={`/apps/${h.app_id}/analytics`} className="ml-2 text-xs text-muted-foreground hover:underline">
                        {h.app_name}
                      </Link>
                    )}
                  </span>
                  <span className="shrink-0 tabular-nums text-muted-foreground">{formatCount(h.requests)}</span>
                </li>
              ))}
            </ul>
          </SectionCard>
        )}
        <RankedList title="Top paths" rows={report.top_paths} empty="No paths yet" />
        <RankedList title="Top referrers" rows={report.top_referrers} empty="No referrers. Visits so far are direct." />
        <RankedList title="Browsers and clients" rows={report.top_user_agents} empty="No data yet" />
        {/* Hidden without a GeoIP database (Settings > Advanced > Traffic analytics). */}
        {report.countries && (
          <RankedList
            title="Countries"
            rows={report.countries}
            empty="No country data yet"
            render={(code) => `${flag(code)} ${code}`.trim()}
          />
        )}
      </div>
    </div>
  );
}
