import { useState } from "react";
import { Link, useParams, useNavigate, useSearchParams } from "react-router";
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
  TeamAuditLogPage,
  TeamAuditLogQuery,
  TeamAuditAction,
  TeamAuditResourceType,
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
  Mail,
  MailPlus,
  RefreshCw,
  Clock,
  Activity,
  ChevronLeft,
  ChevronRight,
  Calendar,
  Filter,
} from "lucide-react";
import { TeamNotificationChannelsCard } from "@/components/team-notification-channels-card";

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString();
}

function formatDateTime(dateStr: string): string {
  return new Date(dateStr).toLocaleString();
}

function formatRelativeTime(dateStr: string): string {
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = date.getTime() - now.getTime();
  const diffDays = Math.ceil(diffMs / (1000 * 60 * 60 * 24));

  if (diffDays < 0) {
    return "Expired";
  } else if (diffDays === 0) {
    return "Expires today";
  } else if (diffDays === 1) {
    return "Expires tomorrow";
  } else {
    return `Expires in ${diffDays} days`;
  }
}

function isExpired(dateStr: string): boolean {
  return new Date(dateStr) < new Date();
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

// Audit log action types for filter dropdown
const AUDIT_ACTION_OPTIONS: { value: TeamAuditAction; label: string }[] = [
  { value: "team_created", label: "Team Created" },
  { value: "team_updated", label: "Team Updated" },
  { value: "team_deleted", label: "Team Deleted" },
  { value: "member_invited", label: "Member Invited" },
  { value: "member_joined", label: "Member Joined" },
  { value: "member_removed", label: "Member Removed" },
  { value: "role_changed", label: "Role Changed" },
  { value: "invitation_created", label: "Invitation Created" },
  { value: "invitation_revoked", label: "Invitation Revoked" },
  { value: "invitation_accepted", label: "Invitation Accepted" },
  { value: "invitation_resent", label: "Invitation Resent" },
  { value: "app_created", label: "App Created" },
  { value: "app_updated", label: "App Updated" },
  { value: "app_deleted", label: "App Deleted" },
  { value: "project_created", label: "Project Created" },
  { value: "project_updated", label: "Project Updated" },
  { value: "project_deleted", label: "Project Deleted" },
  { value: "database_created", label: "Database Created" },
  { value: "database_deleted", label: "Database Deleted" },
  { value: "service_created", label: "Service Created" },
  { value: "service_deleted", label: "Service Deleted" },
  { value: "deployment_triggered", label: "Deployment Triggered" },
  { value: "deployment_rolled_back", label: "Deployment Rolled Back" },
];

// Audit log resource types for filter dropdown
const AUDIT_RESOURCE_TYPE_OPTIONS: { value: TeamAuditResourceType; label: string }[] = [
  { value: "team", label: "Team" },
  { value: "member", label: "Member" },
  { value: "invitation", label: "Invitation" },
  { value: "app", label: "App" },
  { value: "project", label: "Project" },
  { value: "database", label: "Database" },
  { value: "service", label: "Service" },
  { value: "deployment", label: "Deployment" },
];

function formatActionLabel(action: string): string {
  return action
    .split("_")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}

function getActionBadgeVariant(
  action: string
): "default" | "secondary" | "outline" | "destructive" {
  if (action.includes("deleted") || action.includes("removed") || action.includes("revoked")) {
    return "destructive";
  }
  if (action.includes("created") || action.includes("joined") || action.includes("accepted")) {
    return "default";
  }
  if (action.includes("updated") || action.includes("changed")) {
    return "secondary";
  }
  return "outline";
}

export default function TeamDetailPage() {
  const { id } = useParams();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const queryClient = useQueryClient();

  // Get initial tab from URL or default to "members"
  const initialTab = searchParams.get("tab") || "members";
  const [activeTab, setActiveTab] = useState(initialTab);

  const [showInviteDialog, setShowInviteDialog] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [showRemoveMemberDialog, setShowRemoveMemberDialog] = useState(false);
  const [showRevokeInvitationDialog, setShowRevokeInvitationDialog] = useState(false);
  const [showRoleChangeDialog, setShowRoleChangeDialog] = useState(false);
  const [selectedMember, setSelectedMember] =
    useState<TeamMemberWithUser | null>(null);
  const [selectedInvitation, setSelectedInvitation] =
    useState<TeamInvitation | null>(null);
  const [pendingRoleChange, setPendingRoleChange] = useState<{
    member: TeamMemberWithUser;
    newRole: TeamRole;
  } | null>(null);
  const [inviteRole, setInviteRole] = useState<TeamRole>("developer");
  const [inviteEmail, setInviteEmail] = useState("");

  // Audit log filters
  const [auditActionFilter, setAuditActionFilter] = useState<string>("");
  const [auditResourceTypeFilter, setAuditResourceTypeFilter] = useState<string>("");
  const [auditStartDate, setAuditStartDate] = useState<string>("");
  const [auditEndDate, setAuditEndDate] = useState<string>("");
  const [auditPage, setAuditPage] = useState(1);
  const auditPerPage = 20;

  const { data: team, isLoading } = useQuery<TeamDetail>({
    queryKey: ["team", id],
    queryFn: () => api.getTeam(id!),
    enabled: !!id,
  });

  const { data: invitations = [], isLoading: isLoadingInvitations } = useQuery<TeamInvitation[]>({
    queryKey: ["team-invitations", id],
    queryFn: () => api.getTeamInvitations(id!),
    enabled: !!id,
  });

  // Build audit log query parameters
  const auditLogQuery: TeamAuditLogQuery = {
    page: auditPage,
    per_page: auditPerPage,
    ...(auditActionFilter && { action: auditActionFilter }),
    ...(auditResourceTypeFilter && { resource_type: auditResourceTypeFilter }),
    ...(auditStartDate && { start_date: new Date(auditStartDate).toISOString() }),
    ...(auditEndDate && { end_date: new Date(auditEndDate + "T23:59:59").toISOString() }),
  };

  const { data: auditLogs, isLoading: isLoadingAuditLogs } = useQuery<TeamAuditLogPage>({
    queryKey: ["team-audit-logs", id, auditLogQuery],
    queryFn: () => api.getTeamAuditLogs(id!, auditLogQuery),
    enabled: !!id && activeTab === "activity",
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

  const createInvitationMutation = useMutation({
    mutationFn: ({ email, role }: { email: string; role: TeamRole }) =>
      api.createTeamInvitation(id!, { email, role }),
    onSuccess: () => {
      toast.success("Invitation sent successfully");
      setInviteEmail("");
      setInviteRole("developer");
      queryClient.invalidateQueries({ queryKey: ["team-invitations", id] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to send invitation");
    },
  });

  const resendInvitationMutation = useMutation({
    mutationFn: (invitationId: string) =>
      api.resendTeamInvitation(id!, invitationId),
    onSuccess: () => {
      toast.success("Invitation email resent");
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to resend invitation");
    },
  });

  const revokeInvitationMutation = useMutation({
    mutationFn: (invitationId: string) =>
      api.deleteTeamInvitation(id!, invitationId),
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
      navigate("/settings/teams");
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to delete team");
    },
  });

  // Get current user's role in this team from the API response
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

    inviteMemberMutation.mutate({
      userIdentifier: userIdentifier.trim(),
      role: inviteRole,
    });
  };

  const handleCreateInvitation = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!inviteEmail?.trim()) {
      toast.error("Email is required");
      return;
    }
    createInvitationMutation.mutate({
      email: inviteEmail.trim(),
      role: inviteRole,
    });
  };

  const handleRoleChange = (member: TeamMemberWithUser, newRole: TeamRole) => {
    // Don't show dialog if role hasn't changed
    if (member.role === newRole) return;

    // Show confirmation dialog
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

  // Filter pending invitations (not accepted and not expired)
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

      {/* Tabs for Members, Invitations, and Activity */}
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
        </TabsList>

        {/* Members Tab */}
        <TabsContent value="members">
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
                            {/* Role change dropdown - only for members current user can manage */}
                            {/* Owners can change any role; Admins can only change developer/viewer */}
                            {(currentUserRole === "owner" && member.role !== "owner") ||
                             (currentUserRole === "admin" && (member.role === "developer" || member.role === "viewer")) ? (
                              <Select
                                value={member.role}
                                onValueChange={(value) =>
                                  handleRoleChange(member, value as TeamRole)
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
                                      : r.value !== "owner" && r.value !== "admin"
                                  ).map((role) => (
                                    <SelectItem key={role.value} value={role.value}>
                                      {role.label}
                                    </SelectItem>
                                  ))}
                                </SelectContent>
                              </Select>
                            ) : null}
                            {/* Remove button - show for members current user can remove */}
                            {/* Owners can remove anyone (backend validates last owner); Admins can remove developers/viewers */}
                            {(currentUserRole === "owner" ||
                              (currentUserRole === "admin" && (member.role === "developer" || member.role === "viewer"))) && (
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => {
                                  setSelectedMember(member);
                                  setShowRemoveMemberDialog(true);
                                }}
                                title="Remove member"
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
          <Card className="mt-6">
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
        </TabsContent>

        {/* Invitations Tab */}
        {canManage && (
          <TabsContent value="invitations">
            <Card>
              <CardHeader>
                <div className="flex items-center justify-between">
                  <div>
                    <CardTitle className="flex items-center gap-2">
                      <MailPlus className="h-5 w-5" />
                      Send Invitation
                    </CardTitle>
                    <CardDescription>
                      Invite new users to join this team via email.
                    </CardDescription>
                  </div>
                </div>
              </CardHeader>
              <CardContent>
                <form onSubmit={handleCreateInvitation} className="flex gap-4 items-end">
                  <div className="flex-1 space-y-2">
                    <Label htmlFor="inviteEmail">Email Address</Label>
                    <Input
                      id="inviteEmail"
                      type="email"
                      placeholder="user@example.com"
                      value={inviteEmail}
                      onChange={(e) => setInviteEmail(e.target.value)}
                      required
                    />
                  </div>
                  <div className="w-40 space-y-2">
                    <Label htmlFor="inviteRoleSelect">Role</Label>
                    <Select
                      value={inviteRole}
                      onValueChange={(value) => setInviteRole(value as TeamRole)}
                    >
                      <SelectTrigger id="inviteRoleSelect">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {ROLE_OPTIONS.filter((r) =>
                          currentUserRole === "owner" ? true : r.value !== "owner"
                        ).map((role) => (
                          <SelectItem key={role.value} value={role.value}>
                            {role.label}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                  <Button type="submit" disabled={createInvitationMutation.isPending}>
                    {createInvitationMutation.isPending ? (
                      <>
                        <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                        Sending...
                      </>
                    ) : (
                      <>
                        <Mail className="h-4 w-4 mr-2" />
                        Send Invitation
                      </>
                    )}
                  </Button>
                </form>
              </CardContent>
            </Card>

            <Card className="mt-6">
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Clock className="h-5 w-5" />
                  Pending Invitations
                </CardTitle>
                <CardDescription>
                  Invitations that have been sent but not yet accepted.
                </CardDescription>
              </CardHeader>
              <CardContent>
                {isLoadingInvitations ? (
                  <div className="flex items-center justify-center py-8">
                    <Loader2 className="h-6 w-6 animate-spin" />
                  </div>
                ) : pendingInvitations.length === 0 ? (
                  <div className="text-center py-8 text-muted-foreground">
                    No pending invitations
                  </div>
                ) : (
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Email</TableHead>
                        <TableHead>Role</TableHead>
                        <TableHead>Sent</TableHead>
                        <TableHead>Expiry</TableHead>
                        <TableHead className="w-40">Actions</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {pendingInvitations.map((invitation) => (
                        <TableRow key={invitation.id}>
                          <TableCell>
                            <span className="font-medium">{invitation.email}</span>
                          </TableCell>
                          <TableCell>
                            <div className="flex items-center gap-2">
                              {getRoleIcon(invitation.role)}
                              <Badge variant={getRoleBadgeVariant(invitation.role)}>
                                {formatRole(invitation.role)}
                              </Badge>
                            </div>
                          </TableCell>
                          <TableCell>
                            <span className="text-sm text-muted-foreground">
                              {formatDate(invitation.created_at)}
                            </span>
                          </TableCell>
                          <TableCell>
                            <span className="text-sm text-muted-foreground">
                              {formatRelativeTime(invitation.expires_at)}
                            </span>
                          </TableCell>
                          <TableCell>
                            <div className="flex items-center gap-2">
                              <Button
                                variant="outline"
                                size="sm"
                                onClick={() => resendInvitationMutation.mutate(invitation.id)}
                                disabled={resendInvitationMutation.isPending}
                                title="Resend invitation email"
                              >
                                <RefreshCw className={`h-4 w-4 ${resendInvitationMutation.isPending ? 'animate-spin' : ''}`} />
                              </Button>
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => {
                                  setSelectedInvitation(invitation);
                                  setShowRevokeInvitationDialog(true);
                                }}
                                title="Revoke invitation"
                              >
                                <Trash2 className="h-4 w-4 text-destructive" />
                              </Button>
                            </div>
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                )}
              </CardContent>
            </Card>
          </TabsContent>
        )}

        {/* Activity Tab (Audit Logs) */}
        {canManage && (
          <TabsContent value="activity">
            <Card>
              <CardHeader>
                <div className="flex items-center justify-between">
                  <div>
                    <CardTitle className="flex items-center gap-2">
                      <Activity className="h-5 w-5" />
                      Team Activity
                    </CardTitle>
                    <CardDescription>
                      View all actions and events that have occurred in this team.
                    </CardDescription>
                  </div>
                </div>
              </CardHeader>
              <CardContent className="space-y-4">
                {/* Filters */}
                <div className="flex flex-wrap gap-4">
                  <div className="flex items-center gap-2">
                    <Filter className="h-4 w-4 text-muted-foreground" />
                    <Select
                      value={auditActionFilter || "all"}
                      onValueChange={(value) => {
                        setAuditActionFilter(value === "all" ? "" : value);
                        setAuditPage(1);
                      }}
                    >
                      <SelectTrigger className="w-48">
                        <SelectValue placeholder="All Actions" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="all">All Actions</SelectItem>
                        {AUDIT_ACTION_OPTIONS.map((option) => (
                          <SelectItem key={option.value} value={option.value}>
                            {option.label}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>

                  <div>
                    <Select
                      value={auditResourceTypeFilter || "all"}
                      onValueChange={(value) => {
                        setAuditResourceTypeFilter(value === "all" ? "" : value);
                        setAuditPage(1);
                      }}
                    >
                      <SelectTrigger className="w-40">
                        <SelectValue placeholder="All Resources" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="all">All Resources</SelectItem>
                        {AUDIT_RESOURCE_TYPE_OPTIONS.map((option) => (
                          <SelectItem key={option.value} value={option.value}>
                            {option.label}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>

                  <div className="flex items-center gap-2">
                    <Calendar className="h-4 w-4 text-muted-foreground" />
                    <Input
                      type="date"
                      value={auditStartDate}
                      onChange={(e) => {
                        setAuditStartDate(e.target.value);
                        setAuditPage(1);
                      }}
                      className="w-36"
                      placeholder="Start date"
                    />
                    <span className="text-muted-foreground">to</span>
                    <Input
                      type="date"
                      value={auditEndDate}
                      onChange={(e) => {
                        setAuditEndDate(e.target.value);
                        setAuditPage(1);
                      }}
                      className="w-36"
                      placeholder="End date"
                    />
                  </div>

                  {(auditActionFilter || auditResourceTypeFilter || auditStartDate || auditEndDate) && (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => {
                        setAuditActionFilter("");
                        setAuditResourceTypeFilter("");
                        setAuditStartDate("");
                        setAuditEndDate("");
                        setAuditPage(1);
                      }}
                    >
                      Clear Filters
                    </Button>
                  )}
                </div>

                {/* Audit Log Table */}
                {isLoadingAuditLogs ? (
                  <div className="flex items-center justify-center py-8">
                    <Loader2 className="h-6 w-6 animate-spin" />
                  </div>
                ) : !auditLogs || auditLogs.items.length === 0 ? (
                  <div className="text-center py-8 text-muted-foreground">
                    No activity found
                  </div>
                ) : (
                  <>
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead className="w-40">Timestamp</TableHead>
                          <TableHead className="w-36">User</TableHead>
                          <TableHead className="w-44">Action</TableHead>
                          <TableHead className="w-28">Resource</TableHead>
                          <TableHead>Details</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {auditLogs.items.map((log) => (
                          <TableRow key={log.id}>
                            <TableCell className="text-sm text-muted-foreground">
                              {formatDateTime(log.created_at)}
                            </TableCell>
                            <TableCell>
                              <div className="flex flex-col">
                                <span className="font-medium text-sm truncate max-w-32">
                                  {log.user_name || "System"}
                                </span>
                                {log.user_email && (
                                  <span className="text-xs text-muted-foreground truncate max-w-32">
                                    {log.user_email}
                                  </span>
                                )}
                              </div>
                            </TableCell>
                            <TableCell>
                              <Badge variant={getActionBadgeVariant(log.action)}>
                                {formatActionLabel(log.action)}
                              </Badge>
                            </TableCell>
                            <TableCell>
                              <span className="text-sm capitalize">{log.resource_type}</span>
                            </TableCell>
                            <TableCell>
                              {log.details && (
                                <span className="text-sm text-muted-foreground">
                                  {Object.entries(log.details)
                                    .filter(([key]) => !key.includes("id") || key === "old_role" || key === "new_role")
                                    .map(([key, value]) => (
                                      <span key={key} className="mr-3">
                                        <span className="capitalize">
                                          {key.replace(/_/g, " ")}
                                        </span>
                                        : {String(value)}
                                      </span>
                                    ))}
                                </span>
                              )}
                            </TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>

                    {/* Pagination */}
                    {auditLogs.total_pages > 1 && (
                      <div className="flex items-center justify-between pt-4">
                        <div className="text-sm text-muted-foreground">
                          Showing {(auditLogs.page - 1) * auditLogs.per_page + 1} to{" "}
                          {Math.min(auditLogs.page * auditLogs.per_page, auditLogs.total)} of{" "}
                          {auditLogs.total} entries
                        </div>
                        <div className="flex items-center gap-2">
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => setAuditPage((p) => Math.max(1, p - 1))}
                            disabled={auditLogs.page <= 1}
                          >
                            <ChevronLeft className="h-4 w-4" />
                            Previous
                          </Button>
                          <span className="text-sm text-muted-foreground">
                            Page {auditLogs.page} of {auditLogs.total_pages}
                          </span>
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => setAuditPage((p) => Math.min(auditLogs.total_pages, p + 1))}
                            disabled={auditLogs.page >= auditLogs.total_pages}
                          >
                            Next
                            <ChevronRight className="h-4 w-4" />
                          </Button>
                        </div>
                      </div>
                    )}
                  </>
                )}
              </CardContent>
            </Card>
          </TabsContent>
        )}
      </Tabs>

      {/* Notification Channels - Only visible to admins */}
      {canManage && <TeamNotificationChannelsCard teamId={id!} />}

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
              Are you sure you want to change {pendingRoleChange?.member.user_name}'s
              role from <span className="font-medium capitalize">{pendingRoleChange?.member.role}</span> to{" "}
              <span className="font-medium capitalize">{pendingRoleChange?.newRole}</span>?
              {pendingRoleChange?.newRole === "owner" && (
                <span className="block mt-2 text-yellow-600">
                  Warning: This will give them full control over the team, including the ability to remove other owners.
                </span>
              )}
              {pendingRoleChange?.member.role === "admin" && pendingRoleChange?.newRole !== "owner" && pendingRoleChange?.newRole !== "admin" && (
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
              <Button
                disabled={updateRoleMutation.isPending}
                onClick={confirmRoleChange}
              >
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
