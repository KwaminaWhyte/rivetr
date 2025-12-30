import { useState } from "react";
import { Link, useParams, useNavigate } from "react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
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
import { api } from "@/lib/api";
import type {
  TeamDetail,
  TeamRole,
  TeamMemberWithUser,
} from "@/types/api";
import {
  canManageMembers,
  canDeleteTeam,
} from "@/types/api";
import {
  ArrowLeft,
  Users,
  UserPlus,
  Trash2,
  Crown,
  Shield,
  Code,
  Eye,
  Loader2,
} from "lucide-react";

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString();
}

function getRoleBadgeVariant(
  role: TeamRole
): "default" | "secondary" | "outline" {
  switch (role) {
    case "owner":
      return "default";
    case "admin":
      return "secondary";
    default:
      return "outline";
  }
}

function formatRole(role: TeamRole): string {
  return role.charAt(0).toUpperCase() + role.slice(1);
}

function getRoleIcon(role: TeamRole) {
  switch (role) {
    case "owner":
      return <Crown className="h-4 w-4" />;
    case "admin":
      return <Shield className="h-4 w-4" />;
    case "developer":
      return <Code className="h-4 w-4" />;
    case "viewer":
      return <Eye className="h-4 w-4" />;
  }
}

const ROLE_OPTIONS: { value: TeamRole; label: string; description: string }[] =
  [
    {
      value: "owner",
      label: "Owner",
      description: "Full access, can delete team",
    },
    {
      value: "admin",
      label: "Admin",
      description: "Manage apps, projects, and members",
    },
    {
      value: "developer",
      label: "Developer",
      description: "Create/edit apps, deploy, view logs",
    },
    {
      value: "viewer",
      label: "Viewer",
      description: "Read-only access",
    },
  ];

