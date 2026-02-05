import { useMemo, useState } from "react";
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

// Time range options
const TIME_RANGES = [
  { value: "1", label: "Last 1 hour", axisLabels: ["1h ago", "45m", "30m", "15m", "Now"] },
  { value: "6", label: "Last 6 hours", axisLabels: ["6h ago", "4h 30m", "3h", "1h 30m", "Now"] },
  { value: "24", label: "Last 24 hours", axisLabels: ["24h ago", "18h", "12h", "6h", "Now"] },
  { value: "168", label: "Last 7 days", axisLabels: ["7d ago", "5d", "3d", "1d", "Now"] },
  { value: "720", label: "Last 30 days", axisLabels: ["30d ago", "22d", "15d", "7d", "Now"] },
] as const;

// Fetch stats history from API
async function fetchStatsHistory(hours: number): Promise<DataPoint[]> {
  try {
    // Get auth token from localStorage
    const token = localStorage.getItem("rivetr_auth_token");
    const headers: HeadersInit = {
      "Content-Type": "application/json",
    };
    if (token) {
      headers["Authorization"] = `Bearer ${token}`;
    }

    const response = await fetch(`/api/system/stats/history?hours=${hours}`, { headers });
    if (!response.ok) {
      throw new Error("Failed to fetch stats history");
    }
    const data = await response.json();

    if (!data.history || data.history.length === 0) {
      return [];
    }

    // Convert API response to chart data points
    return data.history.map((point: StatsHistoryPoint) => {
      const date = new Date(point.timestamp);
      const timeStr = date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
      const memoryPercent = point.memory_total_bytes > 0
        ? (point.memory_used_bytes / point.memory_total_bytes) * 100
        : 0;

      return {
        time: timeStr,
        cpu: point.cpu_percent,
        memory: memoryPercent,
      };
    });
  } catch (error) {
    console.error("Error fetching stats history:", error);
    return [];
  }
}

// Generate placeholder data when no history is available
function generatePlaceholderData(cpu: number, memory: number): DataPoint[] {
  const points: DataPoint[] = [];
  for (let i = 23; i >= 0; i--) {
    points.push({
      time: `${i}h`,
      cpu: i === 0 ? cpu : 0,
      memory: i === 0 ? memory : 0,
    });
  }
  return points;
}

export function ResourceChart({ cpuPercent, memoryPercent }: ResourceChartProps) {
  const [timeRange, setTimeRange] = useState("24");

  // Get the current time range config
  const timeRangeConfig = TIME_RANGES.find(t => t.value === timeRange) ?? TIME_RANGES[2];

  // Fetch real historical data
  const { data: historyData } = useQuery({
    queryKey: ["statsHistory", timeRange],
    queryFn: () => fetchStatsHistory(parseInt(timeRange)),
    refetchInterval: 60000, // Refresh every minute
    staleTime: 30000,
  });

  // Use real data if available, otherwise show placeholder with current values
  const data = useMemo(() => {
    if (historyData && historyData.length > 0) {
      return historyData;
    }
    return generatePlaceholderData(cpuPercent, memoryPercent);
  }, [historyData, cpuPercent, memoryPercent]);

  const width = 100;
  const height = 40;
  const padding = { top: 2, right: 2, bottom: 2, left: 2 };
  const chartWidth = width - padding.left - padding.right;
  const chartHeight = height - padding.top - padding.bottom;

  // Create path strings for CPU and Memory lines
  const createPath = (values: number[]) => {
    if (values.length === 0) return "";

    const xStep = chartWidth / (values.length - 1);
    const points = values.map((value, i) => {
      const x = padding.left + i * xStep;
      const y = padding.top + chartHeight - (value / 100) * chartHeight;
      return `${x},${y}`;
    });

    return `M${points.join(" L")}`;
  };

  const createAreaPath = (values: number[]) => {
    if (values.length === 0) return "";

    const xStep = chartWidth / (values.length - 1);
    const points = values.map((value, i) => {
      const x = padding.left + i * xStep;
      const y = padding.top + chartHeight - (value / 100) * chartHeight;
      return `${x},${y}`;
    });

    const baseline = padding.top + chartHeight;
    return `M${padding.left},${baseline} L${points.join(" L")} L${padding.left + chartWidth},${baseline} Z`;
  };

  const cpuValues = data.map(d => d.cpu);
  const memoryValues = data.map(d => d.memory);

  const cpuPath = createPath(cpuValues);
  const memoryPath = createPath(memoryValues);
  const cpuAreaPath = createAreaPath(cpuValues);
  const memoryAreaPath = createAreaPath(memoryValues);

  return (
    <Card>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle>Global Resource Utilization</CardTitle>
          <div className="flex items-center gap-4">
            <div className="flex items-center gap-4 text-xs">
              <div className="flex items-center gap-1.5">
                <div className="h-2.5 w-2.5 rounded-full bg-blue-500" />
                <span className="text-muted-foreground">CPU</span>
              </div>
              <div className="flex items-center gap-1.5">
                <div className="h-2.5 w-2.5 rounded-full bg-purple-500" />
                <span className="text-muted-foreground">Memory</span>
              </div>
            </div>
            <Select value={timeRange} onValueChange={setTimeRange}>
              <SelectTrigger className="w-[140px] h-8">
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
        </div>
      </CardHeader>
      <CardContent>
        <div className="h-[200px] w-full">
          <svg
            viewBox={`0 0 ${width} ${height}`}
            preserveAspectRatio="none"
            className="h-full w-full"
          >
            {/* Grid lines */}
            <defs>
              <linearGradient id="cpuGradient" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor="rgb(59, 130, 246)" stopOpacity="0.3" />
                <stop offset="100%" stopColor="rgb(59, 130, 246)" stopOpacity="0.05" />
              </linearGradient>
              <linearGradient id="memoryGradient" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor="rgb(168, 85, 247)" stopOpacity="0.3" />
                <stop offset="100%" stopColor="rgb(168, 85, 247)" stopOpacity="0.05" />
              </linearGradient>
            </defs>

            {/* Horizontal grid lines */}
            {[0, 25, 50, 75, 100].map((percent) => {
              const y = padding.top + chartHeight - (percent / 100) * chartHeight;
              return (
                <line
                  key={percent}
                  x1={padding.left}
                  y1={y}
                  x2={width - padding.right}
                  y2={y}
                  stroke="currentColor"
                  strokeOpacity="0.1"
                  strokeWidth="0.1"
                />
              );
            })}

            {/* Memory area and line */}
            <path
              d={memoryAreaPath}
              fill="url(#memoryGradient)"
            />
            <path
              d={memoryPath}
              fill="none"
              stroke="rgb(168, 85, 247)"
              strokeWidth="0.4"
              strokeLinecap="round"
              strokeLinejoin="round"
            />

            {/* CPU area and line */}
            <path
              d={cpuAreaPath}
              fill="url(#cpuGradient)"
            />
            <path
              d={cpuPath}
              fill="none"
              stroke="rgb(59, 130, 246)"
              strokeWidth="0.4"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </div>

        {/* X-axis labels */}
        <div className="flex justify-between mt-2 text-xs text-muted-foreground px-1">
          {timeRangeConfig.axisLabels.map((label, i) => (
            <span key={i}>{label}</span>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
