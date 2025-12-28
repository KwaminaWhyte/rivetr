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
        matches!(self, TeamRole::Owner | TeamRole::Admin | TeamRole::Developer)
    }

    /// Check if the role can create/edit apps
    pub fn can_manage_apps(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin | TeamRole::Developer)
    }

    /// Check if the role can delete apps
    pub fn can_delete_apps(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin)
    }

    /// Check if the role can manage projects
    pub fn can_manage_projects(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin | TeamRole::Developer)
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
