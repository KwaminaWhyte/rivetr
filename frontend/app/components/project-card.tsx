import { Link } from "react-router";
import { Settings, ExternalLink, ArrowRight, Database } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { App, ManagedDatabase, ProjectWithApps, DeploymentStatus } from "@/types/api";

interface ProjectCardProps {
  project: ProjectWithApps;
  appStatuses?: Record<string, DeploymentStatus>;
  databaseStatuses?: Record<string, string>;
}

function getStatusColor(status?: DeploymentStatus): string {
  if (!status) return "bg-gray-400";

  switch (status) {
    case "running":
      return "bg-green-500";
    case "building":
    case "cloning":
    case "starting":
    case "checking":
    case "pending":
      return "bg-yellow-500";
    case "failed":
    case "stopped":
      return "bg-red-500";
    default:
      return "bg-gray-400";
  }
}

function StatusDot({ status }: { status?: DeploymentStatus | string }) {
  const color = typeof status === "string" ? getDbStatusColor(status) : getStatusColor(status);
  return (
    <span
      className={`inline-block w-2 h-2 rounded-full ${color}`}
      title={status || "unknown"}
    />
  );
}

function getDbStatusColor(status?: string): string {
  if (!status) return "bg-gray-400";
  switch (status) {
    case "running":
      return "bg-green-500";
    case "starting":
    case "pulling":
    case "pending":
      return "bg-yellow-500";
    case "failed":
    case "stopped":
      return "bg-red-500";
    default:
      return "bg-gray-400";
  }
}

export function ProjectCard({ project, appStatuses = {}, databaseStatuses = {} }: ProjectCardProps) {
  const databases = project.databases || [];
  const appCount = project.apps.length;
  const dbCount = databases.length;
  const serviceCount = appCount + dbCount;

  return (
    <Card className="group hover:shadow-md transition-shadow">
      <CardHeader className="flex flex-row items-start justify-between space-y-0 pb-2">
        <div className="space-y-1">
          <CardTitle className="text-lg font-semibold">{project.name}</CardTitle>
          {project.description && (
            <p className="text-sm text-muted-foreground line-clamp-2">
              {project.description}
            </p>
          )}
        </div>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8 opacity-0 group-hover:opacity-100 transition-opacity"
          asChild
        >
          <Link to={`/projects/${project.id}`} title="Project Settings">
            <Settings className="h-4 w-4" />
          </Link>
        </Button>
      </CardHeader>
      <CardContent className="space-y-3">
        {/* Services list (apps + databases) */}
        <div className="space-y-2">
          {serviceCount === 0 ? (
            <p className="text-sm text-muted-foreground italic">
              No services in this project
            </p>
          ) : (
            <>
              {/* Apps */}
              {project.apps.slice(0, 3).map((app) => (
                <AppListItem
                  key={app.id}
                  app={app}
                  status={appStatuses[app.id]}
                />
              ))}
              {/* Databases */}
              {databases.slice(0, Math.max(0, 3 - project.apps.length)).map((db) => (
                <DatabaseListItem
                  key={db.id}
                  database={db}
                  status={databaseStatuses[db.id] || db.status}
                />
              ))}
            </>
          )}
          {serviceCount > 3 && (
            <p className="text-xs text-muted-foreground">
              +{serviceCount - 3} more {serviceCount - 3 === 1 ? "service" : "services"}
            </p>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between pt-2 border-t">
          <span className="text-sm text-muted-foreground">
            {appCount > 0 && `${appCount} app${appCount !== 1 ? "s" : ""}`}
            {appCount > 0 && dbCount > 0 && ", "}
            {dbCount > 0 && `${dbCount} db${dbCount !== 1 ? "s" : ""}`}
            {serviceCount === 0 && "No services"}
          </span>
          <Link
            to={`/projects/${project.id}`}
            className="inline-flex items-center text-sm font-medium text-primary hover:underline"
          >
            View Project
            <ArrowRight className="ml-1 h-4 w-4" />
          </Link>
        </div>
      </CardContent>
    </Card>
  );
}

interface AppListItemProps {
  app: App;
  status?: DeploymentStatus;
}

function AppListItem({ app, status }: AppListItemProps) {
  return (
    <div className="flex items-center justify-between py-1">
      <div className="flex items-center gap-2 min-w-0">
        <StatusDot status={status} />
        <span className="text-sm font-medium truncate">{app.name}</span>
        <Badge variant="outline" className="text-xs shrink-0">
          APP
        </Badge>
      </div>
      <Button
        variant="ghost"
        size="icon"
        className="h-6 w-6 shrink-0"
        asChild
      >
        <Link to={`/apps/${app.id}`} title="View App">
          <ExternalLink className="h-3 w-3" />
        </Link>
      </Button>
    </div>
  );
}

interface DatabaseListItemProps {
  database: ManagedDatabase;
  status?: string;
}

function DatabaseListItem({ database, status }: DatabaseListItemProps) {
  return (
    <div className="flex items-center justify-between py-1">
      <div className="flex items-center gap-2 min-w-0">
        <StatusDot status={status} />
        <span className="text-sm font-medium truncate">{database.name}</span>
        <Badge variant="outline" className="text-xs shrink-0 bg-blue-50 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400">
          {database.db_type.toUpperCase()}
        </Badge>
      </div>
      <Button
        variant="ghost"
        size="icon"
        className="h-6 w-6 shrink-0"
        asChild
      >
        <Link to={`/databases/${database.id}`} title="View Database">
          <Database className="h-3 w-3" />
        </Link>
      </Button>
    </div>
  );
}
