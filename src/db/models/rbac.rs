//! Granular team RBAC: fine-grained abilities, the built-in role presets and
//! custom per-team roles.
//!
//! A team member's `role` column holds either a preset key (`owner`, `admin`,
//! `developer`, `deployer`, `viewer`) or `custom:<team_roles.id>`. The four
//! pre-1.1 role values are presets with the same names, so existing rows keep
//! their meaning without a data migration. Every authorization decision is made
//! from the ability set a role resolves to (see `crate::api::authz`).
//!
//! Per-resource allow/deny overrides (`team_resource_permissions`) are applied on
//! top of the role abilities by [`effective_allows`].

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A fine-grained team ability. The string form (`apps.deploy`) is what the API
/// and the `team_roles.abilities` JSON column use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Ability {
    #[serde(rename = "apps.read")]
    AppsRead,
    #[serde(rename = "apps.deploy")]
    AppsDeploy,
    #[serde(rename = "apps.write")]
    AppsWrite,
    #[serde(rename = "apps.delete")]
    AppsDelete,
    #[serde(rename = "services.read")]
    ServicesRead,
    #[serde(rename = "services.deploy")]
    ServicesDeploy,
    #[serde(rename = "services.write")]
    ServicesWrite,
    #[serde(rename = "services.delete")]
    ServicesDelete,
    #[serde(rename = "databases.read")]
    DatabasesRead,
    #[serde(rename = "databases.deploy")]
    DatabasesDeploy,
    #[serde(rename = "databases.write")]
    DatabasesWrite,
    #[serde(rename = "databases.delete")]
    DatabasesDelete,
    #[serde(rename = "projects.read")]
    ProjectsRead,
    #[serde(rename = "projects.write")]
    ProjectsWrite,
    #[serde(rename = "projects.delete")]
    ProjectsDelete,
    #[serde(rename = "env.read_secrets")]
    EnvReadSecrets,
    #[serde(rename = "servers.read")]
    ServersRead,
    #[serde(rename = "servers.manage")]
    ServersManage,
    #[serde(rename = "integrations.manage")]
    IntegrationsManage,
    #[serde(rename = "team.manage")]
    TeamManage,
    #[serde(rename = "team.audit")]
    TeamAudit,
    #[serde(rename = "team.delete")]
    TeamDelete,
    #[serde(rename = "settings.manage")]
    SettingsManage,
    #[serde(rename = "tokens.manage")]
    TokensManage,
}

/// Static description of one ability, for the role editor.
#[derive(Debug, Clone, Serialize)]
pub struct AbilityInfo {
    pub key: &'static str,
    pub area: &'static str,
    pub label: &'static str,
    pub description: &'static str,
}

impl Ability {
    /// Every ability, in display order.
    pub const ALL: [Ability; 24] = [
        Ability::AppsRead,
        Ability::AppsDeploy,
        Ability::AppsWrite,
        Ability::AppsDelete,
        Ability::ServicesRead,
        Ability::ServicesDeploy,
        Ability::ServicesWrite,
        Ability::ServicesDelete,
        Ability::DatabasesRead,
        Ability::DatabasesDeploy,
        Ability::DatabasesWrite,
        Ability::DatabasesDelete,
        Ability::ProjectsRead,
        Ability::ProjectsWrite,
        Ability::ProjectsDelete,
        Ability::EnvReadSecrets,
        Ability::ServersRead,
        Ability::ServersManage,
        Ability::IntegrationsManage,
        Ability::TeamManage,
        Ability::TeamAudit,
        Ability::TeamDelete,
        Ability::SettingsManage,
        Ability::TokensManage,
    ];

