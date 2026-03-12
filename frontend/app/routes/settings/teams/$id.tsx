import { useState, useEffect } from "react";
import { Link, useParams, useNavigate, useSearchParams } from "react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useBreadcrumb } from "@/lib/breadcrumb-context";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";

export function meta() {
  return [
    { title: "Team Details - Rivetr" },
    { name: "description", content: "Manage team members and permissions" },
  ];
}
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui/tabs";
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
  TeamInvitation,
} from "@/types/api";
import { canManageMembers, canDeleteTeam } from "@/types/api";
import {
  ArrowLeft,
  Users,
  Mail,
  Loader2,
  Trash2,
  Activity,
  SlidersHorizontal,
} from "lucide-react";
import { TeamNotificationChannelsCard } from "@/components/team-notification-channels-card";
import { SharedEnvVarsTable } from "@/components/shared-env-vars-table";
import { MembersTab } from "@/components/teams/members-tab";
import { InvitationsTab } from "@/components/teams/invitations-tab";
import { AuditTab } from "@/components/teams/audit-tab";

const ROLE_OPTIONS: { value: TeamRole; label: string; description: string }[] = [
  { value: "owner", label: "Owner", description: "Full access, can delete team" },
  { value: "admin", label: "Admin", description: "Manage apps, projects, and members" },
  { value: "developer", label: "Developer", description: "Create/edit apps, deploy, view logs" },
  { value: "viewer", label: "Viewer", description: "Read-only access" },
];

function isExpired(dateStr: string): boolean {
  return new Date(dateStr) < new Date();
}

