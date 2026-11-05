/**
 * /team/roles: the built-in role presets and the team's custom roles, with an
 * editor that builds a custom role from abilities grouped by area.
 */
import { useMemo, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Pencil, Plus, Trash2 } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
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
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { teamsApi } from "@/lib/api/teams";
import type { AbilityInfo, CustomRole, TeamRole } from "@/types/api";
import { RoleIcon, useTeamRoles } from "@/components/teams/role-utils";

interface Props {
  teamId: string;
  canManage: boolean;
  currentUserRole: TeamRole | null;
  /** Abilities the caller holds; a custom role can only use these. */
  userAbilities: string[];
}

function groupByArea(abilities: AbilityInfo[]): [string, AbilityInfo[]][] {
  const groups = new Map<string, AbilityInfo[]>();
  for (const a of abilities) {
    groups.set(a.area, [...(groups.get(a.area) ?? []), a]);
  }
  return [...groups.entries()];
}

export function RolesPanel({ teamId, canManage, currentUserRole, userAbilities }: Props) {
  const queryClient = useQueryClient();
  const { data: roles, isLoading } = useTeamRoles(teamId);
  const [editing, setEditing] = useState<CustomRole | null>(null);
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [deleting, setDeleting] = useState<CustomRole | null>(null);
  const [expandedPreset, setExpandedPreset] = useState<string | null>(null);

  const groups = useMemo(() => groupByArea(roles?.abilities ?? []), [roles]);
  const isOwner = currentUserRole === "owner";
  const grantable = (key: string) => isOwner || userAbilities.includes(key);
  const abilityLabel = (key: string) => {
    const a = roles?.abilities.find((x) => x.key === key);
    return a ? `${a.area}: ${a.label}` : key;
  };

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: ["team-roles", teamId] });
    queryClient.invalidateQueries({ queryKey: ["team", teamId] });
  };

  const saveMutation = useMutation({
    mutationFn: () => {
      const body = {
        name: name.trim(),
        description: description.trim() || null,
        abilities: [...selected],
      };
      return editing
        ? teamsApi.updateRole(teamId, editing.id, body)
        : teamsApi.createRole(teamId, body);
    },
    onSuccess: () => {
      toast.success(editing ? "Role updated" : "Role created");
      setOpen(false);
      invalidate();
    },
    onError: (e: Error) => toast.error(e.message || "Failed to save role"),
  });

  const deleteMutation = useMutation({
    mutationFn: (role: CustomRole) => teamsApi.deleteRole(teamId, role.id),
    onSuccess: () => {
      toast.success("Role deleted");
      setDeleting(null);
      invalidate();
    },
    onError: (e: Error) => toast.error(e.message || "Failed to delete role"),
  });

  const openEditor = (role: CustomRole | null) => {
    setEditing(role);
    setName(role?.name ?? "");
    setDescription(role?.description ?? "");
    setSelected(new Set(role?.abilities ?? ["apps.read", "services.read", "databases.read", "projects.read"]));
    setOpen(true);
  };

  const toggle = (key: string, on: boolean) =>
    setSelected((prev) => {
      const next = new Set(prev);
      if (on) next.add(key);
      else next.delete(key);
      return next;
    });

  const toggleGroup = (items: AbilityInfo[], on: boolean) =>
    setSelected((prev) => {
      const next = new Set(prev);
      for (const a of items) {
        if (!grantable(a.key)) continue;
        if (on) next.add(a.key);
        else next.delete(a.key);
      }
      return next;
    });

  const copyPreset = (key: string) => {
    const preset = roles?.presets.find((p) => p.key === key);
    if (preset) setSelected(new Set(preset.abilities.filter(grantable)));
  };

  if (isLoading || !roles) {
    return <p className="text-sm text-muted-foreground">Loading roles…</p>;
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Presets</CardTitle>
          <CardDescription>
            Built-in roles. Existing owner, admin, developer and viewer members kept the same
            access they had before presets.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-2">
          {roles.presets.map((p) => (
            <div key={p.key} className="rounded-lg border p-3">
              <button
                type="button"
                className="flex w-full items-start gap-3 text-left"
                onClick={() => setExpandedPreset(expandedPreset === p.key ? null : p.key)}
                aria-expanded={expandedPreset === p.key}
              >
                <div className="mt-0.5">
                  <RoleIcon role={p.key} />
                </div>
                <div className="flex-1">
                  <div className="font-medium">{p.name}</div>
                  <div className="text-sm text-muted-foreground">{p.description}</div>
                </div>
                <span className="text-xs text-muted-foreground whitespace-nowrap">
                  {p.abilities.length} abilities
                </span>
              </button>
              {expandedPreset === p.key && (
                <div className="mt-3 flex flex-wrap gap-1">
                  {p.abilities.map((a) => (
                    <Badge key={a} variant="secondary" className="font-mono text-xs">
                      {a}
                    </Badge>
                  ))}
                </div>
              )}
            </div>
          ))}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <CardTitle>Custom roles</CardTitle>
              <CardDescription>
                Pick exactly the abilities a role grants. Assign custom roles on the Members
                page.
              </CardDescription>
            </div>
            {canManage && (
              <Button onClick={() => openEditor(null)}>
                <Plus className="h-4 w-4 mr-2" />
                New role
              </Button>
            )}
          </div>
        </CardHeader>
        <CardContent className="overflow-x-auto">
          {roles.custom.length === 0 ? (
            <p className="py-4 text-center text-sm text-muted-foreground">
              No custom roles yet.
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Abilities</TableHead>
                  <TableHead>Members</TableHead>
                  {canManage && <TableHead />}
                </TableRow>
              </TableHeader>
              <TableBody>
                {roles.custom.map((r) => (
                  <TableRow key={r.id}>
                    <TableCell>
                      <div className="font-medium">{r.name}</div>
                      {r.description && (
                        <div className="text-xs text-muted-foreground">{r.description}</div>
                      )}
                    </TableCell>
                    <TableCell>
                      <div className="flex max-w-xl flex-wrap gap-1">
                        {r.abilities.length === 0 && (
                          <span className="text-xs text-muted-foreground">None</span>
                        )}
                        {r.abilities.map((a) => (
                          <Badge key={a} variant="secondary" className="text-xs" title={a}>
                            {abilityLabel(a)}
                          </Badge>
                        ))}
                      </div>
                    </TableCell>
                    <TableCell>{r.member_count}</TableCell>
                    {canManage && (
                      <TableCell className="text-right whitespace-nowrap">
                        <Button
                          size="sm"
                          variant="ghost"
                          aria-label="Edit role"
                          onClick={() => openEditor(r)}
                        >
                          <Pencil className="h-4 w-4" />
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          aria-label="Delete role"
                          className="text-destructive hover:text-destructive"
                          onClick={() => setDeleting(r)}
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </TableCell>
                    )}
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="sm:max-w-3xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>{editing ? `Edit ${editing.name}` : "New custom role"}</DialogTitle>
            <DialogDescription>
              Abilities your own role does not have cannot be granted.
            </DialogDescription>
          </DialogHeader>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              if (name.trim()) saveMutation.mutate();
            }}
            className="space-y-5"
          >
            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="role-name">Name</Label>
                <Input
                  id="role-name"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="e.g. Release manager"
                  maxLength={64}
                  required
                />
              </div>
              <div className="space-y-2">
                <Label>Start from preset</Label>
                <Select onValueChange={copyPreset}>
                  <SelectTrigger>
                    <SelectValue placeholder="Copy abilities of…" />
                  </SelectTrigger>
                  <SelectContent>
                    {roles.presets.map((p) => (
                      <SelectItem key={p.key} value={p.key}>
                        {p.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="role-description">Description (optional)</Label>
              <Input
                id="role-description"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder="What this role is for"
              />
            </div>

            <div className="grid gap-4 sm:grid-cols-2">
              {groups.map(([area, items]) => {
                const allOn = items.filter((a) => grantable(a.key)).every((a) => selected.has(a.key));
                return (
                  <fieldset key={area} className="rounded-lg border p-3">
                    <legend className="px-1 text-sm font-medium">{area}</legend>
                    <label className="mb-2 flex items-center gap-2 text-xs text-muted-foreground cursor-pointer">
                      <Checkbox
                        checked={allOn}
                        onCheckedChange={(v) => toggleGroup(items, v === true)}
                      />
                      All
                    </label>
                    <div className="space-y-2">
                      {items.map((a) => (
                        <label
                          key={a.key}
                          className={`flex items-start gap-2 text-sm ${
                            grantable(a.key) ? "cursor-pointer" : "opacity-50"
                          }`}
                          title={grantable(a.key) ? a.key : "Your role does not have this ability"}
                        >
                          <Checkbox
                            className="mt-0.5"
                            checked={selected.has(a.key)}
                            disabled={!grantable(a.key)}
                            onCheckedChange={(v) => toggle(a.key, v === true)}
                          />
                          <span>
                            <span className="font-medium">{a.label}</span>
                            <span className="block text-xs text-muted-foreground">
                              {a.description}
                            </span>
                          </span>
                        </label>
                      ))}
                    </div>
                  </fieldset>
                );
              })}
            </div>

            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setOpen(false)}>
                Cancel
              </Button>
              <Button type="submit" disabled={saveMutation.isPending}>
                {saveMutation.isPending ? "Saving…" : editing ? "Save role" : "Create role"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <AlertDialog open={!!deleting} onOpenChange={(o) => !o && setDeleting(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete role "{deleting?.name}"?</AlertDialogTitle>
            <AlertDialogDescription>
              Roles that members or pending invitations still use cannot be deleted; give them
              another role first.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              onClick={() => deleting && deleteMutation.mutate(deleting)}
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
