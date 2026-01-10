import { useState, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Skeleton } from "@/components/ui/skeleton";
import { api } from "@/lib/api";
import type {
  DashboardCostResponse,
  CostResponse,
  CostSummary,
  AppCostBreakdown,
  DailyCostPoint,
  TeamWithMemberCount,
  Project,
} from "@/types/api";
import {
  DollarSign,
  TrendingUp,
  TrendingDown,
  Download,
  ChevronDown,
  ChevronRight,
  Building2,
  FolderOpen,
  Box,
  RefreshCw,
  Cpu,
  HardDrive,
  Database,
} from "lucide-react";

export function meta() {
  return [
    { title: "Cost Analysis - Rivetr" },
    { name: "description", content: "Analyze costs across teams, projects, and apps" },
  ];
}

type Period = "7d" | "30d" | "90d";

/** Format currency value */
function formatCurrency(value: number): string {
  if (value === 0) return "$0.00";
  if (value < 0.01) return "<$0.01";
  if (value >= 1000) {
    return `$${(value / 1000).toFixed(2)}k`;
  }
  return `$${value.toFixed(2)}`;
}

/** Create SVG path for cost breakdown chart */
function createBarChartData(cpu: number, memory: number, disk: number): { type: string; value: number; color: string; percent: number }[] {
  const total = cpu + memory + disk;
  if (total === 0) {
    return [
      { type: "CPU", value: 0, color: "bg-blue-500", percent: 33.3 },
      { type: "Memory", value: 0, color: "bg-purple-500", percent: 33.3 },
      { type: "Disk", value: 0, color: "bg-orange-500", percent: 33.3 },
    ];
  }
  return [
    { type: "CPU", value: cpu, color: "bg-blue-500", percent: (cpu / total) * 100 },
    { type: "Memory", value: memory, color: "bg-purple-500", percent: (memory / total) * 100 },
    { type: "Disk", value: disk, color: "bg-orange-500", percent: (disk / total) * 100 },
  ];
}

/** Calculate trend percentage between first and last half of data */
function calculateTrend(data: DailyCostPoint[]): { percent: number; isUp: boolean } {
  if (data.length < 2) return { percent: 0, isUp: false };

  const midpoint = Math.floor(data.length / 2);
  const firstHalf = data.slice(0, midpoint);
  const secondHalf = data.slice(midpoint);

  const firstAvg = firstHalf.reduce((sum, d) => sum + d.total_cost, 0) / (firstHalf.length || 1);
  const secondAvg = secondHalf.reduce((sum, d) => sum + d.total_cost, 0) / (secondHalf.length || 1);

  if (firstAvg === 0) return { percent: 0, isUp: false };

  const percent = ((secondAvg - firstAvg) / firstAvg) * 100;
  return { percent: Math.abs(percent), isUp: percent > 0 };
}

