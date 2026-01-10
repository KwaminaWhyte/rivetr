//! Team and role-based access control models.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Team roles with hierarchical permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TeamRole {
    /// Full access, can delete team, manage all members
    Owner,
    /// Manage apps, projects, members (except owners), deploy
    Admin,
    /// Create/edit apps, deploy, view logs
    Developer,
    /// Read-only access to apps, deployments, logs
    Viewer,
}

impl TeamRole {
    /// Check if this role has at least the specified permission level
    pub fn has_at_least(&self, required: TeamRole) -> bool {
        self.level() >= required.level()
    }

    /// Get the permission level (higher = more permissions)
    pub fn level(&self) -> u8 {
        match self {
            TeamRole::Owner => 4,
            TeamRole::Admin => 3,
            TeamRole::Developer => 2,
            TeamRole::Viewer => 1,
        }
    }

    /// Check if the role can manage team members
    pub fn can_manage_members(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin)
    }

    /// Check if the role can manage members of the given role
    pub fn can_manage_member_role(&self, target_role: TeamRole) -> bool {
        match self {
            TeamRole::Owner => true,
            TeamRole::Admin => !matches!(target_role, TeamRole::Owner),
            _ => false,
        }
    }

    /// Check if the role can deploy apps
    pub fn can_deploy(&self) -> bool {
        matches!(
            self,
            TeamRole::Owner | TeamRole::Admin | TeamRole::Developer
        )
    }

    /// Check if the role can create/edit apps
    pub fn can_manage_apps(&self) -> bool {
        matches!(
            self,
            TeamRole::Owner | TeamRole::Admin | TeamRole::Developer
        )
    }

    /// Check if the role can delete apps
    pub fn can_delete_apps(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin)
    }

    /// Check if the role can manage projects
    pub fn can_manage_projects(&self) -> bool {
        matches!(
            self,
            TeamRole::Owner | TeamRole::Admin | TeamRole::Developer
        )
    }

    /// Check if the role can delete projects
    pub fn can_delete_projects(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin)
    }

    /// Check if the role can delete the team
    pub fn can_delete_team(&self) -> bool {
        matches!(self, TeamRole::Owner)
    }

    /// Check if the role can view resources (all roles can view)
    pub fn can_view(&self) -> bool {
        true
    }
}

impl std::fmt::Display for TeamRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TeamRole::Owner => write!(f, "owner"),
            TeamRole::Admin => write!(f, "admin"),
            TeamRole::Developer => write!(f, "developer"),
            TeamRole::Viewer => write!(f, "viewer"),
        }
    }
}

impl std::str::FromStr for TeamRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "owner" => Ok(TeamRole::Owner),
            "admin" => Ok(TeamRole::Admin),
            "developer" => Ok(TeamRole::Developer),
            "viewer" => Ok(TeamRole::Viewer),
            _ => Err(format!("Unknown team role: {}", s)),
        }
    }
}

impl From<String> for TeamRole {
    fn from(s: String) -> Self {
        s.parse().unwrap_or(TeamRole::Viewer)
    }
}

/// Team entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Team member entity linking users to teams with roles
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TeamMember {
    pub id: String,
    pub team_id: String,
    pub user_id: String,
    pub role: String,
    pub created_at: String,
}

impl TeamMember {
    /// Get the role as a TeamRole enum
    pub fn role_enum(&self) -> TeamRole {
        TeamRole::from(self.role.clone())
    }
}

/// Team with member count for list views
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamWithMemberCount {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub created_at: String,
    pub updated_at: String,
    pub member_count: i64,
    /// Current user's role in this team (if applicable)
    pub user_role: Option<String>,
}

/// Team member with user details
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TeamMemberWithUser {
    pub id: String,
    pub team_id: String,
    pub user_id: String,
    pub role: String,
    pub created_at: String,
    /// User's name
    pub user_name: String,
    /// User's email
    pub user_email: String,
}

/// Team detail response with members
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamDetail {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub created_at: String,
    pub updated_at: String,
    pub members: Vec<TeamMemberWithUser>,
}

/// Request to create a new team
#[derive(Debug, Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
    /// Optional slug (auto-generated from name if not provided)
    pub slug: Option<String>,
}

