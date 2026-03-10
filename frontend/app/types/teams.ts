// -------------------------------------------------------------------------
// Team types
// -------------------------------------------------------------------------

/** Team roles with hierarchical permissions */
export type TeamRole = "owner" | "admin" | "developer" | "viewer";

/** Team entity */
export interface Team {
  id: string;
  name: string;
  slug: string;
  created_at: string;
  updated_at: string;
}

/** Team with member count for list views */
export interface TeamWithMemberCount {
  id: string;
  name: string;
  slug: string;
  created_at: string;
  updated_at: string;
  member_count: number;
  /** Current user's role in this team (if applicable) */
  user_role: TeamRole | null;
}

/** Team member entity */
export interface TeamMember {
  id: string;
  team_id: string;
  user_id: string;
  role: TeamRole;
  created_at: string;
}

/** Team member with user details */
export interface TeamMemberWithUser {
  id: string;
  team_id: string;
  user_id: string;
  role: TeamRole;
  created_at: string;
  user_name: string;
  user_email: string;
}

/** Team detail response with members */
export interface TeamDetail {
  id: string;
  name: string;
  slug: string;
  created_at: string;
  updated_at: string;
  members: TeamMemberWithUser[];
  /** Current user's role in this team */
  user_role: TeamRole | null;
}

/** Request to create a new team */
export interface CreateTeamRequest {
  name: string;
  /** Optional slug (auto-generated from name if not provided) */
  slug?: string;
}

/** Request to update a team */
export interface UpdateTeamRequest {
  name?: string;
  slug?: string;
}

/** Request to invite/add a member to a team */
export interface InviteMemberRequest {
  /** User ID or email to invite */
  user_identifier: string;
  /** Role to assign */
  role: TeamRole;
}

/** Request to update a member's role */
export interface UpdateMemberRoleRequest {
  role: TeamRole;
}

// -------------------------------------------------------------------------
// Team Invitation types
// -------------------------------------------------------------------------

/** Team invitation entity */
export interface TeamInvitation {
  id: string;
  team_id: string;
  email: string;
  role: TeamRole;
  expires_at: string;
  accepted_at: string | null;
  created_by: string;
  created_at: string;
  /** Team name (for display purposes) */
  team_name: string | null;
  /** Inviter name (for display purposes) */
  inviter_name: string | null;
}

/** Request to create a team invitation */
export interface CreateInvitationRequest {
  /** Email address to invite */
  email: string;
  /** Role to assign */
  role: TeamRole;
}

// -------------------------------------------------------------------------
// Team Audit Log types
// -------------------------------------------------------------------------

/** Team audit action types */
export type TeamAuditAction =
  | "team_created"
  | "team_updated"
  | "team_deleted"
  | "member_invited"
  | "member_joined"
  | "member_removed"
  | "role_changed"
  | "invitation_created"
  | "invitation_revoked"
  | "invitation_accepted"
  | "invitation_resent"
  | "app_created"
  | "app_updated"
  | "app_deleted"
  | "project_created"
  | "project_updated"
  | "project_deleted"
  | "database_created"
  | "database_deleted"
  | "service_created"
  | "service_deleted"
  | "deployment_triggered"
  | "deployment_rolled_back";

/** Team audit resource types */
export type TeamAuditResourceType =
  | "team"
  | "member"
  | "invitation"
  | "app"
  | "project"
  | "database"
  | "service"
  | "deployment";

/** Team audit log entry */
export interface TeamAuditLog {
  id: string;
  team_id: string;
  user_id: string | null;
  action: TeamAuditAction;
  resource_type: TeamAuditResourceType;
  resource_id: string | null;
  details: Record<string, unknown> | null;
  created_at: string;
  /** User's name for display */
  user_name: string | null;
  /** User's email for display */
  user_email: string | null;
}

/** Paginated response for team audit logs */
export interface TeamAuditLogPage {
  items: TeamAuditLog[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}

/** Query parameters for fetching team audit logs */
export interface TeamAuditLogQuery {
  /** Filter by action type */
  action?: string;
  /** Filter by resource type */
  resource_type?: string;
  /** Start date for date range filter (ISO 8601) */
  start_date?: string;
  /** End date for date range filter (ISO 8601) */
  end_date?: string;
  /** Page number (1-indexed) */
  page?: number;
  /** Items per page (default 20, max 100) */
  per_page?: number;
}

// -------------------------------------------------------------------------
// Team Role Helper Functions
// -------------------------------------------------------------------------

/** Helper: Check if user has at least the required role */
export function hasRoleAtLeast(
  userRole: TeamRole | null,
  requiredRole: TeamRole,
): boolean {
  if (!userRole) return false;
  const roleOrder: TeamRole[] = ["viewer", "developer", "admin", "owner"];
  return roleOrder.indexOf(userRole) >= roleOrder.indexOf(requiredRole);
}

/** Helper: Check if user can manage team members */
export function canManageMembers(role: TeamRole | null): boolean {
  return hasRoleAtLeast(role, "admin");
}

/** Helper: Check if user can deploy apps */
export function canDeploy(role: TeamRole | null): boolean {
  return hasRoleAtLeast(role, "developer");
}

/** Helper: Check if user can manage apps (create/edit) */
export function canManageApps(role: TeamRole | null): boolean {
  return hasRoleAtLeast(role, "developer");
}

/** Helper: Check if user can delete apps */
export function canDeleteApps(role: TeamRole | null): boolean {
  return hasRoleAtLeast(role, "admin");
}

/** Helper: Check if user can delete the team */
export function canDeleteTeam(role: TeamRole | null): boolean {
  return role === "owner";
}
