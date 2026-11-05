import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  Copy,
  Plus,
  Trash2,
  Key,
  Download,
  Pause,
  Play,
  RefreshCw,
  Pencil,
} from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
import { QueryError } from "@/components/query-error";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { api, apiRequest } from "@/lib/api";
import { useTeamContext } from "@/lib/team-context";
import type {
  ApiToken,
  ApiTokenScope,
  ApiTokenResource,
  CreateApiTokenRequest,
} from "@/types/api-tokens";
import { API_TOKEN_SCOPES } from "@/types/api-tokens";

export function meta() {
  return [
    { title: "API Tokens - Rivetr" },
    { name: "description", content: "Manage API tokens for programmatic access" },
  ];
}

const SCOPE_PRESETS: { label: string; scopes: ApiTokenScope[] }[] = [
  { label: "Read only", scopes: ["read"] },
  { label: "CI deploy", scopes: ["read", "deploy"] },
  { label: "Full access", scopes: ["admin"] },
];

const GRACE_OPTIONS = [
  { value: "0", label: "Revoke the old secret now" },
  { value: "15", label: "15 minutes" },
  { value: "60", label: "1 hour" },
  { value: "1440", label: "1 day" },
  { value: "10080", label: "7 days" },
];

const ANY_TEAM = "__any__";

interface TokenForm {
  name: string;
  scopes: ApiTokenScope[];
  expiry: string;
  teamId: string;
  projectIds: string[];
}

const emptyForm: TokenForm = {
  name: "",
  scopes: ["read"],
  expiry: "",
  teamId: ANY_TEAM,
  projectIds: [],
};

function formFromToken(t: ApiToken): TokenForm {
  return {
    name: t.name,
    scopes: t.scopes,
    expiry: t.expires_at ? t.expires_at.slice(0, 10) : "",
    teamId: t.team_id ?? ANY_TEAM,
    projectIds: t.resources.filter((r) => r.type === "project").map((r) => r.id),
  };
}

function tokenStatus(t: ApiToken): { label: string; variant: "default" | "secondary" | "destructive" | "outline" } {
  if (t.disabled) return { label: "Paused", variant: "secondary" };
  if (t.expires_at && new Date(t.expires_at) <= new Date()) {
    return { label: "Expired", variant: "destructive" };
  }
  return { label: "Active", variant: "outline" };
}

const formatDate = (iso: string | null) => {
  if (!iso) return "-";
  // SQLite datetime('now') values have no zone marker; they are UTC.
  const d = new Date(/[zZ]|[+-]\d\d:\d\d$/.test(iso) ? iso : `${iso.replace(" ", "T")}Z`);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString();
};

