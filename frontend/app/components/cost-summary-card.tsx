import { useMemo } from "react";
import { Link } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { api } from "@/lib/api";
import { DollarSign, TrendingUp, TrendingDown, ArrowRight } from "lucide-react";
import type { DashboardCostResponse, DailyCostPoint } from "@/types/api";

/** Format currency value */
function formatCurrency(value: number): string {
  if (value === 0) return "$0.00";
  if (value < 0.01) return "<$0.01";
  if (value >= 1000) {
    return `$${(value / 1000).toFixed(1)}k`;
  }
  return `$${value.toFixed(2)}`;
}

/** Create sparkline path from trend data */
function createSparklinePath(data: DailyCostPoint[], width: number, height: number): string {
  if (data.length === 0) return "";

  const values = data.map((d) => d.total_cost);
  const maxValue = Math.max(...values, 0.01); // Avoid division by zero
  const xStep = width / (values.length - 1 || 1);

  const points = values.map((value, i) => {
    const x = i * xStep;
    const y = height - (value / maxValue) * height;
    return `${x},${y}`;
  });

  return `M${points.join(" L")}`;
}

/** Calculate trend percentage between first and last half of data */
function calculateTrend(data: DailyCostPoint[]): { percent: number; isUp: boolean } {
  if (data.length < 2) return { percent: 0, isUp: false };

  const midpoint = Math.floor(data.length / 2);
  const firstHalf = data.slice(0, midpoint);
  const secondHalf = data.slice(midpoint);

  const firstAvg =
    firstHalf.reduce((sum, d) => sum + d.total_cost, 0) / (firstHalf.length || 1);
  const secondAvg =
    secondHalf.reduce((sum, d) => sum + d.total_cost, 0) / (secondHalf.length || 1);

  if (firstAvg === 0) return { percent: 0, isUp: false };

  const percent = ((secondAvg - firstAvg) / firstAvg) * 100;
  return { percent: Math.abs(percent), isUp: percent > 0 };
}

export function CostSummaryCard() {
  const { data: costData, isLoading } = useQuery<DashboardCostResponse | null>({
    queryKey: ["dashboard-costs"],
    queryFn: () => api.getDashboardCosts("30d"),
    refetchInterval: 60000, // Refresh every minute
    staleTime: 30000,
  });

  // Calculate sparkline path
  const sparklineConfig = useMemo(() => {
    if (!costData?.trend || costData.trend.length === 0) {
      return { path: "", trend: { percent: 0, isUp: false } };
    }
    const width = 80;
    const height = 24;
    return {
      path: createSparklinePath(costData.trend, width, height),
      trend: calculateTrend(costData.trend),
    };
  }, [costData?.trend]);

  const summary = costData?.summary;
  const topApps = costData?.top_apps ?? [];

  return (
    <Card>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div className="rounded-lg p-2 bg-emerald-100 dark:bg-emerald-900/30">
              <DollarSign className="h-4 w-4 text-emerald-600" />
            </div>
            <CardTitle className="text-base">Cost Overview</CardTitle>
          </div>
          <Button variant="ghost" size="sm" asChild className="text-xs">
            <Link to="/costs">
              View Details
              <ArrowRight className="ml-1 h-3 w-3" />
            </Link>
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="space-y-3 animate-pulse">
            <div className="h-8 bg-muted rounded w-24" />
            <div className="h-4 bg-muted rounded w-32" />
            <div className="space-y-2">
              <div className="h-3 bg-muted rounded w-full" />
              <div className="h-3 bg-muted rounded w-3/4" />
            </div>
          </div>
        ) : (
          <div className="space-y-4">
            {/* Main cost display with sparkline */}
            <div className="flex items-start justify-between">
              <div>
                <p className="text-2xl font-bold">
                  {formatCurrency(summary?.projected_monthly_cost ?? 0)}
                </p>
                <p className="text-xs text-muted-foreground">
                  Projected monthly cost
                </p>
              </div>
              <div className="flex flex-col items-end gap-1">
                {/* Sparkline */}
                {sparklineConfig.path && (
                  <svg
                    width="80"
                    height="24"
                    viewBox="0 0 80 24"
                    className="overflow-visible"
                  >
                    <defs>
                      <linearGradient
                        id="costSparklineGradient"
                        x1="0"
                        y1="0"
                        x2="0"
                        y2="1"
                      >
                        <stop
                          offset="0%"
                          stopColor="rgb(16, 185, 129)"
                          stopOpacity="0.3"
                        />
                        <stop
                          offset="100%"
                          stopColor="rgb(16, 185, 129)"
                          stopOpacity="0.05"
                        />
                      </linearGradient>
                    </defs>
                    <path
                      d={sparklineConfig.path}
                      fill="none"
                      stroke="rgb(16, 185, 129)"
                      strokeWidth="1.5"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    />
                  </svg>
                )}
                {/* Trend indicator */}
                {sparklineConfig.trend.percent > 0 && (
                  <Badge
                    variant="secondary"
                    className={`text-xs ${
                      sparklineConfig.trend.isUp
                        ? "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400"
                        : "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400"
                    }`}
                  >
                    {sparklineConfig.trend.isUp ? (
                      <TrendingUp className="h-3 w-3 mr-1" />
                    ) : (
                      <TrendingDown className="h-3 w-3 mr-1" />
                    )}
                    {sparklineConfig.trend.percent.toFixed(1)}%
                  </Badge>
                )}
              </div>
            </div>

            {/* Period cost breakdown */}
            <div className="grid grid-cols-3 gap-2 text-center">
              <div className="rounded-md bg-muted/50 p-2">
                <p className="text-xs text-muted-foreground">CPU</p>
                <p className="text-sm font-medium">
                  {formatCurrency(summary?.cpu_cost ?? 0)}
                </p>
              </div>
              <div className="rounded-md bg-muted/50 p-2">
                <p className="text-xs text-muted-foreground">Memory</p>
                <p className="text-sm font-medium">
                  {formatCurrency(summary?.memory_cost ?? 0)}
                </p>
              </div>
              <div className="rounded-md bg-muted/50 p-2">
                <p className="text-xs text-muted-foreground">Disk</p>
                <p className="text-sm font-medium">
                  {formatCurrency(summary?.disk_cost ?? 0)}
                </p>
              </div>
            </div>

            {/* Top apps by cost */}
            {topApps.length > 0 && (
              <div className="space-y-2">
                <p className="text-xs font-medium text-muted-foreground">
                  Top Apps by Cost
                </p>
                <div className="space-y-1.5">
                  {topApps.slice(0, 5).map((app) => (
                    <div
                      key={app.app_id}
                      className="flex items-center justify-between text-sm"
                    >
                      <Link
                        to={`/apps/${app.app_id}`}
                        className="text-muted-foreground hover:text-foreground truncate max-w-[60%]"
                      >
                        {app.app_name}
                      </Link>
                      <span className="font-medium">
                        {formatCurrency(app.total_cost)}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Empty state */}
            {!isLoading && topApps.length === 0 && (!summary || summary.total_cost === 0) && (
              <div className="text-center py-4 text-muted-foreground text-sm">
                <p>No cost data available yet.</p>
                <p className="text-xs mt-1">
                  Cost data will appear once apps start running.
                </p>
              </div>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