    pub fn as_str(self) -> &'static str {
        self.info().key
    }

    pub fn parse(s: &str) -> Option<Ability> {
        Ability::ALL.into_iter().find(|a| a.as_str() == s)
    }

    pub fn info(self) -> AbilityInfo {
        let (key, area, label, description) = match self {
            Ability::AppsRead => (
                "apps.read",
                "Apps",
                "View",
                "See apps, deployments, logs and metrics",
            ),
            Ability::AppsDeploy => (
                "apps.deploy",
                "Apps",
                "Deploy",
                "Deploy, start, stop, restart, roll back and cancel",
            ),
            Ability::AppsWrite => (
                "apps.write",
                "Apps",
                "Edit",
                "Create apps and change settings, env vars and domains",
            ),
            Ability::AppsDelete => (
                "apps.delete",
                "Apps",
                "Delete",
                "Delete apps and their child resources",
            ),
            Ability::ServicesRead => (
                "services.read",
                "Services",
                "View",
                "See Compose services, logs and metrics",
            ),
            Ability::ServicesDeploy => (
                "services.deploy",
                "Services",
                "Deploy",
                "Deploy, start, stop, restart and roll back",
            ),
            Ability::ServicesWrite => (
                "services.write",
                "Services",
                "Edit",
                "Create services and change their configuration",
            ),
            Ability::ServicesDelete => ("services.delete", "Services", "Delete", "Delete services"),
            Ability::DatabasesRead => (
                "databases.read",
                "Databases",
                "View",
                "See databases, backups and logs",
            ),
            Ability::DatabasesDeploy => (
                "databases.deploy",
                "Databases",
                "Start/stop",
                "Start, stop and restart databases",
            ),
            Ability::DatabasesWrite => (
                "databases.write",
                "Databases",
                "Edit",
                "Create databases, change settings, back up and restore",
            ),
            Ability::DatabasesDelete => (
                "databases.delete",
                "Databases",
                "Delete",
                "Delete databases and backups",
            ),
            Ability::ProjectsRead => (
                "projects.read",
                "Projects",
                "View",
                "See projects and environments",
            ),
            Ability::ProjectsWrite => (
                "projects.write",
                "Projects",
                "Edit",
                "Create and change projects, environments and shared variables",
            ),
            Ability::ProjectsDelete => (
                "projects.delete",
                "Projects",
                "Delete",
                "Delete projects and environments",
            ),
            Ability::EnvReadSecrets => (
                "env.read_secrets",
                "Secrets",
                "Reveal secrets",
                "Reveal secret env var values and download backups",
            ),
            Ability::ServersRead => (
                "servers.read",
                "Servers",
                "View",
                "See team servers and their health",
            ),
            Ability::ServersManage => (
                "servers.manage",
                "Servers",
                "Manage",
                "Add, change, maintain and remove team servers",
            ),
            Ability::IntegrationsManage => (
                "integrations.manage",
                "Integrations",
                "Manage",
                "Tunnels, GitHub Apps, SSH keys, S3, destinations, build servers",
            ),
            Ability::TeamManage => (
                "team.manage",
                "Team",
                "Manage members",
                "Invite, remove and change members, roles and resource overrides",
            ),
            Ability::TeamAudit => ("team.audit", "Team", "Audit log", "Read the team audit log"),
            Ability::TeamDelete => (
                "team.delete",
                "Team",
                "Delete team",
                "Delete the team and change its 2FA requirement",
            ),
            Ability::SettingsManage => (
                "settings.manage",
                "Team",
                "Settings",
                "Rename the team and manage its notification channels",
            ),
            Ability::TokensManage => (
                "tokens.manage",
                "Team",
                "API tokens",
                "Create API tokens restricted to this team",
            ),
        };
        AbilityInfo {
            key,
            area,
            label,
            description,
        }
    }
}

/// A set of abilities.
pub type AbilitySet = BTreeSet<Ability>;

/// A built-in role preset.
#[derive(Debug, Clone, Serialize)]
pub struct RolePreset {
    pub key: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub abilities: Vec<Ability>,
}

/// Preset keys in display order (most to least privileged).
pub const PRESET_KEYS: [&str; 5] = ["owner", "admin", "developer", "deployer", "viewer"];

/// Prefix of a custom role reference in `team_members.role`.
pub const CUSTOM_ROLE_PREFIX: &str = "custom:";

fn reads() -> [Ability; 5] {
    [
        Ability::AppsRead,
        Ability::ServicesRead,
        Ability::DatabasesRead,
        Ability::ProjectsRead,
        Ability::ServersRead,
    ]
}

/// Abilities of a preset, or `None` for an unknown key.
pub fn preset_abilities(key: &str) -> Option<AbilitySet> {
    use Ability::*;
    let set: AbilitySet = match key {
        "owner" => Ability::ALL.into_iter().collect(),
        "admin" => Ability::ALL
            .into_iter()
            .filter(|a| *a != TeamDelete)
            .collect(),
        // Matches what the pre-1.1 developer role could do: everything on
        // resources (including delete, which the old write check allowed) but no
        // team administration.
        "developer" => reads()
            .into_iter()
            .chain([
                AppsDeploy,
                AppsWrite,
                AppsDelete,
                ServicesDeploy,
                ServicesWrite,
                ServicesDelete,
                DatabasesDeploy,
                DatabasesWrite,
                DatabasesDelete,
                ProjectsWrite,
                ProjectsDelete,
                EnvReadSecrets,
                ServersManage,
                IntegrationsManage,
                TokensManage,
            ])
            .collect(),
        "deployer" => reads()
            .into_iter()
            .chain([AppsDeploy, ServicesDeploy, DatabasesDeploy, TokensManage])
            .collect(),
        "viewer" => reads().into_iter().collect(),
        _ => return None,
    };
    Some(set)
}