export default function TeamDetailPage() {
  const { id } = useParams();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [showInviteDialog, setShowInviteDialog] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [showRemoveMemberDialog, setShowRemoveMemberDialog] = useState(false);
  const [selectedMember, setSelectedMember] =
    useState<TeamMemberWithUser | null>(null);
  const [inviteRole, setInviteRole] = useState<TeamRole>("developer");

  const { data: team, isLoading } = useQuery<TeamDetail>({
    queryKey: ["team", id],
    queryFn: () => api.getTeam(id!),
    enabled: !!id,
  });

  const updateTeamMutation = useMutation({
    mutationFn: ({ name, slug }: { name?: string; slug?: string }) =>
      api.updateTeam(id!, { name, slug }),
    onSuccess: () => {
      toast.success("Team updated successfully");
      queryClient.invalidateQueries({ queryKey: ["team", id] });
      queryClient.invalidateQueries({ queryKey: ["teams"] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to update team");
    },
  });

  const inviteMemberMutation = useMutation({
    mutationFn: ({ userIdentifier, role }: { userIdentifier: string; role: TeamRole }) =>
      api.inviteTeamMember(id!, {
        user_identifier: userIdentifier,
        role,
      }),
    onSuccess: () => {
      toast.success("Member invited successfully");
      setShowInviteDialog(false);
      queryClient.invalidateQueries({ queryKey: ["team", id] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to invite member");
    },
  });

  const updateRoleMutation = useMutation({
    mutationFn: ({ userId, role }: { userId: string; role: TeamRole }) =>
      api.updateTeamMemberRole(id!, userId, { role }),
    onSuccess: () => {
      toast.success("Member role updated");
      queryClient.invalidateQueries({ queryKey: ["team", id] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to update member role");
    },
  });

  const removeMemberMutation = useMutation({
    mutationFn: (userId: string) => api.removeTeamMember(id!, userId),
    onSuccess: () => {
      toast.success("Member removed");
      setShowRemoveMemberDialog(false);
      setSelectedMember(null);
      queryClient.invalidateQueries({ queryKey: ["team", id] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to remove member");
    },
  });

  const deleteTeamMutation = useMutation({
    mutationFn: () => api.deleteTeam(id!),
    onSuccess: () => {
      toast.success("Team deleted");
      navigate("/settings/teams");
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to delete team");
    },
  });

  // Find current user's role in this team
  // For now, we'll get it from the members list by checking the current session
  // In a real app, you'd have this from the user context
  const currentUserRole: TeamRole | null =
    team?.members.find((m) => m.role === "owner")?.role || "viewer";

  const canManage = canManageMembers(currentUserRole);
  const canDelete = canDeleteTeam(currentUserRole);

  const isSubmitting =
    updateTeamMutation.isPending ||
    inviteMemberMutation.isPending ||
    updateRoleMutation.isPending ||
    removeMemberMutation.isPending ||
    deleteTeamMutation.isPending;

  const handleUpdateTeam = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const name = formData.get("name") as string;
    const slug = formData.get("slug") as string;

    updateTeamMutation.mutate({
      name: name?.trim() || undefined,
      slug: slug?.trim() || undefined,
    });
  };

  const handleInviteSubmit = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const userIdentifier = formData.get("userIdentifier") as string;

    if (!userIdentifier?.trim()) {
      toast.error("User email is required");
      return;
    }

    inviteMemberMutation.mutate({
      userIdentifier: userIdentifier.trim(),
      role: inviteRole,
    });
  };

  const handleRoleChange = (userId: string, newRole: TeamRole) => {
    updateRoleMutation.mutate({ userId, role: newRole });
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-16">
        <Loader2 className="h-8 w-8 animate-spin" />
      </div>
    );
  }

  if (!team) {
    return <div>Team not found</div>;
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="sm" asChild>
          <Link to="/settings/teams">
            <ArrowLeft className="h-4 w-4 mr-1" />
            Back to Teams
          </Link>
        </Button>
      </div>

      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">{team.name}</h1>
          <p className="text-muted-foreground">/{team.slug}</p>
        </div>
        {canDelete && (
          <Button
            variant="destructive"
            onClick={() => setShowDeleteDialog(true)}
          >
            <Trash2 className="h-4 w-4 mr-2" />
            Delete Team
          </Button>
        )}
      </div>

      {/* Team Settings Card */}
      {canManage && (
        <Card>
          <CardHeader>
            <CardTitle>Team Settings</CardTitle>
            <CardDescription>
              Update your team's name and URL slug.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <form onSubmit={handleUpdateTeam} className="space-y-4">
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="name">Team Name</Label>
                  <Input
                    id="name"
                    name="name"
                    defaultValue={team.name}
                    placeholder="Team name"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="slug">URL Slug</Label>
                  <Input
                    id="slug"
                    name="slug"
                    defaultValue={team.slug}
                    placeholder="url-slug"
                  />
                </div>
              </div>
              <Button type="submit" disabled={isSubmitting}>
                {updateTeamMutation.isPending ? "Saving..." : "Save Changes"}
              </Button>
            </form>
          </CardContent>
        </Card>
      )}

      {/* Members Card */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Users className="h-5 w-5" />
                Team Members
              </CardTitle>
              <CardDescription>
                Manage who has access to this team and their roles.
              </CardDescription>
            </div>
            {canManage && (
              <Button onClick={() => setShowInviteDialog(true)}>
                <UserPlus className="h-4 w-4 mr-2" />
                Invite Member
              </Button>
            )}
          </div>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Member</TableHead>
                <TableHead>Role</TableHead>
                <TableHead>Joined</TableHead>
                {canManage && <TableHead className="w-32">Actions</TableHead>}
              </TableRow>
            </TableHeader>
            <TableBody>
              {team.members.map((member) => (
                <TableRow key={member.id}>
                  <TableCell>
                    <div className="flex flex-col">
                      <span className="font-medium">{member.user_name}</span>
                      <span className="text-xs text-muted-foreground">
                        {member.user_email}
                      </span>
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-2">
                      {getRoleIcon(member.role as TeamRole)}
                      <Badge variant={getRoleBadgeVariant(member.role as TeamRole)}>
                        {formatRole(member.role as TeamRole)}
                      </Badge>
                    </div>
                  </TableCell>
                  <TableCell>{formatDate(member.created_at)}</TableCell>
                  {canManage && (
                    <TableCell>
                      <div className="flex items-center gap-2">
                        {/* Role change dropdown */}
                        {member.role !== "owner" && (
                          <Select
                            defaultValue={member.role}
                            onValueChange={(value) =>
                              handleRoleChange(member.user_id, value as TeamRole)
                            }
                            disabled={updateRoleMutation.isPending}
                          >
                            <SelectTrigger className="w-28 h-8">
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              {ROLE_OPTIONS.filter((r) =>
                                currentUserRole === "owner"
                                  ? true
                                  : r.value !== "owner"
                              ).map((role) => (
                                <SelectItem key={role.value} value={role.value}>
                                  {role.label}
                                </SelectItem>
                              ))}
                            </SelectContent>
                          </Select>
                        )}
                        {/* Remove button */}
                        {member.role !== "owner" && (
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => {
                              setSelectedMember(member);
                              setShowRemoveMemberDialog(true);
                            }}
                          >
                            <Trash2 className="h-4 w-4 text-destructive" />
                          </Button>
                        )}
                      </div>
                    </TableCell>
                  )}
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      {/* Role Legend */}
      <Card>
        <CardHeader>
          <CardTitle>Role Permissions</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
            {ROLE_OPTIONS.map((role) => (
              <div key={role.value} className="flex items-start gap-3 p-3 rounded-lg border">
                <div className="mt-0.5">{getRoleIcon(role.value)}</div>
                <div>
                  <div className="font-medium">{role.label}</div>
                  <div className="text-sm text-muted-foreground">
                    {role.description}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      {/* Invite Member Dialog */}
      <Dialog open={showInviteDialog} onOpenChange={setShowInviteDialog}>
        <DialogContent>
          <form onSubmit={handleInviteSubmit}>
            <DialogHeader>
              <DialogTitle>Invite Team Member</DialogTitle>
              <DialogDescription>
                Invite a user to join this team. They must have an existing
                account.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <Label htmlFor="userIdentifier">User Email</Label>
                <Input
                  id="userIdentifier"
                  name="userIdentifier"
                  type="email"
                  placeholder="user@example.com"
                  required
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="role">Role</Label>
                <Select
                  value={inviteRole}
                  onValueChange={(value) => setInviteRole(value as TeamRole)}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {ROLE_OPTIONS.filter((r) =>
                      currentUserRole === "owner" ? true : r.value !== "owner"
                    ).map((role) => (
                      <SelectItem key={role.value} value={role.value}>
                        <div className="flex flex-col">
                          <span>{role.label}</span>
                          <span className="text-xs text-muted-foreground">
                            {role.description}
                          </span>
                        </div>
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => setShowInviteDialog(false)}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={inviteMemberMutation.isPending}>
                {inviteMemberMutation.isPending ? "Inviting..." : "Send Invitation"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Remove Member Confirmation */}
      <AlertDialog
        open={showRemoveMemberDialog}
        onOpenChange={setShowRemoveMemberDialog}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Remove Team Member</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to remove {selectedMember?.user_name} from
              this team? They will lose access to all team resources.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel
              onClick={() => {
                setShowRemoveMemberDialog(false);
                setSelectedMember(null);
              }}
            >
              Cancel
            </AlertDialogCancel>
            <AlertDialogAction asChild>
              <Button
                variant="destructive"
                disabled={removeMemberMutation.isPending}
                onClick={() =>
                  selectedMember && removeMemberMutation.mutate(selectedMember.user_id)
                }
              >
                {removeMemberMutation.isPending ? "Removing..." : "Remove Member"}
              </Button>
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Delete Team Confirmation */}
      <AlertDialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Team</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete "{team.name}"? This action cannot
              be undone. All team memberships will be removed.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction asChild>
              <Button
                variant="destructive"
                disabled={deleteTeamMutation.isPending}
                onClick={() => deleteTeamMutation.mutate()}
              >
                {deleteTeamMutation.isPending ? "Deleting..." : "Delete Team"}
              </Button>
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
