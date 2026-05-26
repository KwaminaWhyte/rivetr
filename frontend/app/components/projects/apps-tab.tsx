import { useState, useMemo } from "react";
import { Link } from "react-router";
import { useQuery, useQueryClient, useMutation } from "@tanstack/react-query";
import { toast } from "sonner";
import { bulkApi } from "@/lib/api/bulk";
import {
  ExternalLink,
  Play,
  Plus,
  RotateCw,
  Rocket,
  Settings2,
  Square,
  X,
} from "lucide-react";
import { api } from "@/lib/api";
import { getPrimaryDomain } from "@/lib/utils";
import { useTeamContext } from "@/lib/team-context";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Checkbox } from "@/components/ui/checkbox";
import { EnvironmentBadge } from "@/components/environment-badge";
import { Badge } from "@/components/ui/badge";
import type { App, ProjectWithApps, ProjectEnvironment } from "@/types/api";

interface AppsTabProps {
  project: ProjectWithApps;
  projectId: string;
}

function StatusBadge({ status }: { status: string }) {
  const variants: Record<string, { className: string; label: string }> = {
    running: { className: "bg-green-500 text-white", label: "Running" },
    stopped: { className: "bg-gray-500 text-white", label: "Stopped" },
    not_deployed: {
      className: "bg-gray-400 text-white",
      label: "Not Deployed",
    },
    failed: { className: "bg-red-500 text-white", label: "Failed" },
    building: { className: "bg-blue-500 text-white", label: "Building" },
    pending: { className: "bg-yellow-500 text-white", label: "Pending" },
  };
  const variant = variants[status] || variants.stopped;
  return <Badge className={variant.className}>{variant.label}</Badge>;
}

