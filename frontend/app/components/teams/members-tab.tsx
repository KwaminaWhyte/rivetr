import { Link } from "react-router";
import { Trash2, Users, UserPlus, Settings2 } from "lucide-react";
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
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { TeamMemberWithUser, TeamRole, TeamRolesResponse } from "@/types/api";
import { RoleIcon, RoleSelectItems, roleBadgeVariant, roleName } from "@/components/teams/role-utils";

interface MembersTabProps {
  members: TeamMemberWithUser[];
  currentUserRole: TeamRole | null;
  /** Roles of the team (presets and custom roles). */
  roles?: TeamRolesResponse;
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

export function MembersTab({
  members,
  currentUserRole,
  roles,
  canManage,
  isUpdatingRole,
  onShowInviteDialog,
  onRoleChange,
  onRemoveMember,
  onManagePermissions,
}: MembersTabProps) {
  // Only owners touch owners; the backend also refuses roles with abilities
  // the caller does not have.
  const canEdit = (member: TeamMemberWithUser) =>
    canManage && (currentUserRole === "owner" || member.role !== "owner");

  return (
    <>
      <Card>
        <CardHeader>
          <div className="flex flex-wrap items-center justify-between gap-3">
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
        <CardContent className="overflow-x-auto">
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
                      <RoleIcon role={member.role} />
                      <Badge variant={roleBadgeVariant(member.role)}>
                        {roleName(member.role, roles)}
                      </Badge>
                    </div>
                  </TableCell>
                  <TableCell>{formatDate(member.created_at)}</TableCell>
                  {canManage && (
                    <TableCell>
                      <div className="flex items-center gap-2">
                        {canEdit(member) ? (
                          <Select
                            value={member.role}
                            onValueChange={(value) => onRoleChange(member, value as TeamRole)}
                            disabled={isUpdatingRole}
                          >
                            <SelectTrigger aria-label="Member role" className="w-36 h-8">
                              <SelectValue>{roleName(member.role, roles)}</SelectValue>
                            </SelectTrigger>
                            <SelectContent>
                              <RoleSelectItems roles={roles} currentUserRole={currentUserRole} />
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
                        {canEdit(member) && (
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
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <CardTitle>Roles</CardTitle>
              <CardDescription>
                Built-in presets. Resource overrides (the gear icon) adjust a member's access to
                single resources.
              </CardDescription>
            </div>
            <Button variant="outline" size="sm" asChild>
              <Link to="/team/roles">Custom roles</Link>
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {(roles?.presets ?? []).map((role) => (
              <div key={role.key} className="flex items-start gap-3 p-3 rounded-lg border">
                <div className="mt-0.5">
                  <RoleIcon role={role.key} />
                </div>
                <div>
                  <div className="font-medium">{role.name}</div>
                  <div className="text-sm text-muted-foreground">{role.description}</div>
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    </>
  );
}
