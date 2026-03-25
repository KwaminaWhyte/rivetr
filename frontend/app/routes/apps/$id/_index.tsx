import { useOutletContext } from "react-router";
import { useParams } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { getPrimaryDomain } from "@/lib/utils";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { ResourceLimitsCard } from "@/components/resource-limits-card";
import { ResourceMonitor } from "@/components/resource-monitor";
import { EnvironmentBadge } from "@/components/environment-badge";
import { api } from "@/lib/api";
import { aiApi } from "@/lib/api/ai";
import type { App, AuditLog, AuditLogListResponse, Deployment } from "@/types/api";
import {
  RotateCw,
  Play,
  Square,
  Rocket,
  Pencil,
  Trash2,
  Activity,
  Sparkles,
  TrendingUp,
  TrendingDown,
  Minus,
} from "lucide-react";

export function meta() {
  return [
    { title: "App Overview - Rivetr" },
    { name: "description", content: "Application overview, status, and resource usage" },
  ];
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString();
}

interface OutletContext {
  app: App;
  deployments: Deployment[];
  token: string;
}

function activityIcon(action: string) {
  if (action === "app.restart") return <RotateCw className="h-3.5 w-3.5 text-blue-500" />;
  if (action === "app.start") return <Play className="h-3.5 w-3.5 text-green-500" />;
  if (action === "app.stop") return <Square className="h-3.5 w-3.5 text-yellow-500" />;
  if (action === "deployment.trigger") return <Rocket className="h-3.5 w-3.5 text-purple-500" />;
  if (action === "deployment.rollback") return <RotateCw className="h-3.5 w-3.5 text-orange-500" />;
  if (action === "app.update") return <Pencil className="h-3.5 w-3.5 text-muted-foreground" />;
  if (action === "app.delete") return <Trash2 className="h-3.5 w-3.5 text-destructive" />;
  return <Activity className="h-3.5 w-3.5 text-muted-foreground" />;
}

function activityLabel(action: string): string {
  const labels: Record<string, string> = {
    "app.restart": "Restarted",
    "app.start": "Started",
    "app.stop": "Stopped",
    "app.create": "Created",
    "app.update": "Updated settings",
    "app.delete": "Deleted",
    "deployment.trigger": "Deployment triggered",
    "deployment.rollback": "Rollback triggered",
  };
  return labels[action] ?? action;
}

function TrendIcon({ trend }: { trend: "improving" | "degrading" | "stable" }) {
  if (trend === "improving") return <TrendingUp className="h-4 w-4 text-green-500" />;
  if (trend === "degrading") return <TrendingDown className="h-4 w-4 text-red-500" />;
  return <Minus className="h-4 w-4 text-muted-foreground" />;
}

function trendLabel(trend: "improving" | "degrading" | "stable"): string {
  if (trend === "improving") return "Improving";
  if (trend === "degrading") return "Degrading";
  return "Stable";
}

function trendColorClass(trend: "improving" | "degrading" | "stable"): string {
  if (trend === "improving") return "text-green-600";
  if (trend === "degrading") return "text-red-600";
  return "text-muted-foreground";
}