export function AppsTab({ project, projectId }: AppsTabProps) {
  const queryClient = useQueryClient();
  const { currentTeamId } = useTeamContext();
  const [isAddAppDialogOpen, setIsAddAppDialogOpen] = useState(false);
  const [selectedEnvironmentId, setSelectedEnvironmentId] = useState<string>("all");
  const [selectedAppIds, setSelectedAppIds] = useState<Set<string>>(new Set());
  const [isBulkLoading, setIsBulkLoading] = useState(false);

  const { data: environments = [] } = useQuery<ProjectEnvironment[]>({
    queryKey: ["environments", projectId],
    queryFn: () => api.getEnvironments(projectId),
    enabled: !!projectId,
  });

  const { data: allApps = [] } = useQuery<App[]>({
    queryKey: ["apps", currentTeamId],
    queryFn: () => api.getApps({ teamId: currentTeamId ?? undefined }),
    enabled: currentTeamId !== null,
  });

  const { data: appStatuses = {} } = useQuery<Record<string, string>>({
    queryKey: ["app-statuses", project?.apps?.map((a) => a.id)],
    queryFn: async () => {
      if (!project?.apps) return {};
      const statuses: Record<string, string> = {};
      await Promise.all(
        project.apps.map(async (app) => {
          try {
            const status = await api.getAppStatus(app.id);
            statuses[app.id] = status.status;
          } catch {
            statuses[app.id] = "stopped";
          }
        })
      );
      return statuses;
    },
    enabled: !!project?.apps?.length,
  });

  const filteredApps = useMemo(() => {
    if (selectedEnvironmentId === "all") return project.apps;
    return project.apps.filter(
      (app) => app.environment_id === selectedEnvironmentId
    );
  }, [project, selectedEnvironmentId]);

  const availableApps = useMemo(() => {
    const projectAppIds = new Set(project.apps.map((a) => a.id));
    return allApps.filter(
      (app) => !app.project_id && !projectAppIds.has(app.id)
    );
  }, [allApps, project]);

  const assignAppMutation = useMutation({
    mutationFn: (appId: string) => api.assignAppToProject(appId, projectId),
    onSuccess: () => {
      toast.success("App added to project");
      setIsAddAppDialogOpen(false);
      queryClient.invalidateQueries({ queryKey: ["project", projectId] });
      queryClient.invalidateQueries({ queryKey: ["apps"] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  const removeAppMutation = useMutation({
    mutationFn: (appId: string) => api.assignAppToProject(appId, null),
    onSuccess: () => {
      toast.success("App removed from project");
      queryClient.invalidateQueries({ queryKey: ["project", projectId] });
      queryClient.invalidateQueries({ queryKey: ["apps"] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  const toggleAppSelection = (appId: string) => {
    setSelectedAppIds((prev) => {
      const next = new Set(prev);
      if (next.has(appId)) next.delete(appId);
      else next.add(appId);
      return next;
    });
  };

  const toggleSelectAll = () => {
    if (selectedAppIds.size === filteredApps.length) {
      setSelectedAppIds(new Set());
    } else {
      setSelectedAppIds(new Set(filteredApps.map((a) => a.id)));
    }
  };

  const handleBulkAction = async (action: "start" | "stop" | "restart" | "deploy") => {
    if (selectedAppIds.size === 0) return;
    setIsBulkLoading(true);
    try {
      const appIds = Array.from(selectedAppIds);
      let result;
      if (action === "start") result = await bulkApi.bulkStart({ app_ids: appIds });
      else if (action === "stop") result = await bulkApi.bulkStop({ app_ids: appIds });
      else if (action === "restart") result = await bulkApi.bulkRestart({ app_ids: appIds });
      else result = await bulkApi.bulkDeploy({ app_ids: appIds });

      const failed = result.results.filter((r) => !r.success);
      if (failed.length === 0) {
        toast.success(`Bulk ${action} completed for ${appIds.length} app(s)`);
      } else {
        toast.warning(
          `Bulk ${action}: ${appIds.length - failed.length} succeeded, ${failed.length} failed`
        );
      }
      setSelectedAppIds(new Set());
      queryClient.invalidateQueries({ queryKey: ["app-statuses"] });
      queryClient.invalidateQueries({ queryKey: ["project", projectId] });
    } catch (err) {
      toast.error(err instanceof Error ? err.message : `Bulk ${action} failed`);
    } finally {
      setIsBulkLoading(false);
    }
  };

  return (
    <>
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <div className="flex items-center gap-4">
            <CardTitle>Applications</CardTitle>
            {environments.length > 0 && (
              <Select
                value={selectedEnvironmentId}
                onValueChange={setSelectedEnvironmentId}
              >
                <SelectTrigger className="w-[180px] h-8">
                  <SelectValue placeholder="All environments" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">All Environments</SelectItem>
                  {environments.map((env) => (
                    <SelectItem key={env.id} value={env.id}>
                      {env.name}
                      {env.is_default && " (default)"}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
          </div>
          <div className="flex gap-2">
            <Button variant="outline" size="sm" asChild>
              <Link to={`/projects/${project.id}/environments`}>
                <Settings2 className="mr-2 h-4 w-4" />
                Environments
              </Link>
            </Button>
            <Button variant="outline" size="sm" asChild>
              <Link to={`/projects/${project.id}/env-vars`}>
                <Settings2 className="mr-2 h-4 w-4" />
                Shared Variables
              </Link>
            </Button>
            <Button asChild>
              <Link to={`/projects/${project.id}/apps/new`}>
                <Plus className="mr-2 h-4 w-4" />
                Create New App
              </Link>
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {/* Bulk action bar */}
          {selectedAppIds.size > 0 && (
            <div className="mb-4 flex items-center gap-2 rounded-md border bg-muted/50 px-3 py-2">
              <span className="text-sm font-medium mr-2">
                {selectedAppIds.size} app{selectedAppIds.size !== 1 ? "s" : ""} selected
              </span>
              <Button
                size="sm"
                variant="outline"
                disabled={isBulkLoading}
                onClick={() => handleBulkAction("start")}
                className="gap-1.5"
              >
                <Play className="h-3.5 w-3.5" />
                Start
              </Button>
              <Button
                size="sm"
                variant="outline"
                disabled={isBulkLoading}
                onClick={() => handleBulkAction("stop")}
                className="gap-1.5"
              >
                <Square className="h-3.5 w-3.5" />
                Stop
              </Button>
              <Button
                size="sm"
                variant="outline"
                disabled={isBulkLoading}
                onClick={() => handleBulkAction("restart")}
                className="gap-1.5"
              >
                <RotateCw className="h-3.5 w-3.5" />
                Restart
              </Button>
              <Button
                size="sm"
                variant="outline"
                disabled={isBulkLoading}
                onClick={() => handleBulkAction("deploy")}
                className="gap-1.5"
              >
                <Rocket className="h-3.5 w-3.5" />
                Deploy
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => setSelectedAppIds(new Set())}
                className="ml-auto"
              >
                <X className="h-3.5 w-3.5 mr-1" />
                Clear
              </Button>
            </div>
          )}
          {filteredApps.length === 0 ? (
            <div className="py-8 text-center">
              <p className="text-muted-foreground">
                No applications in this project yet. Use the{" "}
                <span className="font-medium">Create New App</span> button
                above to add one.
              </p>
            </div>
          ) : (
            <>
              {/* Select All checkbox row */}
              <div className="flex items-center gap-2 mb-3">
                <Checkbox
                  id="select-all-apps"
                  checked={
                    filteredApps.length > 0 &&
                    selectedAppIds.size === filteredApps.length
                  }
                  onCheckedChange={toggleSelectAll}
                />
                <label
                  htmlFor="select-all-apps"
                  className="text-sm text-muted-foreground cursor-pointer select-none"
                >
                  Select all ({filteredApps.length})
                </label>
              </div>
              <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                {filteredApps.map((app) => {
                  const status = appStatuses?.[app.id] || "stopped";
                  const isSelected = selectedAppIds.has(app.id);
                  return (
                    <Card
                      key={app.id}
                      className={`group relative hover:shadow-md transition-shadow ${isSelected ? "ring-2 ring-primary" : ""}`}
                    >
                      <div
                        className="absolute top-3 left-3 z-10"
                        onClick={(e) => e.stopPropagation()}
                      >
                        <Checkbox
                          checked={isSelected}
                          onCheckedChange={() => toggleAppSelection(app.id)}
                        />
                      </div>
                      <Link
                        to={`/apps/${app.id}`}
                        className="absolute inset-0 z-0"
                      />
                      <CardHeader className="pb-2 pl-10">
                        <div className="flex items-start justify-between">
                          <div className="space-y-1">
                            <CardTitle className="text-base font-semibold">
                              {app.name}
                            </CardTitle>
                            <div className="flex items-center gap-2">
                              <StatusBadge status={status} />
                              <EnvironmentBadge environment={app.environment} />
                            </div>
                          </div>
                          <Button
                            type="button"
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8 opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-destructive transition-opacity relative z-10"
                            disabled={removeAppMutation.isPending}
                            title="Remove from project"
                            onClick={(e) => {
                              e.preventDefault();
                              removeAppMutation.mutate(app.id);
                            }}
                          >
                            <X className="h-4 w-4" />
                          </Button>
                        </div>
                      </CardHeader>
                      <CardContent className="pt-0 pb-4">
                        <div className="space-y-2 text-sm text-muted-foreground">
                          {getPrimaryDomain(app) && (
                            <div className="flex items-center gap-2 truncate">
                              <ExternalLink className="h-3 w-3 flex-shrink-0" />
                              <span className="truncate">{getPrimaryDomain(app)}</span>
                            </div>
                          )}
                          {app.git_url && (
                            <div className="truncate text-xs opacity-75">
                              {app.git_url
                                .replace(/^https?:\/\//, "")
                                .replace(/\.git$/, "")}
                            </div>
                          )}
                        </div>
                      </CardContent>
                    </Card>
                  );
                })}
              </div>
            </>
          )}
        </CardContent>
      </Card>

      {/* Add App Dialog */}
      <Dialog open={isAddAppDialogOpen} onOpenChange={setIsAddAppDialogOpen}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Add App to Project</DialogTitle>
            <DialogDescription>
              Select an unassigned app to add to this project.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4">
            {availableApps.length === 0 ? (
              <div className="text-center py-8">
                <p className="text-muted-foreground mb-4">
                  No unassigned apps available.
                </p>
                <Button asChild>
                  <Link to={`/projects/${project.id}/apps/new`}>
                    Create New App
                  </Link>
                </Button>
              </div>
            ) : (
              <div className="space-y-2 max-h-80 overflow-y-auto">
                {availableApps.map((app) => (
                  <button
                    key={app.id}
                    type="button"
                    className="flex items-center justify-between w-full p-3 rounded-lg border hover:bg-muted/50 cursor-pointer text-left"
                    disabled={assignAppMutation.isPending}
                    onClick={() => assignAppMutation.mutate(app.id)}
                  >
                    <div className="flex items-center gap-3">
                      <div>
                        <p className="font-medium">{app.name}</p>
                        <p className="text-sm text-muted-foreground truncate max-w-md">
                          {app.git_url}
                        </p>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <EnvironmentBadge environment={app.environment} />
                      <Plus className="h-4 w-4" />
                    </div>
                  </button>
                ))}
              </div>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setIsAddAppDialogOpen(false)}>
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
