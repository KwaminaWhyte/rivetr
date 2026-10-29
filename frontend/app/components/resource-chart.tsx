import { useMemo, useState } from "react";
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useQuery } from "@tanstack/react-query";

interface DataPoint {
  time: string;
  cpu: number;
  memory: number;
}

interface StatsHistoryPoint {
  timestamp: string;
  cpu_percent: number;
  memory_used_bytes: number;
  memory_total_bytes: number;
}

interface ResourceChartProps {
  cpuPercent: number;
  memoryPercent: number;
  /** Chart plot height in px. */
  height?: number;
  /** Extra classes for the card, e.g. `h-full` when it sits in a stretched grid cell. */
  className?: string;
}

const CPU_COLOR = "#3b82f6";
const MEMORY_COLOR = "#a855f7";

export const TIME_RANGES = [
  { value: "1", label: "Last 1 hour" },
  { value: "6", label: "Last 6 hours" },
  { value: "24", label: "Last 24 hours" },
  { value: "168", label: "Last 7 days" },
  { value: "720", label: "Last 30 days" },
] as const;

async function fetchStatsHistory(hours: number): Promise<DataPoint[]> {
  try {
    const token = localStorage.getItem("rivetr_auth_token");
    const headers: HeadersInit = { "Content-Type": "application/json" };
    if (token) headers["Authorization"] = `Bearer ${token}`;

    const response = await fetch(`/api/system/stats/history?hours=${hours}`, {
      headers,
    });
    if (!response.ok) throw new Error("Failed to fetch stats history");
    const data = await response.json();

    if (!data.history || data.history.length === 0) return [];

    return data.history.map((point: StatsHistoryPoint) => {
      const date = new Date(point.timestamp);
      const timeStr =
        hours <= 24
          ? date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
          : date.toLocaleDateString([], { month: "short", day: "numeric" });

      const memoryPercent =
        point.memory_total_bytes > 0
          ? (point.memory_used_bytes / point.memory_total_bytes) * 100
          : 0;

      return {
        time: timeStr,
        cpu: parseFloat(point.cpu_percent.toFixed(1)),
        memory: parseFloat(memoryPercent.toFixed(1)),
      };
    });
  } catch (error) {
    console.error("Error fetching stats history:", error);
    return [];
  }
}

function CustomTooltip({
  active,
  payload,
  label,
}: {
  active?: boolean;
  payload?: Array<{ dataKey: string; name: string; value: number; color: string }>;
  label?: string;
}) {
  if (!active || !payload?.length) return null;
  return (
    <div className="rounded-lg border border-border bg-popover px-3 py-2 text-xs shadow-lg">
      <p className="mb-1 font-medium text-muted-foreground">{label}</p>
      {payload.map((entry) => (
        <div key={entry.dataKey} className="flex items-center gap-2">
          <span
            className="inline-block h-2 w-2 rounded-full"
            style={{ background: entry.color }}
          />
          <span className="font-medium text-foreground">{entry.name}</span>
          <span className="ml-auto pl-4 tabular-nums text-foreground">
            {entry.value.toFixed(1)}%
          </span>
        </div>
      ))}
    </div>
  );
}

function LegendChip({ color, label, value }: { color: string; label: string; value: number }) {
  return (
    <span className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
      <span
        className="inline-block h-2 w-2 rounded-full"
        style={{ background: color }}
        aria-hidden="true"
      />
      {label}
      <span className="font-medium tabular-nums text-foreground">{value.toFixed(1)}%</span>
    </span>
  );
}

export function ResourceChart({
  cpuPercent,
  memoryPercent,
  height = 240,
  className,
}: ResourceChartProps) {
  const [timeRange, setTimeRange] = useState("24");

  const { data: historyData } = useQuery({
    queryKey: ["statsHistory", timeRange],
    queryFn: () => fetchStatsHistory(parseInt(timeRange)),
    refetchInterval: 60000,
    staleTime: 30000,
  });

  const data = useMemo(() => {
    if (historyData && historyData.length > 0) return historyData;
    return [{ time: "Now", cpu: cpuPercent, memory: memoryPercent }];
  }, [historyData, cpuPercent, memoryPercent]);

  return (
    <Card className={className}>
      <CardHeader className="pb-0">
        <div className="flex flex-wrap items-center justify-between gap-x-6 gap-y-2">
          <div className="flex flex-wrap items-center gap-x-5 gap-y-1">
            <CardTitle>Resource utilization</CardTitle>
            <div className="flex items-center gap-4">
              <LegendChip color={CPU_COLOR} label="CPU" value={cpuPercent} />
              <LegendChip color={MEMORY_COLOR} label="Memory" value={memoryPercent} />
            </div>
          </div>
          <Select value={timeRange} onValueChange={setTimeRange}>
            <SelectTrigger className="h-8 w-[140px] text-xs" aria-label="Time range">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {TIME_RANGES.map((range) => (
                <SelectItem key={range.value} value={range.value}>
                  {range.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </CardHeader>
      <CardContent className="flex-1">
        {/* min sizes stop Recharts logging width(-1) before first layout */}
        <div className="w-full min-w-[200px]" style={{ height, minHeight: height }}>
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart data={data} margin={{ top: 8, right: 4, left: 0, bottom: 0 }}>
              <defs>
                <linearGradient id="gradCpu" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor={CPU_COLOR} stopOpacity={0.28} />
                  <stop offset="95%" stopColor={CPU_COLOR} stopOpacity={0.02} />
                </linearGradient>
                <linearGradient id="gradMemory" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor={MEMORY_COLOR} stopOpacity={0.22} />
                  <stop offset="95%" stopColor={MEMORY_COLOR} stopOpacity={0.02} />
                </linearGradient>
              </defs>
              <CartesianGrid
                strokeDasharray="3 3"
                stroke="var(--border)"
                vertical={false}
              />
              <XAxis
                dataKey="time"
                tick={{ fontSize: 11, fill: "var(--muted-foreground)" }}
                tickLine={false}
                axisLine={false}
                interval="preserveStartEnd"
                minTickGap={48}
                tickMargin={8}
              />
              <YAxis
                width={40}
                tick={{ fontSize: 11, fill: "var(--muted-foreground)" }}
                tickLine={false}
                axisLine={false}
                tickFormatter={(v) => `${v}%`}
                domain={[0, 100]}
                ticks={[0, 25, 50, 75, 100]}
              />
              <Tooltip
                content={<CustomTooltip />}
                cursor={{ stroke: "var(--border)", strokeWidth: 1 }}
              />
              <Area
                type="monotone"
                dataKey="memory"
                name="Memory"
                stroke={MEMORY_COLOR}
                strokeWidth={1.75}
                fill="url(#gradMemory)"
                dot={false}
                isAnimationActive={false}
                activeDot={{ r: 3.5, strokeWidth: 0, fill: MEMORY_COLOR }}
              />
              <Area
                type="monotone"
                dataKey="cpu"
                name="CPU"
                stroke={CPU_COLOR}
                strokeWidth={1.75}
                fill="url(#gradCpu)"
                dot={false}
                isAnimationActive={false}
                activeDot={{ r: 3.5, strokeWidth: 0, fill: CPU_COLOR }}
              />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      </CardContent>
    </Card>
  );
}