export default function AppGeneralTab() {
  const { app, deployments, token } = useOutletContext<OutletContext>();
  const { id } = useParams();
  const runningDeployment = deployments.find((d) => d.status === "running");

  const { data: activityData, isLoading: activityLoading } = useQuery<AuditLogListResponse>({
    queryKey: ["appActivity", id],
    queryFn: () => api.getAppActivity(id!),
    enabled: !!id,
    refetchInterval: 30000,
  });

  const {
    data: insightsData,
    isLoading: insightsLoading,
    error: insightsError,
  } = useQuery({
    queryKey: ["ai-insights", id],
    queryFn: () => aiApi.getInsights(id!),
    enabled: !!id,
    retry: false,
  });

  return (
    <div className="space-y-6">
      <div className="grid gap-6 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Configuration</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <div className="text-sm text-muted-foreground">Environment</div>
                <div className="font-medium mt-1">
                  <EnvironmentBadge environment={app.environment} />
                </div>
              </div>
              <div>
                <div className="text-sm text-muted-foreground">Branch</div>
                <div className="font-medium">{app.branch}</div>
              </div>
              <div>
                <div className="text-sm text-muted-foreground">Port</div>
                <div className="font-medium">{app.port}</div>
              </div>
              <div>
                <div className="text-sm text-muted-foreground">Dockerfile</div>
                <div className="font-medium">{app.dockerfile}</div>
              </div>
              <div>
                <div className="text-sm text-muted-foreground">Domain</div>
                <div className="font-medium">{getPrimaryDomain(app) || "-"}</div>
              </div>
              <div>
                <div className="text-sm text-muted-foreground">Healthcheck</div>
                <div className="font-medium">{app.healthcheck || "-"}</div>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Details</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div>
              <div className="text-sm text-muted-foreground">App ID</div>
              <div className="font-mono text-sm">{app.id}</div>
            </div>
            <div>
              <div className="text-sm text-muted-foreground">Created</div>
              <div className="font-medium">{formatDate(app.created_at)}</div>
            </div>
            <div>
              <div className="text-sm text-muted-foreground">Updated</div>
              <div className="font-medium">{formatDate(app.updated_at)}</div>
            </div>
          </CardContent>
        </Card>
      </div>

      <ResourceLimitsCard app={app} token={token} />

      {runningDeployment && (
        <ResourceMonitor
          appId={app.id}
          token={token}
          cpuLimit={app.cpu_limit ? parseFloat(app.cpu_limit) : undefined}
        />
      )}

      {/* Activity / Restart history */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Activity className="h-4 w-4" />
            Activity
          </CardTitle>
        </CardHeader>
        <CardContent>
          {activityLoading ? (
            <div className="flex items-center gap-2 text-sm text-muted-foreground py-4">
              <RotateCw className="h-4 w-4 animate-spin" />
              Loading activity...
            </div>
          ) : !activityData || activityData.items.length === 0 ? (
            <p className="text-sm text-muted-foreground py-4">
              No activity recorded yet. Start, stop, restart, and deploy events will appear here.
            </p>
          ) : (
            <ol className="relative border-l border-border ml-2 space-y-0">
              {activityData.items.map((entry: AuditLog) => (
                <li key={entry.id} className="ml-4 pb-4">
                  <span className="absolute -left-[9px] flex h-4 w-4 items-center justify-center rounded-full bg-background border border-border">
                    {activityIcon(entry.action)}
                  </span>
                  <div className="flex items-baseline gap-2 flex-wrap">
                    <span className="text-sm font-medium">{activityLabel(entry.action)}</span>
                    {entry.user_email && (
                      <span className="text-xs text-muted-foreground">by {entry.user_email}</span>
                    )}
                    <time className="text-xs text-muted-foreground ml-auto">
                      {new Date(entry.created_at).toLocaleString()}
                    </time>
                  </div>
                </li>
              ))}
            </ol>
          )}
        </CardContent>
      </Card>

      {/* AI Insights Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Sparkles className="h-4 w-4 text-purple-500" />
            AI Insights
          </CardTitle>
        </CardHeader>
        <CardContent>
          {insightsLoading ? (
            <div className="space-y-3">
              <Skeleton className="h-4 w-3/4" />
              <Skeleton className="h-4 w-1/2" />
              <div className="flex gap-3 mt-4">
                <Skeleton className="h-8 w-24" />
                <Skeleton className="h-8 w-24" />
                <Skeleton className="h-8 w-24" />
              </div>
            </div>
          ) : insightsError || !insightsData ? (
            <p className="text-sm text-muted-foreground py-2">
              AI not configured. Enable an AI provider in instance settings to see deployment insights.
            </p>
          ) : (
            <div className="space-y-4">
              <p className="text-sm text-muted-foreground">{insightsData.summary}</p>
              <div className="flex flex-wrap gap-3">
                <Badge variant="outline" className="gap-1.5 px-3 py-1.5 text-sm">
                  <span className="text-muted-foreground">Success rate</span>
                  <span className="font-semibold">{insightsData.success_rate_percent.toFixed(1)}%</span>
                </Badge>
                <Badge variant="outline" className="gap-1.5 px-3 py-1.5 text-sm">
                  <span className="text-muted-foreground">Avg build</span>
                  <span className="font-semibold">{insightsData.avg_build_minutes.toFixed(1)}m</span>
                </Badge>
                <Badge variant="outline" className="gap-1.5 px-3 py-1.5 text-sm">
                  <span className="text-muted-foreground">Total deploys</span>
                  <span className="font-semibold">{insightsData.total_deployments}</span>
                </Badge>
                <Badge variant="outline" className={`gap-1.5 px-3 py-1.5 text-sm ${trendColorClass(insightsData.trend)}`}>
                  <TrendIcon trend={insightsData.trend} />
                  {trendLabel(insightsData.trend)}
                </Badge>
              </div>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
