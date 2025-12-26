import { useState, useMemo } from "react";
import { Link } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
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
import { Skeleton } from "@/components/ui/skeleton";
import { EnvironmentBadge } from "@/components/EnvironmentBadge";
import { api } from "@/lib/api";
import type { App, AppEnvironment } from "@/types/api";

type EnvironmentFilter = AppEnvironment | "all";

export function AppsPage() {
  const [environmentFilter, setEnvironmentFilter] =
    useState<EnvironmentFilter>("all");

  const {
    data: apps = [],
    isLoading,
    error,
  } = useQuery<App[]>({
    queryKey: ["apps"],
    queryFn: () => api.getApps(),
  });

  // Filter apps by environment
  const filteredApps = useMemo(() => {
    if (environmentFilter === "all") {
      return apps;
    }
    return apps.filter((app) => app.environment === environmentFilter);
  }, [apps, environmentFilter]);

  if (error) {
    return (
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <h1 className="text-3xl font-bold">Applications</h1>
        </div>
        <Card>
          <CardContent className="py-8 text-center text-destructive">
            Failed to load applications. Please try again.
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">Applications</h1>
        <Button asChild>
          <Link to="/apps/new">New App</Link>
        </Button>
      </div>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0">
          <CardTitle>All Applications</CardTitle>
          <div className="flex items-center gap-2">
            <span className="text-sm text-muted-foreground">Environment:</span>
            <Select
              value={environmentFilter}
              onValueChange={(value) =>
                setEnvironmentFilter(value as EnvironmentFilter)
              }
            >
              <SelectTrigger className="w-[140px]">
                <SelectValue placeholder="Filter by environment" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">All</SelectItem>
                <SelectItem value="development">Development</SelectItem>
                <SelectItem value="staging">Staging</SelectItem>
                <SelectItem value="production">Production</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-4">
              {[1, 2, 3].map((i) => (
                <div key={i} className="flex items-center gap-4">
                  <Skeleton className="h-12 w-full" />
                </div>
              ))}
            </div>
          ) : filteredApps.length === 0 ? (
            <div className="py-8 text-center">
              <p className="text-muted-foreground mb-4">
                {apps.length === 0
                  ? "No applications yet. Create your first app to get started."
                  : "No applications match the selected filter."}
              </p>
              {apps.length === 0 && (
                <Button asChild>
                  <Link to="/apps/new">Create Application</Link>
                </Button>
              )}
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Environment</TableHead>
                  <TableHead>Repository</TableHead>
                  <TableHead>Branch</TableHead>
                  <TableHead>Resources</TableHead>
                  <TableHead>Domain</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredApps.map((app) => (
                  <TableRow key={app.id}>
                    <TableCell className="font-medium">{app.name}</TableCell>
                    <TableCell>
                      <EnvironmentBadge environment={app.environment} />
                    </TableCell>
                    <TableCell className="text-muted-foreground max-w-xs truncate">
                      {app.git_url}
                    </TableCell>
                    <TableCell>{app.branch}</TableCell>
                    <TableCell>
                      <div className="flex gap-1 flex-wrap">
                        {app.cpu_limit && (
                          <Badge variant="outline" className="text-xs">
                            {app.cpu_limit} CPU
                          </Badge>
                        )}
                        {app.memory_limit && (
                          <Badge variant="outline" className="text-xs">
                            {app.memory_limit.toUpperCase()}
                          </Badge>
                        )}
                        {!app.cpu_limit && !app.memory_limit && (
                          <span className="text-muted-foreground text-xs">Default</span>
                        )}
                      </div>
                    </TableCell>
                    <TableCell>{app.domain || "-"}</TableCell>
                    <TableCell>
                      <Badge variant={app.domain ? "default" : "secondary"}>
                        {app.domain ? "Running" : "Stopped"}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-right">
                      <Button variant="ghost" size="sm" asChild>
                        <Link to={`/apps/${app.id}`}>View</Link>
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
