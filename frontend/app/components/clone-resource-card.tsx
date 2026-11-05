import { useEffect, useState } from "react";
import { Link } from "react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { CheckCircle2, CopyPlus, Loader2, XCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { api, appConfigApi } from "@/lib/api";
import type { Project, ProjectEnvironment } from "@/types/apps";
import type { CloneableResource, CloneResourceResponse } from "@/types/app-config";

const NONE = "__none__";
const LOCAL = "__local__";

interface CloneResourceCardProps {
  resourceKind: CloneableResource;
  resourceId: string;
  resourceName: string;
  currentProjectId: string | null;
  /** Apps only */
  currentEnvironmentId?: string | null;
  /** Apps only; null means this Rivetr host */
  currentServerId?: string | null;
}

const COPIED: Record<CloneableResource, string> = {
  app: "Copies the configuration, environment variables, healthcheck and volume definitions. Domains are not copied, so the clone never takes over the original's traffic.",
  database: "Copies the database type, version, image, credentials, init commands and limits. Public access is turned off on the clone.",
  service: "Copies the compose file, environment variables and limits. The domain is not copied.",
};

const PATHS: Record<CloneableResource, string> = {
  app: "apps",
  database: "databases",
  service: "services",
};

/**
 * "Clone resource" card: create a copy in another project, environment or server,
 * optionally copying volume data in the background.
 */
