/**
 * Linked Databases section for the App env-vars tab.
 *
 * Lets the user link managed databases to the app so that DATABASE_URL,
 * REDIS_URL, MONGODB_URL (plus host/port/user/password/db) are auto-injected
 * into the app container at deploy time.  User-defined env vars take precedence.
 */

import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Database, Link2, Plus, Trash2 } from "lucide-react";
import { api } from "@/lib/api";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import type {
  DatabaseAppLink,
  LinkedEnvVarsForDatabase,
} from "@/lib/api/database-links";
import type { App, ManagedDatabase } from "@/types/api";

interface LinkedDatabasesSectionProps {
  appId: string;
  token?: string;
}

export function LinkedDatabasesSection({
  appId,
  token,
}: LinkedDatabasesSectionProps) {
  const qc = useQueryClient();
  const [showDialog, setShowDialog] = useState(false);
  const [selectedDbId, setSelectedDbId] = useState<string>("");
  const [prefix, setPrefix] = useState("");

  // Need the app to know its project so we can offer same-project DBs.
  const { data: app } = useQuery<App>({
    queryKey: ["app", appId],
    queryFn: () => api.getApp(appId, token),
  });

  const { data: links = [], isLoading: loadingLinks } = useQuery<
    DatabaseAppLink[]
  >({
    queryKey: ["database-links", appId],
    queryFn: () => api.listDatabaseLinks(appId, token),
  });

  const { data: previews = [] } = useQuery<LinkedEnvVarsForDatabase[]>({
    queryKey: ["linked-env-vars", appId],
    queryFn: () => api.previewLinkedEnvVars(appId, token),
  });

  const { data: allDatabases = [] } = useQuery<ManagedDatabase[]>({
    queryKey: ["databases-for-link", app?.team_id ?? ""],
    queryFn: () =>
      api.getDatabases(app?.team_id ? { teamId: app.team_id } : {}, token),
    enabled: !!app,
  });

  // Same-project candidate DBs not yet linked.
  const linkedIds = new Set(links.map((l) => l.database_id));
  const candidates = allDatabases.filter((db) => {
    if (linkedIds.has(db.id)) return false;
    // If the app belongs to a project, restrict to DBs in the same project.
    if (app?.project_id) return db.project_id === app.project_id;
    return true;
  });

  const createMutation = useMutation({
    mutationFn: () =>
      api.createDatabaseLink(
        appId,
        {
          database_id: selectedDbId,
          env_prefix: prefix.trim() || undefined,
        },
        token,
      ),
    onSuccess: () => {
      toast.success("Database linked", {
        description: "Redeploy the app for the change to take effect.",
      });
      qc.invalidateQueries({ queryKey: ["database-links", appId] });
      qc.invalidateQueries({ queryKey: ["linked-env-vars", appId] });
      setShowDialog(false);
      setSelectedDbId("");
      setPrefix("");
    },
    onError: (err: Error) => {
      toast.error(err.message || "Failed to link database");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (linkId: string) =>
      api.deleteDatabaseLink(appId, linkId, token),
    onSuccess: () => {
      toast.success("Link removed", {
        description: "Redeploy the app for the change to take effect.",
      });
      qc.invalidateQueries({ queryKey: ["database-links", appId] });
      qc.invalidateQueries({ queryKey: ["linked-env-vars", appId] });
    },
    onError: (err: Error) => {
      toast.error(err.message || "Failed to remove link");
    },
  });

  const previewByLinkId = new Map(previews.map((p) => [p.link_id, p]));

  return (
    <>
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Link2 className="h-4 w-4" />
                Linked Databases
              </CardTitle>
              <CardDescription>
                Connection details (DATABASE_URL, REDIS_URL, MONGODB_URL, plus
                host/port/user/password/db) are auto-injected into the app
                container at deploy time. User-defined env vars take precedence.
              </CardDescription>
            </div>
            <Button
              size="sm"
              onClick={() => setShowDialog(true)}
              disabled={candidates.length === 0}
              title={
                candidates.length === 0
                  ? "No unlinked databases available in this project"
                  : "Link a database"
              }
            >
              <Plus className="h-4 w-4 mr-1" />
              Link database
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {loadingLinks ? (
            <Skeleton className="h-16 w-full" />
          ) : links.length === 0 ? (
            <div className="text-center py-6 text-sm text-muted-foreground">
              No databases linked yet. Click <strong>Link database</strong> to
              auto-inject connection vars from a managed database.
            </div>
          ) : (
            <div className="space-y-3">
              {links.map((link) => {
                const preview = previewByLinkId.get(link.id);
                return (
                  <div
                    key={link.id}
                    className="border rounded-lg p-3 space-y-2"
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <Database className="h-4 w-4 text-muted-foreground" />
                        <span className="font-medium">
                          {link.database_name}
                        </span>
                        <Badge variant="outline" className="capitalize text-xs">
                          {link.database_type}
                        </Badge>
                        {link.env_prefix ? (
                          <Badge variant="secondary" className="text-xs">
                            prefix: {link.env_prefix}
                          </Badge>
                        ) : (
                          <Badge variant="secondary" className="text-xs">
                            no prefix
                          </Badge>
                        )}
                        <Badge
                          variant={
                            link.database_status === "running"
                              ? "default"
                              : "outline"
                          }
                          className="text-xs capitalize"
                        >
                          {link.database_status}
                        </Badge>
                      </div>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-7 w-7 p-0 text-red-500 hover:text-red-600"
                        title="Remove link"
                        disabled={deleteMutation.isPending}
                        onClick={() => deleteMutation.mutate(link.id)}
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                      </Button>
                    </div>
                    {preview && preview.vars.length > 0 && (
                      <div className="flex flex-wrap gap-1 pl-6">
                        {preview.vars.map((v) => (
                          <code
                            key={v.key}
                            className={`text-xs px-1.5 py-0.5 rounded ${
                              v.overridden
                                ? "bg-yellow-100 text-yellow-900 dark:bg-yellow-950/40 dark:text-yellow-300 line-through"
                                : "bg-muted text-muted-foreground"
                            }`}
                            title={
                              v.overridden
                                ? "Overridden by an existing app env var"
                                : "Will be injected at deploy time"
                            }
                          >
                            {v.key}
                          </code>
                        ))}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>

      <Dialog open={showDialog} onOpenChange={setShowDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Link database</DialogTitle>
            <DialogDescription>
              Auto-inject connection details from a managed database into this
              app's container env at deploy time.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="space-y-2">
              <Label htmlFor="link-db">Database</Label>
              <Select value={selectedDbId} onValueChange={setSelectedDbId}>
                <SelectTrigger id="link-db">
                  <SelectValue placeholder="Select a database…" />
                </SelectTrigger>
                <SelectContent>
                  {candidates.map((db) => (
                    <SelectItem key={db.id} value={db.id}>
                      {db.name}{" "}
                      <span className="text-muted-foreground">
                        ({db.db_type})
                      </span>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {candidates.length === 0 && (
                <p className="text-xs text-muted-foreground">
                  No unlinked databases available. Create one in this project
                  first.
                </p>
              )}
            </div>
            <div className="space-y-2">
              <Label htmlFor="link-prefix">Env var prefix (optional)</Label>
              <Input
                id="link-prefix"
                placeholder="e.g. ANALYTICS or PG"
                value={prefix}
                onChange={(e) => setPrefix(e.target.value)}
                className="font-mono"
              />
              <p className="text-xs text-muted-foreground">
                If set, vars become e.g.{" "}
                <code className="bg-muted px-1 rounded">
                  {(prefix || "").toUpperCase()}
                  {prefix && !prefix.endsWith("_") ? "_" : ""}DATABASE_URL
                </code>
                . Leave empty to inject as <code>DATABASE_URL</code>,{" "}
                <code>HOST</code>, etc.
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowDialog(false)}>
              Cancel
            </Button>
            <Button
              onClick={() => createMutation.mutate()}
              disabled={!selectedDbId || createMutation.isPending}
            >
              {createMutation.isPending ? "Linking…" : "Link"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