/// All presets with descriptions, for the role picker.
pub fn presets() -> Vec<RolePreset> {
    PRESET_KEYS
        .iter()
        .map(|key| {
            let (name, description) = match *key {
                "owner" => (
                    "Owner",
                    "Everything, including deleting the team and managing owners",
                ),
                "admin" => (
                    "Admin",
                    "Everything except deleting the team; manages members and roles",
                ),
                "developer" => (
                    "Developer",
                    "Create, change, deploy and delete resources; no team administration",
                ),
                "deployer" => (
                    "Deployer",
                    "Read everything and deploy, start, stop and roll back; no edits or deletes",
                ),
                _ => (
                    "Viewer",
                    "Read-only access to resources, deployments and logs",
                ),
            };
            RolePreset {
                key,
                name,
                description,
                abilities: preset_abilities(key)
                    .unwrap_or_default()
                    .into_iter()
                    .collect(),
            }
        })
        .collect()
}

/// Is this role value a preset key?
pub fn is_preset(role: &str) -> bool {
    PRESET_KEYS.contains(&role)
}

/// The custom role id referenced by a role value, if it is a custom role.
pub fn custom_role_id(role: &str) -> Option<&str> {
    role.strip_prefix(CUSTOM_ROLE_PREFIX)
        .filter(|id| !id.is_empty())
}

/// Map a stored role value to a preset key. Legacy values are the preset keys
/// themselves; anything unknown falls back to the least privileged preset, which
/// is what the old `TeamRole::from` did.
pub fn legacy_role_to_preset(role: &str) -> &'static str {
    let lower = role.to_ascii_lowercase();
    PRESET_KEYS
        .iter()
        .find(|k| **k == lower)
        .copied()
        .unwrap_or("viewer")
}

/// Parse a JSON array of ability keys, ignoring unknown entries.
pub fn parse_abilities_json(json: &str) -> AbilitySet {
    serde_json::from_str::<Vec<String>>(json)
        .unwrap_or_default()
        .iter()
        .filter_map(|s| Ability::parse(s))
        .collect()
}

/// Encode an ability set as the JSON array stored in `team_roles.abilities`.
pub fn abilities_to_json(set: &AbilitySet) -> String {
    serde_json::to_string(&set.iter().map(|a| a.as_str()).collect::<Vec<_>>())
        .unwrap_or_else(|_| "[]".into())
}

/// Rank used for the "who may manage whom" rule: only owners manage owners.
pub fn role_is_owner(role: &str) -> bool {
    role.eq_ignore_ascii_case("owner")
}

/// Resource area an ability check applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbilityArea {
    Apps,
    Services,
    Databases,
    Projects,
    Servers,
    Integrations,
}

/// A per-resource override from `team_resource_permissions`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceOverride {
    /// No access to the resource at all, whatever the role.
    Deny,
    /// Read, deploy and edit the resource even when the role does not allow it.
    /// Deleting and revealing secrets still follow the role.
    Allow,
}

impl ResourceOverride {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "deny" => Some(Self::Deny),
            "allow" => Some(Self::Allow),
            _ => None,
        }
    }
}

/// Final decision for one ability on one resource: role abilities combined with
/// the most specific override (resource override first, else the project's).
pub fn effective_allows(
    role_abilities: &AbilitySet,
    over: Option<ResourceOverride>,
    needed: Ability,
) -> bool {
    match over {
        Some(ResourceOverride::Deny) => false,
        Some(ResourceOverride::Allow) if allow_override_grants(needed) => true,
        _ => role_abilities.contains(&needed),
    }
}

fn allow_override_grants(a: Ability) -> bool {
    use Ability::*;
    matches!(
        a,
        AppsRead
            | AppsDeploy
            | AppsWrite
            | ServicesRead
            | ServicesDeploy
            | ServicesWrite
            | DatabasesRead
            | DatabasesDeploy
            | DatabasesWrite
            | ProjectsRead
            | ProjectsWrite
    )
}