export function CloneResourceCard({
  resourceKind,
  resourceId,
  resourceName,
  currentProjectId,
  currentEnvironmentId = null,
  currentServerId = null,
}: CloneResourceCardProps) {
  const queryClient = useQueryClient();
  const isApp = resourceKind === "app";
  const [name, setName] = useState(`${resourceName}-clone`.slice(0, 63));
  const [projectId, setProjectId] = useState<string>(currentProjectId ?? NONE);
  const [environmentId, setEnvironmentId] = useState<string>(currentEnvironmentId ?? NONE);
  const [serverId, setServerId] = useState<string>(currentServerId ?? LOCAL);
  const [cloneVolumes, setCloneVolumes] = useState(false);
  const [result, setResult] = useState<CloneResourceResponse | null>(null);

  const { data: projects = [] } = useQuery<Project[]>({
    queryKey: ["projects"],
    queryFn: () => api.getProjects(),
  });

  const { data: environments = [] } = useQuery<ProjectEnvironment[]>({
    queryKey: ["environments", projectId],
    queryFn: () => api.getEnvironments(projectId),
    enabled: isApp && projectId !== NONE,
  });

  const { data: servers = [] } = useQuery({
    queryKey: ["servers"],
    queryFn: () => api.getServers(),
    enabled: isApp,
  });

  // Keep the environment choice valid for the selected project.
  useEffect(() => {
    if (!isApp) return;
    if (projectId === NONE) {
      setEnvironmentId(NONE);
      return;
    }
    if (environments.length > 0 && !environments.some((e) => e.id === environmentId)) {
      const preferred =
        environments.find((e) => e.id === currentEnvironmentId) ??
        environments.find((e) => e.is_default) ??
        environments[0];
      setEnvironmentId(preferred.id);
    }
  }, [isApp, projectId, environments, environmentId, currentEnvironmentId]);

  const remoteServer = isApp && (serverId !== LOCAL || currentServerId !== null);
  useEffect(() => {
    if (remoteServer) setCloneVolumes(false);
  }, [remoteServer]);

  const job = useQuery({
    queryKey: ["clone-job", result?.job.id],
    queryFn: () => appConfigApi.getCloneJob(result!.job.id),
    enabled: Boolean(result),
    initialData: result?.job,
    refetchInterval: (q) => {
      const status = q.state.data?.status;
      return status === "completed" || status === "failed" ? false : 1500;
    },
  });

  const clone = useMutation({
    mutationFn: () =>
      appConfigApi.cloneResource(resourceKind, resourceId, {
        name: name.trim() || undefined,
        project_id: projectId === NONE ? "" : projectId,
        ...(isApp
          ? {
              environment_id: environmentId === NONE ? "" : environmentId,
              server_id: serverId === LOCAL ? "" : serverId,
            }
          : {}),
        clone_volume_data: cloneVolumes,
      }),
    onSuccess: (res) => {
      setResult(res);
      toast.success(`Created ${res.name}`);
      queryClient.invalidateQueries({ queryKey: [`${resourceKind}s`] });
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      queryClient.invalidateQueries({ queryKey: ["project"] });
    },
    onError: (e: Error) => toast.error(e.message || "Clone failed"),
  });

  const status = job.data?.status;
  const done = status === "completed" || status === "failed";
  const nameValid = /^[a-z0-9][a-z0-9-]{0,62}$/.test(name.trim());

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <CopyPlus className="h-5 w-5" />
          Clone resource
        </CardTitle>
        <CardDescription>
          {COPIED[resourceKind]} Stored files, database records, and other persistent data are
          copied only when you select Clone volume data.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        <div className="grid gap-4 md:grid-cols-2">
          <div className="space-y-2">
            <Label htmlFor={`clone-name-${resourceId}`}>Name</Label>
            <Input
              id={`clone-name-${resourceId}`}
              value={name}
              onChange={(e) => setName(e.target.value)}
              aria-invalid={!nameValid}
            />
            <p className="text-xs text-muted-foreground">
              Lowercase letters, numbers and dashes. A number is added if the name is taken.
            </p>
          </div>
          {isApp && (
            <div className="space-y-2">
              <Label htmlFor={`clone-server-${resourceId}`}>Server</Label>
              <Select value={serverId} onValueChange={setServerId}>
                <SelectTrigger id={`clone-server-${resourceId}`}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={LOCAL}>This server (localhost)</SelectItem>
                  {servers.map((s) => (
                    <SelectItem key={s.id} value={s.id}>
                      {s.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}
          <div className="space-y-2">
            <Label htmlFor={`clone-project-${resourceId}`}>Project</Label>
            <Select value={projectId} onValueChange={setProjectId}>
              <SelectTrigger id={`clone-project-${resourceId}`}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={NONE}>No project</SelectItem>
                {projects.map((p) => (
                  <SelectItem key={p.id} value={p.id}>
                    {p.name}
                    {p.id === currentProjectId ? " (current)" : ""}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          {isApp && (
            <div className="space-y-2">
              <Label htmlFor={`clone-env-${resourceId}`}>Environment</Label>
              <Select
                value={environmentId}
                onValueChange={setEnvironmentId}
                disabled={projectId === NONE || environments.length === 0}
              >
                <SelectTrigger id={`clone-env-${resourceId}`}>
                  <SelectValue placeholder="No environment" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={NONE}>No environment</SelectItem>
                  {environments.map((e) => (
                    <SelectItem key={e.id} value={e.id}>
                      {e.name}
                      {e.id === currentEnvironmentId ? " (current)" : ""}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}
        </div>

        <div className="flex items-start gap-3 rounded-lg border p-4">
          <Checkbox
            id={`clone-volumes-${resourceId}`}
            checked={cloneVolumes}
            onCheckedChange={(checked) => setCloneVolumes(checked === true)}
            disabled={remoteServer}
          />
          <div className="space-y-1">
            <Label htmlFor={`clone-volumes-${resourceId}`}>Clone volume data</Label>
            <p className="text-sm text-muted-foreground">
              {remoteServer
                ? "Volume data can only be copied between resources on this Rivetr host."
                : "Copies the data in every volume with a short-lived helper container. Stop the resource first for a consistent copy of a busy database."}
            </p>
          </div>
        </div>

        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <p className="text-sm text-muted-foreground">The running resource will not be changed.</p>
          <Button
            onClick={() => clone.mutate()}
            disabled={!nameValid || clone.isPending || (Boolean(result) && !done)}
            className="gap-2"
          >
            {clone.isPending && <Loader2 className="h-4 w-4 animate-spin" />}
            Clone Resource
          </Button>
        </div>

        {result && job.data && (
          <div className="space-y-3 rounded-lg border bg-muted/30 p-4" aria-live="polite">
            <div className="flex items-center justify-between gap-2">
              <div className="flex items-center gap-2 text-sm font-medium">
                {status === "completed" ? (
                  <CheckCircle2 className="h-4 w-4 text-green-600" />
                ) : status === "failed" ? (
                  <XCircle className="h-4 w-4 text-destructive" />
                ) : (
                  <Loader2 className="h-4 w-4 animate-spin" />
                )}
                {status === "completed"
                  ? `${result.name} is ready`
                  : status === "failed"
                    ? `${result.name} was created, but copying data failed`
                    : `Copying data into ${result.name}...`}
              </div>
              <Button variant="outline" size="sm" asChild>
                <Link to={`/${PATHS[resourceKind]}/${result.resource_id}`}>Open clone</Link>
              </Button>
            </div>
            <div
              className="h-2 w-full overflow-hidden rounded-full bg-muted"
              role="progressbar"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={job.data.progress}
            >
              <div
                className={`h-full transition-all ${status === "failed" ? "bg-destructive" : "bg-primary"}`}
                style={{ width: `${job.data.progress}%` }}
              />
            </div>
            {job.data.log && (
              <pre className="max-h-48 overflow-auto whitespace-pre-wrap break-all rounded bg-background p-3 font-mono text-xs">
                {job.data.log.trim()}
              </pre>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
