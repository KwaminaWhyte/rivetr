/**
 * Dialog for managing per-resource permission overrides for a team member.
 * Allows admins to grant or deny access to specific apps, projects, databases, or services.
 */
import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Loader2, Plus, Trash2, ShieldCheck } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
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
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { teamsApi } from "@/lib/api/teams";
import type { ResourcePermission, TeamMemberWithUser } from "@/types/api";

interface ResourcePermissionsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  teamId: string;
  member: TeamMemberWithUser | null;
}

const RESOURCE_TYPES = [
  { value: "app", label: "App" },
  { value: "project", label: "Project" },
  { value: "database", label: "Database" },
  { value: "service", label: "Service" },
];

export function ResourcePermissionsDialog({
  open,
  onOpenChange,
  teamId,
  member,
}: ResourcePermissionsDialogProps) {
  const queryClient = useQueryClient();
  const [newResourceType, setNewResourceType] = useState("app");
  const [newResourceId, setNewResourceId] = useState("");
  const [newPermission, setNewPermission] = useState<"allow" | "deny">("allow");

  const { data: permissions = [], isLoading } = useQuery({
    queryKey: ["member-permissions", teamId, member?.user_id],
    queryFn: () => teamsApi.getMemberPermissions(teamId, member!.user_id),
    enabled: open && !!member,
  });

  const saveMutation = useMutation({
    mutationFn: (perms: ResourcePermission[]) =>
      teamsApi.setMemberPermissions(teamId, member!.user_id, {
        permissions: perms.map((p) => ({
          resource_type: p.resource_type,
          resource_id: p.resource_id,
          permission: p.permission,
        })),
      }),
    onSuccess: () => {
      toast.success("Permissions saved");
      queryClient.invalidateQueries({
        queryKey: ["member-permissions", teamId, member?.user_id],
      });
    },
    onError: (err: Error) => {
      toast.error(err.message || "Failed to save permissions");
    },
  });

  function addPermission() {
    if (!newResourceId.trim()) {
      toast.error("Resource ID is required");
      return;
    }
    const updated = [
      ...permissions,
      {
        id: crypto.randomUUID(),
        team_id: teamId,
        user_id: member!.user_id,
        resource_type: newResourceType,
        resource_id: newResourceId.trim(),
        permission: newPermission,
        created_at: new Date().toISOString(),
      },
    ];
    saveMutation.mutate(updated);
    setNewResourceId("");
  }

  function removePermission(permId: string) {
    const updated = permissions.filter((p) => p.id !== permId);
    saveMutation.mutate(updated);
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <ShieldCheck className="h-5 w-5" />
            Resource Permissions
          </DialogTitle>
          <DialogDescription>
            Fine-grained access overrides for{" "}
            <strong>{member?.user_name}</strong>. These override their team role
            for specific resources.
          </DialogDescription>
        </DialogHeader>

        {isLoading ? (
          <div className="flex items-center justify-center py-6">
            <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
          </div>
        ) : (
          <div className="space-y-4">
            {/* Existing permissions */}
            {permissions.length === 0 ? (
              <p className="text-sm text-muted-foreground text-center py-4">
                No permission overrides yet. Add one below.
              </p>
            ) : (
              <div className="divide-y border rounded-md">
                {permissions.map((perm) => (
                  <div
                    key={perm.id}
                    className="flex items-center justify-between px-3 py-2 text-sm"
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      <Badge variant="outline" className="capitalize shrink-0 text-xs">
                        {perm.resource_type}
                      </Badge>
                      <span className="font-mono text-xs truncate text-muted-foreground">
                        {perm.resource_id}
                      </span>
                    </div>
                    <div className="flex items-center gap-2 shrink-0 ml-2">
                      <Badge
                        className={
                          perm.permission === "allow"
                            ? "bg-green-100 text-green-700 border-green-300"
                            : "bg-red-100 text-red-700 border-red-300"
                        }
                        variant="outline"
                      >
                        {perm.permission}
                      </Badge>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-7 w-7 p-0 text-muted-foreground hover:text-destructive"
                        onClick={() => removePermission(perm.id)}
                        disabled={saveMutation.isPending}
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            )}

            {/* Add new permission */}
            <div className="border rounded-md p-3 space-y-3 bg-muted/30">
              <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                Add Override
              </p>
              <div className="grid grid-cols-2 gap-2">
                <div className="space-y-1">
                  <Label className="text-xs">Resource Type</Label>
                  <Select
                    value={newResourceType}
                    onValueChange={setNewResourceType}
                  >
                    <SelectTrigger className="h-8 text-xs">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {RESOURCE_TYPES.map((rt) => (
                        <SelectItem key={rt.value} value={rt.value}>
                          {rt.label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
                <div className="space-y-1">
                  <Label className="text-xs">Permission</Label>
                  <Select
                    value={newPermission}
                    onValueChange={(v) =>
                      setNewPermission(v as "allow" | "deny")
                    }
                  >
                    <SelectTrigger className="h-8 text-xs">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="allow">Allow</SelectItem>
                      <SelectItem value="deny">Deny</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
              <div className="space-y-1">
                <Label className="text-xs">Resource ID</Label>
                <div className="flex gap-2">
                  <Input
                    className="h-8 text-xs font-mono"
                    placeholder="e.g. app-uuid-here"
                    value={newResourceId}
                    onChange={(e) => setNewResourceId(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") addPermission();
                    }}
                  />
                  <Button
                    size="sm"
                    className="h-8 shrink-0"
                    onClick={addPermission}
                    disabled={saveMutation.isPending || !newResourceId.trim()}
                  >
                    {saveMutation.isPending ? (
                      <Loader2 className="h-3.5 w-3.5 animate-spin" />
                    ) : (
                      <Plus className="h-3.5 w-3.5" />
                    )}
                  </Button>
                </div>
              </div>
            </div>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