/// Request to update a team
#[derive(Debug, Deserialize)]
pub struct UpdateTeamRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
}

/// Request to invite/add a member to a team
#[derive(Debug, Deserialize)]
pub struct InviteMemberRequest {
    /// User ID or email to invite
    pub user_identifier: String,
    /// Role to assign
    pub role: String,
}

/// Request to update a member's role
#[derive(Debug, Deserialize)]
pub struct UpdateMemberRoleRequest {
    pub role: String,
}

/// Team invitation entity for email-based invitations
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TeamInvitation {
    pub id: String,
    pub team_id: String,
    pub email: String,
    pub role: String,
    pub token: String,
    pub expires_at: String,
    pub accepted_at: Option<String>,
    pub created_by: String,
    pub created_at: String,
}

impl TeamInvitation {
    /// Check if the invitation has expired
    pub fn is_expired(&self) -> bool {
        if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(&self.expires_at) {
            expires < chrono::Utc::now()
        } else {
            true // Treat parse errors as expired
        }
    }

    /// Check if the invitation has been accepted
    pub fn is_accepted(&self) -> bool {
        self.accepted_at.is_some()
    }
}

/// Team invitation response for API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamInvitationResponse {
    pub id: String,
    pub team_id: String,
    pub email: String,
    pub role: String,
    pub expires_at: String,
    pub accepted_at: Option<String>,
    pub created_by: String,
    pub created_at: String,
    /// Team name (for display purposes)
    pub team_name: Option<String>,
    /// Inviter name (for display purposes)
    pub inviter_name: Option<String>,
}

impl From<TeamInvitation> for TeamInvitationResponse {
    fn from(inv: TeamInvitation) -> Self {
        Self {
            id: inv.id,
            team_id: inv.team_id,
            email: inv.email,
            role: inv.role,
            expires_at: inv.expires_at,
            accepted_at: inv.accepted_at,
            created_by: inv.created_by,
            created_at: inv.created_at,
            team_name: None,
            inviter_name: None,
        }
    }
}

/// Request to create a team invitation
#[derive(Debug, Deserialize)]
pub struct CreateInvitationRequest {
    /// Email address to invite
    pub email: String,
    /// Role to assign (owner, admin, developer, viewer)
    pub role: String,
}

/// Team audit log action types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamAuditAction {
    // Team operations
    TeamCreated,
    TeamUpdated,
    TeamDeleted,

    // Member operations
    MemberInvited,
    MemberJoined,
    MemberRemoved,
    RoleChanged,

    // Invitation operations
    InvitationCreated,
    InvitationRevoked,
    InvitationAccepted,
    InvitationResent,

    // App operations
    AppCreated,
    AppUpdated,
    AppDeleted,

    // Project operations
    ProjectCreated,
    ProjectUpdated,
    ProjectDeleted,

    // Database operations
    DatabaseCreated,
    DatabaseDeleted,

    // Service operations
    ServiceCreated,
    ServiceDeleted,

    // Deployment operations
    DeploymentTriggered,
    DeploymentRolledBack,
}

