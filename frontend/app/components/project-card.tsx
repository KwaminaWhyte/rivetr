import { Link } from "react-router";
import { ArrowRight, Box, Database, Layers } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { StatusDot, StatusPill, type Tone } from "@/components/metrics";
import type { DeploymentStatus, ProjectWithApps } from "@/types/api";

export type ProjectHealth = "healthy" | "issues" | "building";

interface ProjectCardProps {
  project: ProjectWithApps;
  health: ProjectHealth;
  appStatuses?: Record<string, DeploymentStatus>;
  databaseStatuses?: Record<string, string>;
}

/** Map any app, database or service status string onto a tone. */
function toneForStatus(status?: string): Tone {
  switch (status) {
    case "running":
      return "ok";
    case "building":
    case "cloning":
    case "starting":
    case "checking":
    case "pending":
    case "pulling":
      return "warn";
    case "failed":
    case "stopped":
      return "bad";
    default:
      return "neutral";
  }
}

const HEALTH_PILL: Record<ProjectHealth, { tone: Tone; label: string }> = {
  healthy: { tone: "ok", label: "Healthy" },
  issues: { tone: "bad", label: "Needs attention" },
  building: { tone: "warn", label: "Building" },
};

const MAX_ROWS = 4;

interface ResourceRow {
  id: string;
  name: string;
  kind: string;
  to: string;
  status?: string;
  icon: typeof Box;
}

export function ProjectCard({
  project,
  health,
  appStatuses = {},
  databaseStatuses = {},
}: ProjectCardProps) {
  const databases = project.databases || [];
  const services = project.services || [];
  const appCount = project.apps.length;
  const dbCount = databases.length;
  const svcCount = services.length;
  const totalCount = appCount + dbCount + svcCount;

  // Unhealthy resources float to the top so a problem is never hidden behind "+N more".
  const rows: ResourceRow[] = [
    ...project.apps.map((app) => ({
      id: app.id,
      name: app.name,
      kind: "App",
      to: `/apps/${app.id}`,
      status: appStatuses[app.id],
      icon: Box,
    })),
    ...databases.map((db) => ({
      id: db.id,
      name: db.name,
      kind: db.db_type.toUpperCase(),
      to: `/databases/${db.id}`,
      status: databaseStatuses[db.id] || db.status,
      icon: Database,
    })),
    ...services.map((svc) => ({
      id: svc.id,
      name: svc.name,
      kind: "Service",
      to: `/services/${svc.id}`,
      status: svc.status,
      icon: Layers,
    })),
  ];
  const severity = (r: ResourceRow) =>
    ({ bad: 0, warn: 1, neutral: 2, ok: 3 })[toneForStatus(r.status)];
  const sorted = [...rows].sort((a, b) => severity(a) - severity(b));
  const visible = sorted.slice(0, MAX_ROWS);
  const hidden = totalCount - visible.length;

  const counts = [
    appCount > 0 && `${appCount} app${appCount !== 1 ? "s" : ""}`,
    dbCount > 0 && `${dbCount} database${dbCount !== 1 ? "s" : ""}`,
    svcCount > 0 && `${svcCount} service${svcCount !== 1 ? "s" : ""}`,
  ].filter(Boolean);

  const pill =
    totalCount === 0 ? { tone: "neutral" as Tone, label: "Empty" } : HEALTH_PILL[health];

  return (
    <Card className="relative flex h-full flex-col gap-0 py-0 transition-colors hover:bg-muted/20">
      <CardHeader className="flex flex-row items-start justify-between gap-3 space-y-0 px-4 pb-3 pt-4">
        <div className="min-w-0 space-y-1">
          <CardTitle className="truncate text-base font-semibold">
            <Link
              to={`/projects/${project.id}`}
              className="outline-none after:absolute after:inset-0 hover:underline focus-visible:underline"
            >
              {project.name}
            </Link>
          </CardTitle>
          {project.description && (
            <p className="line-clamp-2 text-sm text-muted-foreground">{project.description}</p>
          )}
        </div>
        <StatusPill tone={pill.tone} className="shrink-0">
          {pill.label}
        </StatusPill>
      </CardHeader>

      <CardContent className="flex flex-1 flex-col px-4 pb-4">
        {totalCount === 0 ? (
          <p className="flex-1 py-2 text-sm text-muted-foreground">
            Nothing deployed yet. Open the project to add an app, database or service.
          </p>
        ) : (
          <ul className="flex-1 divide-y divide-border/60">
            {visible.map((row) => (
              <li key={`${row.kind}-${row.id}`}>
                <Link
                  to={row.to}
                  className="relative z-10 -mx-2 flex items-center gap-2.5 rounded-md px-2 py-2 text-sm outline-none transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring"
                >
                  <StatusDot tone={toneForStatus(row.status)} label={row.status || "unknown"} />
                  <row.icon
                    className="h-3.5 w-3.5 shrink-0 text-muted-foreground"
                    aria-hidden="true"
                  />
                  <span className="min-w-0 flex-1 truncate font-medium">{row.name}</span>
                  <span className="shrink-0 font-mono text-xs text-muted-foreground">
                    {row.kind}
                  </span>
                </Link>
              </li>
            ))}
          </ul>
        )}

        <div className="mt-3 flex items-center justify-between border-t pt-3 text-sm text-muted-foreground">
          <span>
            {hidden > 0
              ? `${counts.join(", ")}, +${hidden} not shown`
              : counts.join(", ") || "No resources"}
          </span>
          <span className="inline-flex items-center gap-1 font-medium text-foreground">
            Open
            <ArrowRight className="h-3.5 w-3.5" aria-hidden="true" />
          </span>
        </div>
      </CardContent>
    </Card>
  );
}
