import {
  Crown,
  Shield,
  Code,
  Eye,
  Trash2,
  Users,
  UserPlus,
  Settings2,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/ui/card";
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
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { TeamMemberWithUser, TeamRole } from "@/types/api";

interface MembersTabProps {
  members: TeamMemberWithUser[];
  currentUserRole: TeamRole | null;
  canManage: boolean;
  isUpdatingRole: boolean;
  onShowInviteDialog: () => void;
  onRoleChange: (member: TeamMemberWithUser, newRole: TeamRole) => void;
  onRemoveMember: (member: TeamMemberWithUser) => void;
  onManagePermissions?: (member: TeamMemberWithUser) => void;
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString();
}

function getRoleBadgeVariant(role: TeamRole): "default" | "secondary" | "outline" {
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

const ROLE_OPTIONS: { value: TeamRole; label: string; description: string }[] = [
  { value: "owner", label: "Owner", description: "Full access, can delete team" },
  { value: "admin", label: "Admin", description: "Manage apps, projects, and members" },
  { value: "developer", label: "Developer", description: "Create/edit apps, deploy, view logs" },
  { value: "viewer", label: "Viewer", description: "Read-only access" },
];

export function MembersTab({
  members,
  currentUserRole,
  canManage,
  isUpdatingRole,
  onShowInviteDialog,
  onRoleChange,
  onRemoveMember,
  onManagePermissions,
}: MembersTabProps) {
  return (
    <>
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
              <Button onClick={onShowInviteDialog}>
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
              {members.map((member) => (
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
                        {(currentUserRole === "owner" && member.role !== "owner") ||
                        (currentUserRole === "admin" &&
                          (member.role === "developer" || member.role === "viewer")) ? (
                          <Select
                            value={member.role}
                            onValueChange={(value) =>
                              onRoleChange(member, value as TeamRole)
                            }
                            disabled={isUpdatingRole}
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
                        {onManagePermissions && canManage && (
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => onManagePermissions(member)}
                            title="Manage resource permissions"
                          >
                            <Settings2 className="h-4 w-4" />
                          </Button>
                        )}
                        {(currentUserRole === "owner" ||
                          (currentUserRole === "admin" &&
                            (member.role === "developer" || member.role === "viewer"))) && (
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => onRemoveMember(member)}
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
              <div
                key={role.value}
                className="flex items-start gap-3 p-3 rounded-lg border"
              >
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
    </>
  );
}
