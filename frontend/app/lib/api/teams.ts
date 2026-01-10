/**
 * Teams API module.
 * Handles team management and membership operations.
 */

import { apiRequest } from "./core";
import type {
  Team,
  TeamDetail,
  TeamWithMemberCount,
  TeamMemberWithUser,
  TeamInvitation,
  CreateTeamRequest,
  UpdateTeamRequest,
  InviteMemberRequest,
  UpdateMemberRoleRequest,
  CreateInvitationRequest,
  TeamAuditLogPage,
  TeamAuditLogQuery,
} from "@/types/api";

export const teamsApi = {
  // -------------------------------------------------------------------------
  // Team CRUD
  // -------------------------------------------------------------------------

  /** List all teams the user has access to */
  getTeams: (token?: string) =>
    apiRequest<TeamWithMemberCount[]>("/teams", {}, token),

  /** Get a single team with full details */
  getTeam: (id: string, token?: string) =>
    apiRequest<TeamDetail>(`/teams/${id}`, {}, token),

  /** Create a new team */
  createTeam: (data: CreateTeamRequest, token?: string) =>
    apiRequest<Team>(
      "/teams",
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Update an existing team */
  updateTeam: (id: string, data: UpdateTeamRequest, token?: string) =>
    apiRequest<Team>(
      `/teams/${id}`,
      {
        method: "PUT",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Delete a team */
  deleteTeam: (id: string, token?: string) =>
    apiRequest<void>(
      `/teams/${id}`,
      {
        method: "DELETE",
      },
      token
    ),

  // -------------------------------------------------------------------------
  // Team Membership
  // -------------------------------------------------------------------------

  /** Get all members of a team */
  getTeamMembers: (teamId: string, token?: string) =>
    apiRequest<TeamMemberWithUser[]>(`/teams/${teamId}/members`, {}, token),

  /** Invite a user to a team */
  inviteTeamMember: (teamId: string, data: InviteMemberRequest, token?: string) =>
    apiRequest<TeamMemberWithUser>(
      `/teams/${teamId}/members`,
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Update a team member's role */
  updateTeamMemberRole: (
    teamId: string,
    userId: string,
    data: UpdateMemberRoleRequest,
    token?: string
  ) =>
    apiRequest<TeamMemberWithUser>(
      `/teams/${teamId}/members/${userId}`,
      {
        method: "PUT",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Remove a member from a team */
  removeTeamMember: (teamId: string, userId: string, token?: string) =>
    apiRequest<void>(
      `/teams/${teamId}/members/${userId}`,
      {
        method: "DELETE",
      },
      token
    ),

  // -------------------------------------------------------------------------
  // Team Invitations
  // -------------------------------------------------------------------------

  /** List pending invitations for a team */
  getTeamInvitations: (teamId: string, token?: string) =>
    apiRequest<TeamInvitation[]>(`/teams/${teamId}/invitations`, {}, token),

  /** Create a new invitation */
  createTeamInvitation: (
    teamId: string,
    data: CreateInvitationRequest,
    token?: string
  ) =>
    apiRequest<TeamInvitation>(
      `/teams/${teamId}/invitations`,
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Revoke/delete a pending invitation */
  deleteTeamInvitation: (teamId: string, invitationId: string, token?: string) =>
    apiRequest<void>(
      `/teams/${teamId}/invitations/${invitationId}`,
      {
        method: "DELETE",
      },
      token
    ),

  /** Resend an invitation email */
  resendTeamInvitation: (teamId: string, invitationId: string, token?: string) =>
    apiRequest<void>(
      `/teams/${teamId}/invitations/${invitationId}/resend`,
      {
        method: "POST",
      },
      token
    ),

  // -------------------------------------------------------------------------
  // Public Invitation Operations
  // -------------------------------------------------------------------------

  /** Validate an invitation token (public endpoint) */
  validateInvitation: (invitationToken: string) =>
    apiRequest<TeamInvitation>(`/auth/invitations/${invitationToken}`, {}),

  /** Accept an invitation (requires authentication) */
  acceptInvitation: (invitationToken: string, token?: string) =>
    apiRequest<TeamMemberWithUser>(
      `/invitations/${invitationToken}/accept`,
      {
        method: "POST",
      },
      token
    ),

  // -------------------------------------------------------------------------
  // Team Audit Logs
  // -------------------------------------------------------------------------

  /** Get paginated audit logs for a team */
  getTeamAuditLogs: (
    teamId: string,
    query?: TeamAuditLogQuery,
    token?: string
  ) => {
    const params = new URLSearchParams();
    if (query?.action) params.set("action", query.action);
    if (query?.resource_type) params.set("resource_type", query.resource_type);
    if (query?.start_date) params.set("start_date", query.start_date);
    if (query?.end_date) params.set("end_date", query.end_date);
    if (query?.page) params.set("page", query.page.toString());
    if (query?.per_page) params.set("per_page", query.per_page.toString());

    const queryString = params.toString();
    const url = `/teams/${teamId}/audit-logs${queryString ? `?${queryString}` : ""}`;
    return apiRequest<TeamAuditLogPage>(url, {}, token);
  },
};
