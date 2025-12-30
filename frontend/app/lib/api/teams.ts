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
  CreateTeamRequest,
  UpdateTeamRequest,
  InviteMemberRequest,
  UpdateMemberRoleRequest,
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
};