export default function SettingsTokensPage() {
  const queryClient = useQueryClient();
  const { teams } = useTeamContext();
  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<ApiToken | null>(null);
  const [form, setForm] = useState<TokenForm>(emptyForm);
  const [showDelete, setShowDelete] = useState<ApiToken | null>(null);
  const [rolling, setRolling] = useState<ApiToken | null>(null);
  const [grace, setGrace] = useState("60");
  const [revealed, setRevealed] = useState<{ title: string; token: string } | null>(null);

  const { data: tokens = [], isLoading, error: listError, refetch: refetchList } = useQuery<ApiToken[]>({
    queryKey: ["api-tokens"],
    queryFn: () => apiRequest<ApiToken[]>("/tokens"),
  });

  const { data: projects = [] } = useQuery({
    queryKey: ["projects", "all-for-tokens"],
    queryFn: () => api.getProjects(),
    enabled: formOpen,
  });

  const invalidate = () => queryClient.invalidateQueries({ queryKey: ["api-tokens"] });

  const saveMutation = useMutation({
    mutationFn: (data: CreateApiTokenRequest) =>
      editing
        ? apiRequest<ApiToken>(`/tokens/${editing.id}`, {
            method: "PATCH",
            body: JSON.stringify({
              ...data,
              // Unchanged expiry: leave it alone. Cleared: "" removes it.
              expires_at:
                form.expiry === (editing.expires_at?.slice(0, 10) ?? "")
                  ? undefined
                  : (data.expires_at ?? ""),
              team_id: data.team_id ?? "",
            }),
          })
        : apiRequest<ApiToken>("/tokens", { method: "POST", body: JSON.stringify(data) }),
    onSuccess: (token) => {
      invalidate();
      setFormOpen(false);
      if (token.token) {
        setRevealed({ title: "Token created", token: token.token });
      } else {
        toast.success("Token updated");
      }
    },
    onError: (e: Error) => toast.error(e.message || "Failed to save token"),
  });

  const pauseMutation = useMutation({
    mutationFn: (t: ApiToken) =>
      apiRequest<ApiToken>(`/tokens/${t.id}/${t.disabled ? "resume" : "pause"}`, {
        method: "POST",
      }),
    onSuccess: (t) => {
      invalidate();
      toast.success(t.disabled ? "Token paused" : "Token resumed");
    },
    onError: (e: Error) => toast.error(e.message || "Failed to update token"),
  });

  const rollMutation = useMutation({
    mutationFn: ({ id, grace_minutes }: { id: string; grace_minutes: number }) =>
      apiRequest<ApiToken>(`/tokens/${id}/roll`, {
        method: "POST",
        body: JSON.stringify({ grace_minutes }),
      }),
    onSuccess: (t) => {
      invalidate();
      setRolling(null);
      if (t.token) setRevealed({ title: "Token rolled", token: t.token });
    },
    onError: (e: Error) => toast.error(e.message || "Failed to roll token"),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => apiRequest(`/tokens/${id}`, { method: "DELETE" }),
    onSuccess: () => {
      invalidate();
      toast.success("Token deleted");
      setShowDelete(null);
    },
    onError: () => toast.error("Failed to delete token"),
  });

  const openCreate = () => {
    setEditing(null);
    setForm(emptyForm);
    setFormOpen(true);
  };

  const openEdit = (t: ApiToken) => {
    setEditing(t);
    setForm(formFromToken(t));
    setFormOpen(true);
  };

  const toggleScope = (scope: ApiTokenScope, on: boolean) =>
    setForm((f) => ({
      ...f,
      scopes: on ? [...new Set([...f.scopes, scope])] : f.scopes.filter((s) => s !== scope),
    }));

  const toggleProject = (id: string, on: boolean) =>
    setForm((f) => ({
      ...f,
      projectIds: on ? [...f.projectIds, id] : f.projectIds.filter((p) => p !== id),
    }));

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!form.name.trim()) return;
    if (form.scopes.length === 0) {
      toast.error("Pick at least one scope");
      return;
    }
    const resources: ApiTokenResource[] = form.projectIds.map((id) => ({ type: "project", id }));
    saveMutation.mutate({
      name: form.name.trim(),
      scopes: form.scopes,
      // A date picker value means "valid through that day".
      expires_at: form.expiry ? `${form.expiry}T23:59:59Z` : undefined,
      team_id: form.teamId === ANY_TEAM ? undefined : form.teamId,
      resources,
    });
  };

  const copyToken = (token: string) => {
    navigator.clipboard.writeText(token);
    toast.success("Token copied to clipboard");
  };

  const teamName = (id: string | null) =>
    id ? (teams.find((t) => t.id === id)?.name ?? "Team") : null;

  const visibleProjects =
    form.teamId === ANY_TEAM ? projects : projects.filter((p) => p.team_id === form.teamId);

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">API Tokens</h1>
          <p className="text-muted-foreground">
            Scoped tokens for scripts, CI and the MCP server
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button variant="outline" asChild>
            <a href="/api/sdk" download="rivetr-sdk.ts">
              <Download className="h-4 w-4 mr-2" />
              Download TypeScript SDK
            </a>
          </Button>
          <Button onClick={openCreate}>
            <Plus className="h-4 w-4 mr-2" />
            Create Token
          </Button>
        </div>
      </div>

      {revealed && (
        <Card className="border-green-500/50 bg-green-500/5">
          <CardHeader>
            <CardTitle className="text-green-500 flex items-center gap-2">
              <Key className="h-4 w-4" />
              {revealed.title}
            </CardTitle>
            <CardDescription>Copy this token now, it will not be shown again.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-2">
            <div className="flex items-center gap-2">
              <code className="flex-1 rounded bg-muted px-3 py-2 text-sm font-mono break-all">
                {revealed.token}
              </code>
              <Button
                aria-label="Copy to clipboard"
                size="sm"
                variant="outline"
                onClick={() => copyToken(revealed.token)}
              >
                <Copy className="h-4 w-4" />
              </Button>
            </div>
            <Button
              size="sm"
              variant="ghost"
              className="text-muted-foreground"
              onClick={() => setRevealed(null)}
            >
              I've saved my token
            </Button>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardHeader>
          <CardTitle>API Tokens</CardTitle>
          <CardDescription>
            Send a token in the{" "}
            <code className="text-xs bg-muted px-1 rounded">Authorization: Bearer &lt;token&gt;</code>{" "}
            header. A token can never do more than your own account and team role allow.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {listError && tokens.length === 0 ? (
            <QueryError what="API tokens" error={listError} onRetry={() => refetchList()} />
          ) : isLoading ? (
            <p className="text-muted-foreground py-4 text-center text-sm">Loading…</p>
          ) : tokens.length === 0 ? (
            <p className="text-muted-foreground py-4 text-center text-sm">
              No API tokens yet. Create one to get started.
            </p>
          ) : (
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Name</TableHead>
                    <TableHead>Scopes</TableHead>
                    <TableHead>Restricted to</TableHead>
                    <TableHead>Last used</TableHead>
                    <TableHead>Expires</TableHead>
                    <TableHead />
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {tokens.map((token) => {
                    const status = tokenStatus(token);
                    const projectCount = token.resources.length;
                    return (
                      <TableRow key={token.id}>
                        <TableCell>
                          <div className="flex items-center gap-2 font-medium">
                            {token.name}
                            <Badge variant={status.variant}>{status.label}</Badge>
                          </div>
                          <div className="text-xs text-muted-foreground font-mono">
                            rvt_…{token.token_hint ?? "????"}
                            {token.previous_expires_at && (
                              <span className="ml-2 font-sans">
                                old secret valid until {formatDate(token.previous_expires_at)}
                              </span>
                            )}
                          </div>
                        </TableCell>
                        <TableCell>
                          <div className="flex flex-wrap gap-1">
                            {token.scopes.map((s) => (
                              <Badge key={s} variant="secondary" className="font-mono text-xs">
                                {s}
                              </Badge>
                            ))}
                          </div>
                        </TableCell>
                        <TableCell className="text-sm text-muted-foreground">
                          {!token.team_id && projectCount === 0 && "Everything"}
                          {token.team_id && <div>Team: {teamName(token.team_id)}</div>}
                          {projectCount > 0 && (
                            <div>
                              {projectCount} resource{projectCount === 1 ? "" : "s"}
                            </div>
                          )}
                        </TableCell>
                        <TableCell className="text-sm text-muted-foreground">
                          <div>{formatDate(token.last_used_at)}</div>
                          {token.last_used_ip && (
                            <div className="text-xs font-mono">{token.last_used_ip}</div>
                          )}
                        </TableCell>
                        <TableCell className="text-sm text-muted-foreground">
                          {formatDate(token.expires_at)}
                        </TableCell>
                        <TableCell className="text-right whitespace-nowrap">
                          <Button
                            aria-label="Edit"
                            title="Edit"
                            size="sm"
                            variant="ghost"
                            onClick={() => openEdit(token)}
                          >
                            <Pencil className="h-4 w-4" />
                          </Button>
                          <Button
                            aria-label={token.disabled ? "Resume" : "Pause"}
                            title={token.disabled ? "Resume" : "Pause"}
                            size="sm"
                            variant="ghost"
                            disabled={pauseMutation.isPending}
                            onClick={() => pauseMutation.mutate(token)}
                          >
                            {token.disabled ? <Play className="h-4 w-4" /> : <Pause className="h-4 w-4" />}
                          </Button>
                          <Button
                            aria-label="Roll"
                            title="Roll secret"
                            size="sm"
                            variant="ghost"
                            onClick={() => {
                              setGrace("60");
                              setRolling(token);
                            }}
                          >
                            <RefreshCw className="h-4 w-4" />
                          </Button>
                          <Button
                            aria-label="Delete"
                            title="Delete"
                            size="sm"
                            variant="ghost"
                            className="text-destructive hover:text-destructive"
                            onClick={() => setShowDelete(token)}
                          >
                            <Trash2 className="h-4 w-4" />
                          </Button>
                        </TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Create / edit dialog */}
      <Dialog open={formOpen} onOpenChange={setFormOpen}>
        <DialogContent className="sm:max-w-xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>{editing ? "Edit API Token" : "Create API Token"}</DialogTitle>
            <DialogDescription>
              Give the token only the scopes it needs. Restrictions narrow it further.
            </DialogDescription>
          </DialogHeader>
          <form onSubmit={handleSubmit}>
            <div className="space-y-5 py-2">
              <div className="space-y-2">
                <Label htmlFor="token-name">Name</Label>
                <Input
                  id="token-name"
                  placeholder="e.g. CI/CD Pipeline"
                  value={form.name}
                  onChange={(e) => setForm((f) => ({ ...f, name: e.target.value }))}
                  required
                />
              </div>

              <div className="space-y-2">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <Label>Scopes</Label>
                  <div className="flex flex-wrap gap-1">
                    {SCOPE_PRESETS.map((p) => (
                      <Button
                        key={p.label}
                        type="button"
                        size="sm"
                        variant="outline"
                        className="h-7 text-xs"
                        onClick={() => setForm((f) => ({ ...f, scopes: p.scopes }))}
                      >
                        {p.label}
                      </Button>
                    ))}
                  </div>
                </div>
                <div className="grid gap-2 sm:grid-cols-2">
                  {API_TOKEN_SCOPES.map((s) => (
                    <label
                      key={s.value}
                      className="flex items-start gap-2 rounded-md border p-2 text-sm cursor-pointer"
                    >
                      <Checkbox
                        className="mt-0.5"
                        checked={form.scopes.includes(s.value)}
                        onCheckedChange={(v) => toggleScope(s.value, v === true)}
                      />
                      <span>
                        <span className="font-mono font-medium">{s.value}</span>
                        <span className="block text-xs text-muted-foreground">{s.description}</span>
                      </span>
                    </label>
                  ))}
                </div>
              </div>

              <div className="space-y-2">
                <Label htmlFor="token-expiry">Expires (optional)</Label>
                <Input
                  id="token-expiry"
                  type="date"
                  value={form.expiry}
                  onChange={(e) => setForm((f) => ({ ...f, expiry: e.target.value }))}
                />
              </div>

              <div className="space-y-2">
                <Label>Team</Label>
                <Select
                  value={form.teamId}
                  onValueChange={(v) => setForm((f) => ({ ...f, teamId: v, projectIds: [] }))}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value={ANY_TEAM}>All my teams</SelectItem>
                    {teams.map((t) => (
                      <SelectItem key={t.id} value={t.id}>
                        {t.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  A team token only reaches that team's resources and cannot call admin routes.
                </p>
              </div>

              <div className="space-y-2">
                <Label>Projects (optional)</Label>
                {visibleProjects.length === 0 ? (
                  <p className="text-xs text-muted-foreground">No projects to restrict to.</p>
                ) : (
                  <div className="max-h-40 space-y-1 overflow-y-auto rounded-md border p-2">
                    {visibleProjects.map((p) => (
                      <label key={p.id} className="flex items-center gap-2 text-sm cursor-pointer">
                        <Checkbox
                          checked={form.projectIds.includes(p.id)}
                          onCheckedChange={(v) => toggleProject(p.id, v === true)}
                        />
                        {p.name}
                      </label>
                    ))}
                  </div>
                )}
                <p className="text-xs text-muted-foreground">
                  With projects selected, the token can only call routes that address those
                  projects or the apps, databases and services in them.
                </p>
              </div>
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setFormOpen(false)}>
                Cancel
              </Button>
              <Button type="submit" disabled={saveMutation.isPending}>
                {saveMutation.isPending ? "Saving…" : editing ? "Save" : "Create Token"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Roll dialog */}
      <Dialog open={!!rolling} onOpenChange={(o) => !o && setRolling(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Roll "{rolling?.name}"</DialogTitle>
            <DialogDescription>
              Issues a new secret. Keep the old one working for a while so you can update
              your scripts without downtime.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-2 py-2">
            <Label>Old secret keeps working for</Label>
            <Select value={grace} onValueChange={setGrace}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {GRACE_OPTIONS.map((g) => (
                  <SelectItem key={g.value} value={g.value}>
                    {g.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setRolling(null)}>
              Cancel
            </Button>
            <Button
              disabled={rollMutation.isPending}
              onClick={() =>
                rolling && rollMutation.mutate({ id: rolling.id, grace_minutes: Number(grace) })
              }
            >
              {rollMutation.isPending ? "Rolling…" : "Roll secret"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete confirmation */}
      <AlertDialog open={!!showDelete} onOpenChange={() => setShowDelete(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Token</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete "{showDelete?.name}"? Any scripts using this
              token will stop working immediately. Pause it instead if you may need it again.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              onClick={() => showDelete && deleteMutation.mutate(showDelete.id)}
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