impl std::fmt::Display for TeamAuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TeamAuditAction::TeamCreated => "team_created",
            TeamAuditAction::TeamUpdated => "team_updated",
            TeamAuditAction::TeamDeleted => "team_deleted",
            TeamAuditAction::MemberInvited => "member_invited",
            TeamAuditAction::MemberJoined => "member_joined",
            TeamAuditAction::MemberRemoved => "member_removed",
            TeamAuditAction::RoleChanged => "role_changed",
            TeamAuditAction::InvitationCreated => "invitation_created",
            TeamAuditAction::InvitationRevoked => "invitation_revoked",
            TeamAuditAction::InvitationAccepted => "invitation_accepted",
            TeamAuditAction::InvitationResent => "invitation_resent",
            TeamAuditAction::AppCreated => "app_created",
            TeamAuditAction::AppUpdated => "app_updated",
            TeamAuditAction::AppDeleted => "app_deleted",
            TeamAuditAction::ProjectCreated => "project_created",
            TeamAuditAction::ProjectUpdated => "project_updated",
            TeamAuditAction::ProjectDeleted => "project_deleted",
            TeamAuditAction::DatabaseCreated => "database_created",
            TeamAuditAction::DatabaseDeleted => "database_deleted",
            TeamAuditAction::ServiceCreated => "service_created",
            TeamAuditAction::ServiceDeleted => "service_deleted",
            TeamAuditAction::DeploymentTriggered => "deployment_triggered",
            TeamAuditAction::DeploymentRolledBack => "deployment_rolled_back",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for TeamAuditAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "team_created" => Ok(TeamAuditAction::TeamCreated),
            "team_updated" => Ok(TeamAuditAction::TeamUpdated),
            "team_deleted" => Ok(TeamAuditAction::TeamDeleted),
            "member_invited" => Ok(TeamAuditAction::MemberInvited),
            "member_joined" => Ok(TeamAuditAction::MemberJoined),
            "member_removed" => Ok(TeamAuditAction::MemberRemoved),
            "role_changed" => Ok(TeamAuditAction::RoleChanged),
            "invitation_created" => Ok(TeamAuditAction::InvitationCreated),
            "invitation_revoked" => Ok(TeamAuditAction::InvitationRevoked),
            "invitation_accepted" => Ok(TeamAuditAction::InvitationAccepted),
            "invitation_resent" => Ok(TeamAuditAction::InvitationResent),
            "app_created" => Ok(TeamAuditAction::AppCreated),
            "app_updated" => Ok(TeamAuditAction::AppUpdated),
            "app_deleted" => Ok(TeamAuditAction::AppDeleted),
            "project_created" => Ok(TeamAuditAction::ProjectCreated),
            "project_updated" => Ok(TeamAuditAction::ProjectUpdated),
            "project_deleted" => Ok(TeamAuditAction::ProjectDeleted),
            "database_created" => Ok(TeamAuditAction::DatabaseCreated),
            "database_deleted" => Ok(TeamAuditAction::DatabaseDeleted),
            "service_created" => Ok(TeamAuditAction::ServiceCreated),
            "service_deleted" => Ok(TeamAuditAction::ServiceDeleted),
            "deployment_triggered" => Ok(TeamAuditAction::DeploymentTriggered),
            "deployment_rolled_back" => Ok(TeamAuditAction::DeploymentRolledBack),
            _ => Err(format!("Unknown audit action: {}", s)),
        }
    }
}

/// Resource types for audit logs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamAuditResourceType {
    Team,
    Member,
    Invitation,
    App,
    Project,
    Database,
    Service,
    Deployment,
}

impl std::fmt::Display for TeamAuditResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TeamAuditResourceType::Team => "team",
            TeamAuditResourceType::Member => "member",
            TeamAuditResourceType::Invitation => "invitation",
            TeamAuditResourceType::App => "app",
            TeamAuditResourceType::Project => "project",
            TeamAuditResourceType::Database => "database",
            TeamAuditResourceType::Service => "service",
            TeamAuditResourceType::Deployment => "deployment",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for TeamAuditResourceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "team" => Ok(TeamAuditResourceType::Team),
            "member" => Ok(TeamAuditResourceType::Member),
            "invitation" => Ok(TeamAuditResourceType::Invitation),
            "app" => Ok(TeamAuditResourceType::App),
            "project" => Ok(TeamAuditResourceType::Project),
            "database" => Ok(TeamAuditResourceType::Database),
            "service" => Ok(TeamAuditResourceType::Service),
            "deployment" => Ok(TeamAuditResourceType::Deployment),
            _ => Err(format!("Unknown resource type: {}", s)),
        }
    }
}

/// Team audit log entry
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TeamAuditLog {
    pub id: String,
    pub team_id: String,
    pub user_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: Option<String>,
    pub created_at: String,
}

/// Team audit log response for API with user details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamAuditLogResponse {
    pub id: String,
    pub team_id: String,
    pub user_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: Option<serde_json::Value>,
    pub created_at: String,
    /// User's name for display
    pub user_name: Option<String>,
    /// User's email for display
    pub user_email: Option<String>,
}

/// Paginated response for audit logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamAuditLogPage {
    pub items: Vec<TeamAuditLogResponse>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
    pub total_pages: i32,
}