/** Generate CSV content from cost data */
function generateCostsCsv(
  dashboardCosts: DashboardCostResponse | null,
  teams: TeamWithMemberCount[],
  teamCosts: Record<string, CostResponse | undefined>,
  projects: Project[],
  projectCosts: Record<string, CostResponse | undefined>,
  period: Period
): string {
  const lines: string[] = [];

  // Header
  lines.push("Type,Name,ID,CPU Cost,Memory Cost,Disk Cost,Total Cost,Period");

  // System total
  if (dashboardCosts?.summary) {
    const s = dashboardCosts.summary;
    lines.push(`System,Total,,${s.cpu_cost.toFixed(4)},${s.memory_cost.toFixed(4)},${s.disk_cost.toFixed(4)},${s.total_cost.toFixed(4)},${period}`);
  }

  // Team data
  teams.forEach((team) => {
    const costs = teamCosts[team.id];
    if (costs?.summary) {
      const s = costs.summary;
      lines.push(`Team,${team.name},${team.id},${s.cpu_cost.toFixed(4)},${s.memory_cost.toFixed(4)},${s.disk_cost.toFixed(4)},${s.total_cost.toFixed(4)},${period}`);
    }
    // Project breakdowns
    if (costs?.breakdown) {
      costs.breakdown.forEach((app) => {
        lines.push(`App (Team: ${team.name}),${app.app_name},${app.app_id},${app.cpu_cost.toFixed(4)},${app.memory_cost.toFixed(4)},${app.disk_cost.toFixed(4)},${app.total_cost.toFixed(4)},${period}`);
      });
    }
  });

  // Project data
  projects.forEach((project) => {
    const costs = projectCosts[project.id];
    if (costs?.summary) {
      const s = costs.summary;
      lines.push(`Project,${project.name},${project.id},${s.cpu_cost.toFixed(4)},${s.memory_cost.toFixed(4)},${s.disk_cost.toFixed(4)},${s.total_cost.toFixed(4)},${period}`);
    }
    if (costs?.breakdown) {
      costs.breakdown.forEach((app) => {
        lines.push(`App (Project: ${project.name}),${app.app_name},${app.app_id},${app.cpu_cost.toFixed(4)},${app.memory_cost.toFixed(4)},${app.disk_cost.toFixed(4)},${app.total_cost.toFixed(4)},${period}`);
      });
    }
  });

  // Top apps from dashboard
  if (dashboardCosts?.top_apps) {
    dashboardCosts.top_apps.forEach((app) => {
      lines.push(`App (Top),${app.app_name},${app.app_id},${app.cpu_cost.toFixed(4)},${app.memory_cost.toFixed(4)},${app.disk_cost.toFixed(4)},${app.total_cost.toFixed(4)},${period}`);
    });
  }

  return lines.join("\n");
}