/// A custom team role row.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TeamRoleRow {
    pub id: String,
    pub team_id: String,
    pub name: String,
    pub description: Option<String>,
    pub abilities: String,
    pub created_at: String,
    pub updated_at: String,
}

/// A custom role as returned by the API.
#[derive(Debug, Clone, Serialize)]
pub struct CustomRoleResponse {
    pub id: String,
    /// Value to store in a member's `role`: `custom:<id>`.
    pub key: String,
    pub team_id: String,
    pub name: String,
    pub description: Option<String>,
    pub abilities: Vec<String>,
    pub member_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Create / update payload for a custom role.
#[derive(Debug, Deserialize)]
pub struct CustomRoleRequest {
    pub name: String,
    pub description: Option<String>,
    pub abilities: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ability_strings_round_trip() {
        for a in Ability::ALL {
            assert_eq!(Ability::parse(a.as_str()), Some(a));
            let json = serde_json::to_string(&a).unwrap();
            assert_eq!(json, format!("\"{}\"", a.as_str()));
        }
        assert_eq!(Ability::parse("apps.fly"), None);
    }

    #[test]
    fn legacy_roles_map_to_presets_with_the_same_power() {
        assert_eq!(legacy_role_to_preset("owner"), "owner");
        assert_eq!(legacy_role_to_preset("Admin"), "admin");
        assert_eq!(legacy_role_to_preset("developer"), "developer");
        assert_eq!(legacy_role_to_preset("viewer"), "viewer");
        assert_eq!(legacy_role_to_preset("garbage"), "viewer");

        let owner = preset_abilities("owner").unwrap();
        let admin = preset_abilities("admin").unwrap();
        let dev = preset_abilities("developer").unwrap();
        let viewer = preset_abilities("viewer").unwrap();
        // Owner can do everything; admin all but deleting the team.
        assert_eq!(owner.len(), Ability::ALL.len());
        assert!(!admin.contains(&Ability::TeamDelete));
        assert!(admin.contains(&Ability::TeamManage));
        // Old developer: write, deploy and delete resources, no team admin.
        for a in [Ability::AppsWrite, Ability::AppsDeploy, Ability::AppsDelete] {
            assert!(dev.contains(&a));
        }
        assert!(!dev.contains(&Ability::TeamManage));
        // Old viewer: read only.
        assert!(viewer.iter().all(|a| a.as_str().ends_with(".read")));
    }

    #[test]
    fn viewer_cannot_deploy_and_deployer_cannot_delete() {
        let viewer = preset_abilities("viewer").unwrap();
        let deployer = preset_abilities("deployer").unwrap();
        assert!(!effective_allows(&viewer, None, Ability::AppsDeploy));
        assert!(effective_allows(&deployer, None, Ability::AppsDeploy));
        assert!(!effective_allows(&deployer, None, Ability::AppsDelete));
        assert!(!effective_allows(&deployer, None, Ability::AppsWrite));
        assert!(!effective_allows(&deployer, None, Ability::DatabasesDelete));
    }

    #[test]
    fn custom_role_abilities_combine_with_overrides() {
        let custom = parse_abilities_json(r#"["apps.read","apps.deploy","bogus"]"#);
        assert_eq!(custom.len(), 2);
        assert!(effective_allows(&custom, None, Ability::AppsDeploy));
        assert!(!effective_allows(&custom, None, Ability::AppsWrite));
        // Deny beats the role.
        assert!(!effective_allows(
            &custom,
            Some(ResourceOverride::Deny),
            Ability::AppsRead
        ));
        // Allow grants edit on that one resource, but never delete or secrets.
        assert!(effective_allows(
            &custom,
            Some(ResourceOverride::Allow),
            Ability::AppsWrite
        ));
        assert!(!effective_allows(
            &custom,
            Some(ResourceOverride::Allow),
            Ability::AppsDelete
        ));
        assert!(!effective_allows(
            &custom,
            Some(ResourceOverride::Allow),
            Ability::EnvReadSecrets
        ));
        assert_eq!(abilities_to_json(&custom), r#"["apps.read","apps.deploy"]"#);
    }

    #[test]
    fn custom_role_reference_parsing() {
        assert_eq!(custom_role_id("custom:abc"), Some("abc"));
        assert_eq!(custom_role_id("custom:"), None);
        assert_eq!(custom_role_id("developer"), None);
        assert!(is_preset("deployer"));
        assert!(!is_preset("custom:abc"));
    }
}
