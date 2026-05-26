import { useMemo, useState } from "react";
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
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
}

const TIME_RANGES = [
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
    <div className="bg-background border border-border rounded-lg px-3 py-2 shadow-lg text-xs">
      <p className="font-medium text-muted-foreground mb-1">{label}</p>
      {payload.map((entry) => (
        <div key={entry.dataKey} className="flex items-center gap-2">
          <span
            className="inline-block h-2 w-2 rounded-full"
            style={{ background: entry.color }}
          />
          <span className="text-foreground font-medium">{entry.name}:</span>
          <span className="text-foreground">{entry.value.toFixed(1)}%</span>
        </div>
      ))}
    </div>
  );
}

export function ResourceChart({ cpuPercent, memoryPercent }: ResourceChartProps) {
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
    <Card>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle>Global Resource Utilization</CardTitle>
          <Select value={timeRange} onValueChange={setTimeRange}>
            <SelectTrigger className="w-[140px] h-8 text-xs">
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
      <CardContent>
        {/* min-height/min-width prevents Recharts width(-1) warning before
            the container has laid out (B19) */}
        <div className="w-full min-w-[200px] min-h-[220px] h-[220px]">
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart data={data} margin={{ top: 5, right: 5, left: -10, bottom: 0 }}>
            <defs>
              <linearGradient id="gradCpu" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#3b82f6" stopOpacity={0.25} />
                <stop offset="95%" stopColor="#3b82f6" stopOpacity={0.02} />
              </linearGradient>
              <linearGradient id="gradMemory" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#a855f7" stopOpacity={0.25} />
                <stop offset="95%" stopColor="#a855f7" stopOpacity={0.02} />
              </linearGradient>
            </defs>
            <CartesianGrid
              strokeDasharray="3 3"
              stroke="hsl(var(--border))"
              strokeOpacity={0.5}
              vertical={false}
            />
            <XAxis
              dataKey="time"
              tick={{ fontSize: 10, fill: "hsl(var(--muted-foreground))" }}
              tickLine={false}
              axisLine={false}
              interval="preserveStartEnd"
              minTickGap={40}
            />
            <YAxis
              tick={{ fontSize: 10, fill: "hsl(var(--muted-foreground))" }}
              tickLine={false}
              axisLine={false}
              tickFormatter={(v) => `${v}%`}
              domain={[0, 100]}
              ticks={[0, 25, 50, 75, 100]}
            />
            <Tooltip content={<CustomTooltip />} />
            <Legend
              wrapperStyle={{ fontSize: "12px", paddingTop: "10px" }}
              iconType="circle"
              iconSize={8}
            />
            <Area
              type="monotone"
              dataKey="memory"
              name="Memory"
              stroke="#a855f7"
              strokeWidth={2}
              fill="url(#gradMemory)"
              dot={false}
              activeDot={{ r: 4, strokeWidth: 0, fill: "#a855f7" }}
            />
            <Area
              type="monotone"
              dataKey="cpu"
              name="CPU"
              stroke="#3b82f6"
              strokeWidth={2}
              fill="url(#gradCpu)"
              dot={false}
              activeDot={{ r: 4, strokeWidth: 0, fill: "#3b82f6" }}
            />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      </CardContent>
    </Card>
  );
}