/** Download CSV file */
function downloadCsv(content: string, filename: string) {
  const blob = new Blob([content], { type: "text/csv;charset=utf-8;" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.setAttribute("download", filename);
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}

/** Cost breakdown bar chart component */
function CostBreakdownChart({ summary }: { summary: CostSummary }) {
  const data = createBarChartData(summary.cpu_cost, summary.memory_cost, summary.disk_cost);
  const total = summary.cpu_cost + summary.memory_cost + summary.disk_cost;

  return (
    <div className="space-y-4">
      {/* Stacked bar */}
      <div className="h-8 rounded-full overflow-hidden flex bg-muted">
        {data.map((item) => (
          <div
            key={item.type}
            className={`${item.color} transition-all duration-300`}
            style={{ width: `${item.percent}%` }}
            title={`${item.type}: ${formatCurrency(item.value)} (${item.percent.toFixed(1)}%)`}
          />
        ))}
      </div>

      {/* Legend */}
      <div className="grid grid-cols-3 gap-4">
        {data.map((item) => (
          <div key={item.type} className="flex items-center gap-2">
            <div className={`w-3 h-3 rounded-full ${item.color}`} />
            <div className="flex flex-col">
              <span className="text-sm font-medium">{item.type}</span>
              <span className="text-xs text-muted-foreground">
                {formatCurrency(item.value)} ({total > 0 ? item.percent.toFixed(1) : "0.0"}%)
              </span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

/** Expandable team cost row */
function TeamCostRow({
  team,
  period,
  isExpanded,
  onToggle,
}: {
  team: TeamWithMemberCount;
  period: Period;
  isExpanded: boolean;
  onToggle: () => void;
}) {
  const { data: costs, isLoading } = useQuery<CostResponse>({
    queryKey: ["team-costs", team.id, period],
    queryFn: () => api.getTeamCosts(team.id, period),
    enabled: isExpanded,
  });

  return (
    <Collapsible open={isExpanded} onOpenChange={onToggle}>
      <TableRow className="cursor-pointer hover:bg-muted/50" onClick={onToggle}>
        <TableCell>
          <CollapsibleTrigger asChild>
            <div className="flex items-center gap-2">
              {isExpanded ? (
                <ChevronDown className="h-4 w-4 text-muted-foreground" />
              ) : (
                <ChevronRight className="h-4 w-4 text-muted-foreground" />
              )}
              <Building2 className="h-4 w-4 text-muted-foreground" />
              <span className="font-medium">{team.name}</span>
              <Badge variant="outline" className="text-xs">Team</Badge>
            </div>
          </CollapsibleTrigger>
        </TableCell>
        <TableCell className="text-right">
          {isExpanded && isLoading ? (
            <Skeleton className="h-4 w-16 ml-auto" />
          ) : isExpanded && costs ? (
            formatCurrency(costs.summary.cpu_cost)
          ) : (
            "—"
          )}
        </TableCell>
        <TableCell className="text-right">
          {isExpanded && isLoading ? (
            <Skeleton className="h-4 w-16 ml-auto" />
          ) : isExpanded && costs ? (
            formatCurrency(costs.summary.memory_cost)
          ) : (
            "—"
          )}
        </TableCell>
        <TableCell className="text-right">
          {isExpanded && isLoading ? (
            <Skeleton className="h-4 w-16 ml-auto" />
          ) : isExpanded && costs ? (
            formatCurrency(costs.summary.disk_cost)
          ) : (
            "—"
          )}
        </TableCell>
        <TableCell className="text-right font-medium">
          {isExpanded && isLoading ? (
            <Skeleton className="h-4 w-20 ml-auto" />
          ) : isExpanded && costs ? (
            formatCurrency(costs.summary.total_cost)
          ) : (
            "—"
          )}
        </TableCell>
      </TableRow>
      <CollapsibleContent asChild>
        <>
          {isExpanded && costs?.breakdown?.map((app) => (
            <TableRow key={app.app_id} className="bg-muted/30">
              <TableCell className="pl-12">
                <Link
                  to={`/apps/${app.app_id}`}
                  className="flex items-center gap-2 hover:text-primary"
                  onClick={(e) => e.stopPropagation()}
                >
                  <Box className="h-4 w-4 text-muted-foreground" />
                  <span>{app.app_name}</span>
                  <Badge variant="secondary" className="text-xs">App</Badge>
                </Link>
              </TableCell>
              <TableCell className="text-right text-muted-foreground">
                {formatCurrency(app.cpu_cost)}
              </TableCell>
              <TableCell className="text-right text-muted-foreground">
                {formatCurrency(app.memory_cost)}
              </TableCell>
              <TableCell className="text-right text-muted-foreground">
                {formatCurrency(app.disk_cost)}
              </TableCell>
              <TableCell className="text-right text-muted-foreground">
                {formatCurrency(app.total_cost)}
              </TableCell>
            </TableRow>
          ))}
          {isExpanded && (!costs?.breakdown || costs.breakdown.length === 0) && !isLoading && (
            <TableRow className="bg-muted/30">
              <TableCell colSpan={5} className="text-center text-muted-foreground py-4">
                No apps with cost data in this team
              </TableCell>
            </TableRow>
          )}
        </>
      </CollapsibleContent>
    </Collapsible>
  );
}

/** Expandable project cost row */
function ProjectCostRow({
  project,
  period,
  isExpanded,
  onToggle,
}: {
  project: Project;
  period: Period;
  isExpanded: boolean;
  onToggle: () => void;
}) {
  const { data: costs, isLoading } = useQuery<CostResponse>({
    queryKey: ["project-costs", project.id, period],
    queryFn: () => api.getProjectCosts(project.id, period),
    enabled: isExpanded,
  });

  return (
    <Collapsible open={isExpanded} onOpenChange={onToggle}>
      <TableRow className="cursor-pointer hover:bg-muted/50" onClick={onToggle}>
        <TableCell>
          <CollapsibleTrigger asChild>
            <div className="flex items-center gap-2">
              {isExpanded ? (
                <ChevronDown className="h-4 w-4 text-muted-foreground" />
              ) : (
                <ChevronRight className="h-4 w-4 text-muted-foreground" />
              )}
              <FolderOpen className="h-4 w-4 text-muted-foreground" />
              <span className="font-medium">{project.name}</span>
              <Badge variant="outline" className="text-xs">Project</Badge>
            </div>
          </CollapsibleTrigger>
        </TableCell>
        <TableCell className="text-right">
          {isExpanded && isLoading ? (
            <Skeleton className="h-4 w-16 ml-auto" />
          ) : isExpanded && costs ? (
            formatCurrency(costs.summary.cpu_cost)
          ) : (
            "—"
          )}
        </TableCell>
        <TableCell className="text-right">
          {isExpanded && isLoading ? (
            <Skeleton className="h-4 w-16 ml-auto" />
          ) : isExpanded && costs ? (
            formatCurrency(costs.summary.memory_cost)
          ) : (
            "—"
          )}
        </TableCell>
        <TableCell className="text-right">
          {isExpanded && isLoading ? (
            <Skeleton className="h-4 w-16 ml-auto" />
          ) : isExpanded && costs ? (
            formatCurrency(costs.summary.disk_cost)
          ) : (
            "—"
          )}
        </TableCell>
        <TableCell className="text-right font-medium">
          {isExpanded && isLoading ? (
            <Skeleton className="h-4 w-20 ml-auto" />
          ) : isExpanded && costs ? (
            formatCurrency(costs.summary.total_cost)
          ) : (
            "—"
          )}
        </TableCell>
      </TableRow>
      <CollapsibleContent asChild>
        <>
          {isExpanded && costs?.breakdown?.map((app) => (
            <TableRow key={app.app_id} className="bg-muted/30">
              <TableCell className="pl-12">
                <Link
                  to={`/apps/${app.app_id}`}
                  className="flex items-center gap-2 hover:text-primary"
                  onClick={(e) => e.stopPropagation()}
                >
                  <Box className="h-4 w-4 text-muted-foreground" />
                  <span>{app.app_name}</span>
                  <Badge variant="secondary" className="text-xs">App</Badge>
                </Link>
              </TableCell>
              <TableCell className="text-right text-muted-foreground">
                {formatCurrency(app.cpu_cost)}
              </TableCell>
              <TableCell className="text-right text-muted-foreground">
                {formatCurrency(app.memory_cost)}
              </TableCell>
              <TableCell className="text-right text-muted-foreground">
                {formatCurrency(app.disk_cost)}
              </TableCell>
              <TableCell className="text-right text-muted-foreground">
                {formatCurrency(app.total_cost)}
              </TableCell>
            </TableRow>
          ))}
          {isExpanded && (!costs?.breakdown || costs.breakdown.length === 0) && !isLoading && (
            <TableRow className="bg-muted/30">
              <TableCell colSpan={5} className="text-center text-muted-foreground py-4">
                No apps with cost data in this project
              </TableCell>
            </TableRow>
          )}
        </>
      </CollapsibleContent>
    </Collapsible>
  );
}

export default function CostsPage() {
  const [period, setPeriod] = useState<Period>("30d");
  const [expandedTeams, setExpandedTeams] = useState<Set<string>>(new Set());
  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(new Set());
  const [viewMode, setViewMode] = useState<"teams" | "projects">("teams");

  // Fetch dashboard costs (system-wide summary)
  const { data: dashboardCosts, isLoading: costsLoading, refetch } = useQuery<DashboardCostResponse | null>({
    queryKey: ["dashboard-costs", period],
    queryFn: () => api.getDashboardCosts(period),
  });

  // Fetch teams for hierarchical view
  const { data: teams = [] } = useQuery<TeamWithMemberCount[]>({
    queryKey: ["teams"],
    queryFn: () => api.getTeams(),
  });

  // Fetch projects for hierarchical view
  const { data: projects = [] } = useQuery<Project[]>({
    queryKey: ["projects"],
    queryFn: () => api.getProjects(),
  });

  // Calculate trend
  const trend = useMemo(() => {
    if (!dashboardCosts?.trend) return { percent: 0, isUp: false };
    return calculateTrend(dashboardCosts.trend);
  }, [dashboardCosts?.trend]);

  // Toggle team expansion
  const toggleTeam = (teamId: string) => {
    setExpandedTeams((prev) => {
      const next = new Set(prev);
      if (next.has(teamId)) {
        next.delete(teamId);
      } else {
        next.add(teamId);
      }
      return next;
    });
  };

  // Toggle project expansion
  const toggleProject = (projectId: string) => {
    setExpandedProjects((prev) => {
      const next = new Set(prev);
      if (next.has(projectId)) {
        next.delete(projectId);
      } else {
        next.add(projectId);
      }
      return next;
    });
  };

  // Export CSV
  const handleExportCsv = () => {
    const teamCosts: Record<string, CostResponse | undefined> = {};
    const projectCosts: Record<string, CostResponse | undefined> = {};

    const csv = generateCostsCsv(dashboardCosts ?? null, teams, teamCosts, projects, projectCosts, period);
    const date = new Date().toISOString().split("T")[0];
    downloadCsv(csv, `rivetr-costs-${period}-${date}.csv`);
  };

  const summary = dashboardCosts?.summary;
  const periodLabel = period === "7d" ? "7 days" : period === "30d" ? "30 days" : "90 days";

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Cost Analysis</h1>
          <p className="text-muted-foreground">
            Analyze infrastructure costs across your organization
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Select value={period} onValueChange={(v) => setPeriod(v as Period)}>
            <SelectTrigger className="w-[140px]">
              <SelectValue placeholder="Select period" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="7d">Last 7 days</SelectItem>
              <SelectItem value="30d">Last 30 days</SelectItem>
              <SelectItem value="90d">Last 90 days</SelectItem>
            </SelectContent>
          </Select>
          <Button variant="outline" size="sm" onClick={() => refetch()}>
            <RefreshCw className="h-4 w-4" />
          </Button>
          <Button variant="outline" size="sm" onClick={handleExportCsv}>
            <Download className="h-4 w-4 mr-2" />
            Export CSV
          </Button>
        </div>
      </div>

      {/* Summary Cards */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total Cost</CardTitle>
            <DollarSign className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            {costsLoading ? (
              <Skeleton className="h-8 w-24" />
            ) : (
              <>
                <div className="text-2xl font-bold">
                  {formatCurrency(summary?.total_cost ?? 0)}
                </div>
                <p className="text-xs text-muted-foreground">
                  Last {periodLabel}
                </p>
              </>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Projected Monthly</CardTitle>
            {trend.percent > 0 && (
              trend.isUp ? (
                <TrendingUp className="h-4 w-4 text-red-500" />
              ) : (
                <TrendingDown className="h-4 w-4 text-green-500" />
              )
            )}
          </CardHeader>
          <CardContent>
            {costsLoading ? (
              <Skeleton className="h-8 w-24" />
            ) : (
              <>
                <div className="text-2xl font-bold">
                  {formatCurrency(summary?.projected_monthly_cost ?? 0)}
                </div>
                {trend.percent > 0 && (
                  <Badge
                    variant="secondary"
                    className={`text-xs ${
                      trend.isUp
                        ? "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400"
                        : "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400"
                    }`}
                  >
                    {trend.isUp ? "+" : "-"}{trend.percent.toFixed(1)}% vs prior period
                  </Badge>
                )}
              </>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Avg. Resources</CardTitle>
            <Cpu className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            {costsLoading ? (
              <Skeleton className="h-8 w-24" />
            ) : (
              <>
                <div className="text-2xl font-bold">
                  {(summary?.avg_cpu_cores ?? 0).toFixed(2)} cores
                </div>
                <p className="text-xs text-muted-foreground">
                  {(summary?.avg_memory_gb ?? 0).toFixed(2)} GB RAM
                </p>
              </>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Days Tracked</CardTitle>
            <Database className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            {costsLoading ? (
              <Skeleton className="h-8 w-16" />
            ) : (
              <>
                <div className="text-2xl font-bold">
                  {summary?.days_in_period ?? 0}
                </div>
                <p className="text-xs text-muted-foreground">
                  of {period === "7d" ? 7 : period === "30d" ? 30 : 90} days
                </p>
              </>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Cost Breakdown Chart */}
      <Card>
        <CardHeader>
          <CardTitle>Cost Breakdown by Resource Type</CardTitle>
          <CardDescription>
            Distribution of costs across CPU, memory, and disk for the last {periodLabel}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {costsLoading ? (
            <div className="space-y-4">
              <Skeleton className="h-8 w-full rounded-full" />
              <div className="grid grid-cols-3 gap-4">
                <Skeleton className="h-12 w-full" />
                <Skeleton className="h-12 w-full" />
                <Skeleton className="h-12 w-full" />
              </div>
            </div>
          ) : summary ? (
            <CostBreakdownChart summary={summary} />
          ) : (
            <div className="text-center py-8 text-muted-foreground">
              <DollarSign className="mx-auto h-12 w-12 text-muted-foreground/50" />
              <p className="mt-4">No cost data available for this period.</p>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Hierarchical Cost View */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Cost by Organization</CardTitle>
              <CardDescription>
                Drill down into costs by {viewMode === "teams" ? "teams" : "projects"} and apps
              </CardDescription>
            </div>
            <Select value={viewMode} onValueChange={(v) => setViewMode(v as "teams" | "projects")}>
              <SelectTrigger className="w-[140px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="teams">By Team</SelectItem>
                <SelectItem value="projects">By Project</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead className="text-right">CPU</TableHead>
                <TableHead className="text-right">Memory</TableHead>
                <TableHead className="text-right">Disk</TableHead>
                <TableHead className="text-right">Total</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {viewMode === "teams" ? (
                teams.length > 0 ? (
                  teams.map((team) => (
                    <TeamCostRow
                      key={team.id}
                      team={team}
                      period={period}
                      isExpanded={expandedTeams.has(team.id)}
                      onToggle={() => toggleTeam(team.id)}
                    />
                  ))
                ) : (
                  <TableRow>
                    <TableCell colSpan={5} className="text-center py-8 text-muted-foreground">
                      <Building2 className="mx-auto h-8 w-8 text-muted-foreground/50 mb-2" />
                      No teams found. Create a team to organize your resources.
                    </TableCell>
                  </TableRow>
                )
              ) : (
                projects.length > 0 ? (
                  projects.map((project) => (
                    <ProjectCostRow
                      key={project.id}
                      project={project}
                      period={period}
                      isExpanded={expandedProjects.has(project.id)}
                      onToggle={() => toggleProject(project.id)}
                    />
                  ))
                ) : (
                  <TableRow>
                    <TableCell colSpan={5} className="text-center py-8 text-muted-foreground">
                      <FolderOpen className="mx-auto h-8 w-8 text-muted-foreground/50 mb-2" />
                      No projects found. Create a project to organize your apps.
                    </TableCell>
                  </TableRow>
                )
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      {/* Top Apps */}
      {dashboardCosts?.top_apps && dashboardCosts.top_apps.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Top Apps by Cost</CardTitle>
            <CardDescription>
              Applications with the highest costs in the last {periodLabel}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>App</TableHead>
                  <TableHead className="text-right">CPU</TableHead>
                  <TableHead className="text-right">Memory</TableHead>
                  <TableHead className="text-right">Disk</TableHead>
                  <TableHead className="text-right">Total</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {dashboardCosts.top_apps.map((app, index) => (
                  <TableRow key={app.app_id}>
                    <TableCell>
                      <Link
                        to={`/apps/${app.app_id}`}
                        className="flex items-center gap-2 hover:text-primary"
                      >
                        <Badge variant="outline" className="w-6 h-6 flex items-center justify-center p-0 text-xs">
                          {index + 1}
                        </Badge>
                        <Box className="h-4 w-4 text-muted-foreground" />
                        <span className="font-medium">{app.app_name}</span>
                      </Link>
                    </TableCell>
                    <TableCell className="text-right">
                      {formatCurrency(app.cpu_cost)}
                    </TableCell>
                    <TableCell className="text-right">
                      {formatCurrency(app.memory_cost)}
                    </TableCell>
                    <TableCell className="text-right">
                      {formatCurrency(app.disk_cost)}
                    </TableCell>
                    <TableCell className="text-right font-medium">
                      {formatCurrency(app.total_cost)}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
