import { useMemo } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

interface DataPoint {
  time: string;
  cpu: number;
  memory: number;
}

interface ResourceChartProps {
  cpuPercent: number;
  memoryPercent: number;
}

// Generate mock historical data based on current values
function generateMockData(cpu: number, memory: number): DataPoint[] {
  const now = new Date();
  const points: DataPoint[] = [];

  for (let i = 23; i >= 0; i--) {
    const time = new Date(now.getTime() - i * 60 * 60 * 1000);
    const timeStr = time.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });

    // Add some variation around the current values
    const variation = () => (Math.random() - 0.5) * 20;
    points.push({
      time: timeStr,
      cpu: Math.max(0, Math.min(100, cpu + variation() + (i * 0.5 - 12))),
      memory: Math.max(0, Math.min(100, memory + variation() * 0.5 + (i * 0.2 - 5))),
    });
  }

  // Ensure last point matches current values
  if (points.length > 0) {
    points[points.length - 1].cpu = cpu;
    points[points.length - 1].memory = memory;
  }

  return points;
}

export function ResourceChart({ cpuPercent, memoryPercent }: ResourceChartProps) {
  const data = useMemo(
    () => generateMockData(cpuPercent, memoryPercent),
    [cpuPercent, memoryPercent]
  );

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
          <span>24h ago</span>
          <span>18h ago</span>
          <span>12h ago</span>
          <span>6h ago</span>
          <span>Now</span>
        </div>
      </CardContent>
    </Card>
  );
}
