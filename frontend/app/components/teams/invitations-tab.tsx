import { useState } from "react";
import { Loader2, Mail, MailPlus, Clock, RefreshCw, Trash2, Copy, Check } from "lucide-react";
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
  Select,
  SelectContent,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { TeamInvitation, TeamRole, TeamRolesResponse } from "@/types/api";
import { RoleIcon, RoleSelectItems, roleBadgeVariant, roleName } from "@/components/teams/role-utils";

interface InvitationsTabProps {
  pendingInvitations: TeamInvitation[];
  currentUserRole: TeamRole | null;
  roles?: TeamRolesResponse;
  isLoadingInvitations: boolean;
  isResending: boolean;
  isRevoking: boolean;
  isCreatingInvitation: boolean;
  onCreateInvitation: (email: string, role: TeamRole) => void;
  onResend: (invitationId: string) => void;
  onRevoke: (invitation: TeamInvitation) => void;
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString();
}

function formatRelativeTime(dateStr: string): string {
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = date.getTime() - now.getTime();
  const diffDays = Math.ceil(diffMs / (1000 * 60 * 60 * 24));

  if (diffDays < 0) return "Expired";
  if (diffDays === 0) return "Expires today";
  if (diffDays === 1) return "Expires tomorrow";
  return `Expires in ${diffDays} days`;
}

/**
 * Copy text on both secure and plain-HTTP dashboards. `navigator.clipboard`
 * only exists in a secure context, so a dashboard served over HTTP (an IP or
 * a bare host on port 8000) needs the textarea fallback or the button does
 * nothing at all.
 */
async function copyText(text: string): Promise<boolean> {
  if (window.isSecureContext && navigator.clipboard) {
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch {
      // Permission denied or blocked: fall through to the legacy path.
    }
  }

  const area = document.createElement("textarea");
  area.value = text;
  area.setAttribute("readonly", "");
  area.style.position = "fixed";
  area.style.top = "-1000px";
  area.style.opacity = "0";
  document.body.appendChild(area);
  area.select();
  area.setSelectionRange(0, text.length);

  let ok = false;
  try {
    ok = document.execCommand("copy");
  } catch {
    ok = false;
  }
  document.body.removeChild(area);
  return ok;
}

export function InvitationsTab({
  pendingInvitations,
  currentUserRole,
  roles,
  isLoadingInvitations,
  isResending,
  isRevoking,
  isCreatingInvitation,
  onCreateInvitation,
  onResend,
  onRevoke,
}: InvitationsTabProps) {
  const [inviteEmail, setInviteEmail] = useState("");
  const [inviteRole, setInviteRole] = useState<TeamRole>("developer");
  const [copiedId, setCopiedId] = useState<string | null>(null);

  const handleCopyLink = async (invitation: TeamInvitation) => {
    const url = `${window.location.origin}/invitations/accept?token=${encodeURIComponent(invitation.token)}`;
    if (await copyText(url)) {
      setCopiedId(invitation.id);
      setTimeout(() => setCopiedId(null), 2000);
    } else {
      toast.error("Could not copy the link. Copy it from here instead.", {
        description: url,
      });
    }
  };

  const handleCreateInvitation = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!inviteEmail?.trim()) return;
    onCreateInvitation(inviteEmail.trim(), inviteRole);
    setInviteEmail("");
    setInviteRole("developer");
  };

  return (
    <>
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
                  <SelectValue>{roleName(inviteRole, roles)}</SelectValue>
                </SelectTrigger>
                <SelectContent>
                  <RoleSelectItems roles={roles} currentUserRole={currentUserRole} />
                </SelectContent>
              </Select>
            </div>
            <Button type="submit" disabled={isCreatingInvitation}>
              {isCreatingInvitation ? (
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
                  <TableHead className="w-44">Actions</TableHead>
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
                        <RoleIcon role={invitation.role} />
                        <Badge variant={roleBadgeVariant(invitation.role)}>
                          {roleName(invitation.role, roles)}
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
                      <div className="flex items-center gap-1">
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => handleCopyLink(invitation)}
                          title="Copy invitation link"
                        >
                          {copiedId === invitation.id ? (
                            <Check className="h-4 w-4 text-green-500" />
                          ) : (
                            <Copy className="h-4 w-4" />
                          )}
                        </Button>
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => onResend(invitation.id)}
                          disabled={isResending}
                          title="Resend invitation email"
                        >
                          <RefreshCw
                            className={`h-4 w-4 ${isResending ? "animate-spin" : ""}`}
                          />
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => onRevoke(invitation)}
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
    </>
  );
}