export default function TeamDetailPage() {
  const { id } = useParams();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const queryClient = useQueryClient();

  const initialTab = searchParams.get("tab") || "members";
  const [activeTab, setActiveTab] = useState(initialTab);

  const [showInviteDialog, setShowInviteDialog] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [showRemoveMemberDialog, setShowRemoveMemberDialog] = useState(false);
  const [showRevokeInvitationDialog, setShowRevokeInvitationDialog] = useState(false);
  const [showRoleChangeDialog, setShowRoleChangeDialog] = useState(false);
  const [selectedMember, setSelectedMember] = useState<TeamMemberWithUser | null>(null);
  const [selectedInvitation, setSelectedInvitation] = useState<TeamInvitation | null>(null);
  const [pendingRoleChange, setPendingRoleChange] = useState<{
    member: TeamMemberWithUser;
    newRole: TeamRole;
  } | null>(null);
  const [inviteRole, setInviteRole] = useState<TeamRole>("developer");
  const [inviteEmail, setInviteEmail] = useState("");

  const { setItems } = useBreadcrumb();

  const { data: team, isLoading } = useQuery<TeamDetail>({
    queryKey: ["team", id],
    queryFn: () => api.getTeam(id!),
    enabled: !!id,
  });

  useEffect(() => {
    if (team) {
      setItems([
        { label: "Teams", href: "/teams" },
        { label: team.name },
      ]);
    }
  }, [team, setItems]);

  const { data: invitations = [], isLoading: isLoadingInvitations } = useQuery<TeamInvitation[]>({
    queryKey: ["team-invitations", id],
    queryFn: () => api.getTeamInvitations(id!),
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
      api.inviteTeamMember(id!, { user_identifier: userIdentifier, role }),
    onSuccess: () => {
      toast.success("Member invited successfully");
      setShowInviteDialog(false);
      queryClient.invalidateQueries({ queryKey: ["team", id] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to invite member");
    },
  });

  const createInvitationMutation = useMutation({
    mutationFn: ({ email, role }: { email: string; role: TeamRole }) =>
      api.createTeamInvitation(id!, { email, role }),
    onSuccess: () => {
      toast.success("Invitation sent successfully");
      queryClient.invalidateQueries({ queryKey: ["team-invitations", id] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to send invitation");
    },
  });

  const resendInvitationMutation = useMutation({
    mutationFn: (invitationId: string) => api.resendTeamInvitation(id!, invitationId),
    onSuccess: () => {
      toast.success("Invitation email resent");
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to resend invitation");
    },
  });

  const revokeInvitationMutation = useMutation({
    mutationFn: (invitationId: string) => api.deleteTeamInvitation(id!, invitationId),
    onSuccess: () => {
      toast.success("Invitation revoked");
      setShowRevokeInvitationDialog(false);
      setSelectedInvitation(null);
      queryClient.invalidateQueries({ queryKey: ["team-invitations", id] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to revoke invitation");
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
      navigate("/teams");
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to delete team");
    },
  });

  const currentUserRole: TeamRole | null = team?.user_role ?? null;
  const canManage = canManageMembers(currentUserRole);
  const canDelete = canDeleteTeam(currentUserRole);

  const isSubmitting =
    updateTeamMutation.isPending ||
    inviteMemberMutation.isPending ||
    createInvitationMutation.isPending ||
    resendInvitationMutation.isPending ||
    revokeInvitationMutation.isPending ||
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
    inviteMemberMutation.mutate({ userIdentifier: userIdentifier.trim(), role: inviteRole });
  };

  const handleRoleChange = (member: TeamMemberWithUser, newRole: TeamRole) => {
    if (member.role === newRole) return;
    setPendingRoleChange({ member, newRole });
    setShowRoleChangeDialog(true);
  };

  const confirmRoleChange = () => {
    if (pendingRoleChange) {
      updateRoleMutation.mutate(
        { userId: pendingRoleChange.member.user_id, role: pendingRoleChange.newRole },
        {
          onSuccess: () => {
            setShowRoleChangeDialog(false);
            setPendingRoleChange(null);
          },
          onError: () => {
            setShowRoleChangeDialog(false);
            setPendingRoleChange(null);
          },
        }
      );
    }
  };

  const handleTabChange = (value: string) => {
    setActiveTab(value);
    setSearchParams({ tab: value });
  };

  const pendingInvitations = invitations.filter(
    (inv) => !inv.accepted_at && !isExpired(inv.expires_at)
  );

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
          <Link to="/teams">
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
          <Button variant="destructive" onClick={() => setShowDeleteDialog(true)}>
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
            <CardDescription>Update your team's name and URL slug.</CardDescription>
          </CardHeader>
          <CardContent>
            <form onSubmit={handleUpdateTeam} className="space-y-4">
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="name">Team Name</Label>
                  <Input id="name" name="name" defaultValue={team.name} placeholder="Team name" />
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

      {/* Tabs */}
      <Tabs value={activeTab} onValueChange={handleTabChange}>
        <TabsList>
          <TabsTrigger value="members" className="gap-2">
            <Users className="h-4 w-4" />
            Members ({team.members.length})
          </TabsTrigger>
          {canManage && (
            <TabsTrigger value="invitations" className="gap-2">
              <Mail className="h-4 w-4" />
              Invitations ({pendingInvitations.length})
            </TabsTrigger>
          )}
          {canManage && (
            <TabsTrigger value="activity" className="gap-2">
              <Activity className="h-4 w-4" />
              Activity
            </TabsTrigger>
          )}
          <TabsTrigger value="shared-vars" className="gap-2">
            <SlidersHorizontal className="h-4 w-4" />
            Shared Variables
          </TabsTrigger>
        </TabsList>

        {/* Members Tab */}
        <TabsContent value="members">
          <MembersTab
            members={team.members}
            currentUserRole={currentUserRole}
            canManage={canManage}
            isUpdatingRole={updateRoleMutation.isPending}
            onShowInviteDialog={() => setShowInviteDialog(true)}
            onRoleChange={handleRoleChange}
            onRemoveMember={(member) => {
              setSelectedMember(member);
              setShowRemoveMemberDialog(true);
            }}
          />
        </TabsContent>

        {/* Invitations Tab */}
        {canManage && (
          <TabsContent value="invitations">
            <InvitationsTab
              pendingInvitations={pendingInvitations}
              currentUserRole={currentUserRole}
              isLoadingInvitations={isLoadingInvitations}
              isResending={resendInvitationMutation.isPending}
              isRevoking={revokeInvitationMutation.isPending}
              isCreatingInvitation={createInvitationMutation.isPending}
              onCreateInvitation={(email, role) =>
                createInvitationMutation.mutate({ email, role })
              }
              onResend={(invitationId) => resendInvitationMutation.mutate(invitationId)}
              onRevoke={(invitation) => {
                setSelectedInvitation(invitation);
                setShowRevokeInvitationDialog(true);
              }}
            />
          </TabsContent>
        )}

        {/* Activity Tab */}
        {canManage && (
          <TabsContent value="activity">
            <AuditTab teamId={id!} isActiveTab={activeTab === "activity"} />
          </TabsContent>
        )}

        {/* Shared Variables Tab */}
        <TabsContent value="shared-vars">
          <SharedEnvVarsTable
            scope="team"
            scopeId={id!}
            title="Team Shared Variables"
            description="These variables are inherited by all apps in this team. App-level variables take precedence."
          />
        </TabsContent>
      </Tabs>

      {/* Notification Channels */}
      {canManage && <TeamNotificationChannelsCard teamId={id!} />}

      {/* Invite Member Dialog */}
      <Dialog open={showInviteDialog} onOpenChange={setShowInviteDialog}>
        <DialogContent>
          <form onSubmit={handleInviteSubmit}>
            <DialogHeader>
              <DialogTitle>Invite Team Member</DialogTitle>
              <DialogDescription>
                Invite a user to join this team. They must have an existing account.
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
      <AlertDialog open={showRemoveMemberDialog} onOpenChange={setShowRemoveMemberDialog}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Remove Team Member</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to remove {selectedMember?.user_name} from this team?
              They will lose access to all team resources.
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

      {/* Role Change Confirmation */}
      <AlertDialog
        open={showRoleChangeDialog}
        onOpenChange={(open) => {
          setShowRoleChangeDialog(open);
          if (!open) setPendingRoleChange(null);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Change Member Role</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to change {pendingRoleChange?.member.user_name}'s role from{" "}
              <span className="font-medium capitalize">{pendingRoleChange?.member.role}</span> to{" "}
              <span className="font-medium capitalize">{pendingRoleChange?.newRole}</span>?
              {pendingRoleChange?.newRole === "owner" && (
                <span className="block mt-2 text-yellow-600">
                  Warning: This will give them full control over the team, including the ability
                  to remove other owners.
                </span>
              )}
              {pendingRoleChange?.member.role === "admin" &&
                pendingRoleChange?.newRole !== "owner" &&
                pendingRoleChange?.newRole !== "admin" && (
                  <span className="block mt-2">
                    They will lose the ability to manage team members and settings.
                  </span>
                )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel
              onClick={() => {
                setShowRoleChangeDialog(false);
                setPendingRoleChange(null);
              }}
            >
              Cancel
            </AlertDialogCancel>
            <AlertDialogAction asChild>
              <Button disabled={updateRoleMutation.isPending} onClick={confirmRoleChange}>
                {updateRoleMutation.isPending ? "Updating..." : "Change Role"}
              </Button>
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Revoke Invitation Confirmation */}
      <AlertDialog
        open={showRevokeInvitationDialog}
        onOpenChange={setShowRevokeInvitationDialog}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Revoke Invitation</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to revoke the invitation for {selectedInvitation?.email}?
              They will no longer be able to join the team using this invitation link.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel
              onClick={() => {
                setShowRevokeInvitationDialog(false);
                setSelectedInvitation(null);
              }}
            >
              Cancel
            </AlertDialogCancel>
            <AlertDialogAction asChild>
              <Button
                variant="destructive"
                disabled={revokeInvitationMutation.isPending}
                onClick={() =>
                  selectedInvitation && revokeInvitationMutation.mutate(selectedInvitation.id)
                }
              >
                {revokeInvitationMutation.isPending ? "Revoking..." : "Revoke Invitation"}
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
              Are you sure you want to delete "{team.name}"? This action cannot be undone.
              All team memberships will be removed.
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
