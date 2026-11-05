//! Advanced instance settings: registration, destructive-action confirmation,
//! API access policy, MCP endpoint, DNS validation and the outbound policy.
//!
//! Values live in the `instance_settings` key/value table. A process-wide
//! cache (`current()`) is loaded at startup and refreshed on every save, so hot
//! paths such as the auth middleware and the SSRF guard never hit the DB.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, LazyLock};

use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::net::{self, IpCidr};
use super::{parse_bool, KEY_PUBLIC_IPV4, KEY_PUBLIC_IPV6};

const KEY_REGISTRATION: &str = "registration_enabled";
const KEY_PASSWORD_REGISTRATION: &str = "allow_password_registration";
const KEY_CONFIRMATION: &str = "destructive_confirmation";
const KEY_API_ACCESS: &str = "api_access_enabled";
const KEY_API_ALLOWED_IPS: &str = "api_allowed_ips";
const KEY_MCP: &str = "mcp_enabled";
const KEY_DNS_VALIDATION: &str = "dns_validation_enabled";
const KEY_DNS_SERVERS: &str = "dns_servers";
const KEY_OUTBOUND_TARGETS: &str = "outbound_allowed_targets";
const KEY_OUTBOUND_LOCALHOST: &str = "outbound_allow_localhost";
const KEY_PASSKEYS: &str = "passkeys_enabled";

/// How destructive actions are confirmed in the dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationMode {
    /// A single "Are you sure?" dialog.
    #[default]
    Simple,
    /// The user must type the resource name before the action is enabled.
    TwoStep,
}

impl ConfirmationMode {
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "simple" => Some(Self::Simple),
            "two_step" | "two-step" => Some(Self::TwoStep),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::TwoStep => "two_step",
        }
    }
}

/// Effective Advanced settings (defaults applied).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AdvancedSettings {
    pub registration_enabled: bool,
    pub allow_password_registration: bool,
    pub destructive_confirmation: ConfirmationMode,
    pub api_access_enabled: bool,
    /// IPs / CIDRs allowed to use token authentication. Empty = every source.
    pub api_allowed_ips: Vec<String>,
    pub mcp_enabled: bool,
    pub dns_validation_enabled: bool,
    /// Nameservers used for domain checks. Empty = the OS resolver.
    pub dns_servers: Vec<String>,
    /// Internal CIDRs that outbound requests (webhooks, log drains) may reach.
    pub outbound_allowed_targets: Vec<String>,
    pub outbound_allow_localhost: bool,
    /// Users may register passkeys and sign in with them.
    pub passkeys_enabled: bool,
}

impl Default for AdvancedSettings {
    fn default() -> Self {
        Self {
            registration_enabled: false,
            allow_password_registration: true,
            destructive_confirmation: ConfirmationMode::Simple,
            api_access_enabled: true,
            api_allowed_ips: Vec::new(),
            // Migration 123 seeds `true` for installs that already had users.
            mcp_enabled: false,
            dns_validation_enabled: true,
            dns_servers: Vec::new(),
            outbound_allowed_targets: Vec::new(),
            outbound_allow_localhost: false,
            passkeys_enabled: true,
        }
    }
}

/// Partial update; every field is optional.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct UpdateAdvancedSettings {
    pub registration_enabled: Option<bool>,
    pub allow_password_registration: Option<bool>,
    pub destructive_confirmation: Option<String>,
    pub api_access_enabled: Option<bool>,
    pub api_allowed_ips: Option<Vec<String>>,
    pub mcp_enabled: Option<bool>,
    pub dns_validation_enabled: Option<bool>,
    pub dns_servers: Option<Vec<String>>,
    pub outbound_allowed_targets: Option<Vec<String>>,
    pub outbound_allow_localhost: Option<bool>,
    pub passkeys_enabled: Option<bool>,
}

/// Pre-parsed form kept in the cache so request paths do no string parsing.
#[derive(Debug, Clone, Default)]
pub struct CachedAdvanced {
    pub settings: AdvancedSettings,
    pub api_allowlist: Vec<IpCidr>,
    pub outbound_allowlist: Vec<IpCidr>,
}

impl CachedAdvanced {
    fn new(settings: AdvancedSettings) -> Self {
        let api_allowlist = settings
            .api_allowed_ips
            .iter()
            .filter_map(|s| IpCidr::parse(s))
            .collect();
        let outbound_allowlist = settings
            .outbound_allowed_targets
            .iter()
            .filter_map(|s| IpCidr::parse(s))
            .collect();
        Self {
            settings,
            api_allowlist,
            outbound_allowlist,
        }
    }
}

static CURRENT: LazyLock<ArcSwap<CachedAdvanced>> =
    LazyLock::new(|| ArcSwap::from_pointee(CachedAdvanced::default()));

/// The cached settings. Defaults until `reload` has run once.
pub fn current() -> Arc<CachedAdvanced> {
    CURRENT.load_full()
}

/// Reload the cache from the database.
pub async fn reload(db: &SqlitePool) -> Result<AdvancedSettings, sqlx::Error> {
    let map = crate::db::load_setting_map(db).await?;
    let settings = AdvancedSettings::from_map(&map);
    CURRENT.store(Arc::new(CachedAdvanced::new(settings.clone())));
    Ok(settings)
}

impl AdvancedSettings {
    /// Build from the raw key/value map, applying defaults for missing keys.
    pub fn from_map(map: &HashMap<String, String>) -> Self {
        let d = Self::default();
        let b =
            |key: &str, default: bool| map.get(key).and_then(|v| parse_bool(v)).unwrap_or(default);
        let list = |key: &str| map.get(key).map(|v| net::split_list(v)).unwrap_or_default();
        Self {
            registration_enabled: b(KEY_REGISTRATION, d.registration_enabled),
            allow_password_registration: b(
                KEY_PASSWORD_REGISTRATION,
                d.allow_password_registration,
            ),
            destructive_confirmation: map
                .get(KEY_CONFIRMATION)
                .and_then(|v| ConfirmationMode::parse(v))
                .unwrap_or_default(),
            api_access_enabled: b(KEY_API_ACCESS, d.api_access_enabled),
            api_allowed_ips: list(KEY_API_ALLOWED_IPS),
            mcp_enabled: b(KEY_MCP, d.mcp_enabled),
            dns_validation_enabled: b(KEY_DNS_VALIDATION, d.dns_validation_enabled),
            dns_servers: list(KEY_DNS_SERVERS),
            outbound_allowed_targets: list(KEY_OUTBOUND_TARGETS),
            outbound_allow_localhost: b(KEY_OUTBOUND_LOCALHOST, d.outbound_allow_localhost),
            passkeys_enabled: b(KEY_PASSKEYS, d.passkeys_enabled),
        }
    }

    /// Validate and persist an update, then refresh the cache.
    pub async fn save(
        db: &SqlitePool,
        req: &UpdateAdvancedSettings,
    ) -> Result<AdvancedSettings, SaveError> {
        let writes = validate_update(req)?;
        for (key, value) in &writes {
            crate::db::set_setting_value(db, key, Some(value)).await?;
        }
        Ok(reload(db).await?)
    }
}

/// Validate an update and turn it into `(key, value)` writes.
pub fn validate_update(
    req: &UpdateAdvancedSettings,
) -> Result<Vec<(&'static str, String)>, SaveError> {
    let mut out: Vec<(&'static str, String)> = Vec::new();
    let mut bool_field = |key: &'static str, v: Option<bool>| {
        if let Some(v) = v {
            out.push((key, v.to_string()));
        }
    };
    bool_field(KEY_REGISTRATION, req.registration_enabled);
    bool_field(KEY_PASSWORD_REGISTRATION, req.allow_password_registration);
    bool_field(KEY_API_ACCESS, req.api_access_enabled);
    bool_field(KEY_MCP, req.mcp_enabled);
    bool_field(KEY_DNS_VALIDATION, req.dns_validation_enabled);
    bool_field(KEY_OUTBOUND_LOCALHOST, req.outbound_allow_localhost);
    bool_field(KEY_PASSKEYS, req.passkeys_enabled);

    if let Some(mode) = &req.destructive_confirmation {
        let mode = ConfirmationMode::parse(mode).ok_or_else(|| {
            SaveError::Invalid("destructive_confirmation must be 'simple' or 'two_step'".into())
        })?;
        out.push((KEY_CONFIRMATION, mode.as_str().to_string()));
    }

    let cidr_list = |key: &'static str, label: &str, v: &Option<Vec<String>>, out: &mut Vec<_>| {
        if let Some(entries) = v {
            let entries: Vec<String> = entries.iter().flat_map(|e| net::split_list(e)).collect();
            net::parse_cidrs(&entries).map_err(|bad| {
                SaveError::Invalid(format!(
                    "{label}: '{bad}' is not a valid IP address or CIDR"
                ))
            })?;
            out.push((key, entries.join("\n")));
        }
        Ok::<(), SaveError>(())
    };
    cidr_list(
        KEY_API_ALLOWED_IPS,
        "Allowed IPs",
        &req.api_allowed_ips,
        &mut out,
    )?;
    cidr_list(
        KEY_OUTBOUND_TARGETS,
        "Allowed internal targets",
        &req.outbound_allowed_targets,
        &mut out,
    )?;

    if let Some(servers) = &req.dns_servers {
        let servers: Vec<String> = servers.iter().flat_map(|e| net::split_list(e)).collect();
        for s in &servers {
            if s.parse::<IpAddr>().is_err() {
                return Err(SaveError::Invalid(format!(
                    "DNS servers: '{s}' is not a valid IP address"
                )));
            }
        }
        out.push((KEY_DNS_SERVERS, servers.join("\n")));
    }

    Ok(out)
}

/// Error from `AdvancedSettings::save`.
#[derive(Debug)]
pub enum SaveError {
    Invalid(String),
    Db(sqlx::Error),
}

impl From<sqlx::Error> for SaveError {
    fn from(e: sqlx::Error) -> Self {
        Self::Db(e)
    }
}

// ---------------------------------------------------------------------------
// Policy decisions (pure functions, unit tested)
// ---------------------------------------------------------------------------

/// Why a token-authenticated API request was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiAccessDenied {
    Disabled,
    IpNotAllowed,
}

/// Decide whether a token-authenticated (API token / admin token) request may
/// proceed. Dashboard sessions never go through this check.
pub fn check_api_access(
    cached: &CachedAdvanced,
    client_ip: Option<IpAddr>,
) -> Result<(), ApiAccessDenied> {
    if !cached.settings.api_access_enabled {
        return Err(ApiAccessDenied::Disabled);
    }
    if cached.api_allowlist.is_empty() {
        return Ok(());
    }
    match client_ip {
        Some(ip) if net::ip_allowed(&cached.api_allowlist, &ip) => Ok(()),
        _ => Err(ApiAccessDenied::IpNotAllowed),
    }
}

/// How a new account is being created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignupMethod {
    /// Email + password sign up form.
    Password,
    /// First login through an OAuth or OIDC/SSO provider.
    External,
}

/// May a new account be created? The very first account (instance setup) is
/// always allowed. Invitations bypass this check entirely.
///
/// `login_providers_enabled` is true when any OAuth or SSO login provider is on;
/// in that case password sign up additionally needs `allow_password_registration`.
pub fn registration_allowed(
    settings: &AdvancedSettings,
    existing_users: i64,
    method: SignupMethod,
    login_providers_enabled: bool,
) -> bool {
    if existing_users == 0 {
        return true;
    }
    if !settings.registration_enabled {
        return false;
    }
    match method {
        SignupMethod::External => true,
        SignupMethod::Password => !login_providers_enabled || settings.allow_password_registration,
    }
}

/// Is any OAuth or OIDC login provider enabled?
pub async fn login_providers_enabled(db: &SqlitePool) -> bool {
    let oauth: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM oauth_providers WHERE enabled = 1")
        .fetch_one(db)
        .await
        .unwrap_or(0);
    if oauth > 0 {
        return true;
    }
    let oidc: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM oidc_providers WHERE enabled = 1")
        .fetch_one(db)
        .await
        .unwrap_or(0);
    oidc > 0
}

/// DB-backed registration check used by the sign up, OAuth and SSO handlers.
pub async fn registration_allowed_now(db: &SqlitePool, method: SignupMethod) -> bool {
    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(db)
        .await
        .unwrap_or(1);
    let map = crate::db::load_setting_map(db).await.unwrap_or_default();
    let settings = AdvancedSettings::from_map(&map);
    let providers = login_providers_enabled(db).await;
    registration_allowed(&settings, users, method, providers)
}

/// The admin-set public IPs (General settings), if any.
pub async fn public_ips(db: &SqlitePool) -> (Option<String>, Option<String>) {
    let map = crate::db::load_setting_map(db).await.unwrap_or_default();
    (
        map.get(KEY_PUBLIC_IPV4).filter(|v| !v.is_empty()).cloned(),
        map.get(KEY_PUBLIC_IPV6).filter(|v| !v.is_empty()).cloned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn defaults_when_nothing_saved() {
        let s = AdvancedSettings::from_map(&HashMap::new());
        assert_eq!(s, AdvancedSettings::default());
        assert!(s.api_access_enabled);
        assert!(!s.mcp_enabled);
        assert!(!s.registration_enabled);
    }

    #[test]
    fn saved_values_override_defaults() {
        let s = AdvancedSettings::from_map(&map(&[
            ("mcp_enabled", "true"),
            ("api_access_enabled", "false"),
            ("destructive_confirmation", "two_step"),
            ("api_allowed_ips", "10.0.0.0/8\n1.2.3.4"),
            ("dns_servers", "1.1.1.1, 8.8.8.8"),
        ]));
        assert!(s.mcp_enabled);
        assert!(!s.api_access_enabled);
        assert_eq!(s.destructive_confirmation, ConfirmationMode::TwoStep);
        assert_eq!(s.api_allowed_ips, vec!["10.0.0.0/8", "1.2.3.4"]);
        assert_eq!(s.dns_servers, vec!["1.1.1.1", "8.8.8.8"]);
    }

    #[test]
    fn garbage_values_fall_back_to_defaults() {
        let s = AdvancedSettings::from_map(&map(&[
            ("api_access_enabled", "maybe"),
            ("destructive_confirmation", "triple"),
        ]));
        assert!(s.api_access_enabled);
        assert_eq!(s.destructive_confirmation, ConfirmationMode::Simple);
    }

    #[test]
    fn api_access_disabled_blocks_tokens() {
        let cached = CachedAdvanced::new(AdvancedSettings {
            api_access_enabled: false,
            ..Default::default()
        });
        assert_eq!(
            check_api_access(&cached, Some("1.1.1.1".parse().unwrap())),
            Err(ApiAccessDenied::Disabled)
        );
    }

    #[test]
    fn api_allowlist_is_enforced() {
        let cached = CachedAdvanced::new(AdvancedSettings {
            api_allowed_ips: vec!["10.0.0.0/8".into()],
            ..Default::default()
        });
        assert!(check_api_access(&cached, Some("10.9.9.9".parse().unwrap())).is_ok());
        assert_eq!(
            check_api_access(&cached, Some("8.8.8.8".parse().unwrap())),
            Err(ApiAccessDenied::IpNotAllowed)
        );
        assert_eq!(
            check_api_access(&cached, None),
            Err(ApiAccessDenied::IpNotAllowed)
        );
    }

    #[test]
    fn open_allowlist_allows_any_source() {
        let cached = CachedAdvanced::new(AdvancedSettings::default());
        assert!(check_api_access(&cached, None).is_ok());
        assert!(check_api_access(&cached, Some("8.8.8.8".parse().unwrap())).is_ok());
    }

    #[test]
    fn first_user_can_always_register() {
        let s = AdvancedSettings::default();
        assert!(registration_allowed(&s, 0, SignupMethod::Password, true));
        assert!(registration_allowed(&s, 0, SignupMethod::External, false));
    }

    #[test]
    fn registration_disabled_blocks_everyone_after_setup() {
        let s = AdvancedSettings::default();
        assert!(!registration_allowed(&s, 3, SignupMethod::Password, false));
        assert!(!registration_allowed(&s, 3, SignupMethod::External, true));
    }

    #[test]
    fn password_registration_respects_oauth_toggle() {
        let s = AdvancedSettings {
            registration_enabled: true,
            allow_password_registration: false,
            ..Default::default()
        };
        // No login providers: the password form is the only way in, so allow it.
        assert!(registration_allowed(&s, 1, SignupMethod::Password, false));
        // Providers enabled and password sign up turned off.
        assert!(!registration_allowed(&s, 1, SignupMethod::Password, true));
        assert!(registration_allowed(&s, 1, SignupMethod::External, true));

        let s = AdvancedSettings {
            allow_password_registration: true,
            ..s
        };
        assert!(registration_allowed(&s, 1, SignupMethod::Password, true));
    }

    #[test]
    fn update_validation_rejects_bad_entries() {
        let bad_ip = UpdateAdvancedSettings {
            api_allowed_ips: Some(vec!["10.0.0.0/8".into(), "nope".into()]),
            ..Default::default()
        };
        assert!(matches!(
            validate_update(&bad_ip),
            Err(SaveError::Invalid(_))
        ));

        let bad_dns = UpdateAdvancedSettings {
            dns_servers: Some(vec!["10.0.0.0/8".into()]),
            ..Default::default()
        };
        assert!(matches!(
            validate_update(&bad_dns),
            Err(SaveError::Invalid(_))
        ));

        let bad_mode = UpdateAdvancedSettings {
            destructive_confirmation: Some("twice".into()),
            ..Default::default()
        };
        assert!(matches!(
            validate_update(&bad_mode),
            Err(SaveError::Invalid(_))
        ));
    }

    #[test]
    fn update_validation_normalizes_lists() {
        let req = UpdateAdvancedSettings {
            api_allowed_ips: Some(vec!["10.0.0.0/8, 1.2.3.4".into()]),
            destructive_confirmation: Some("two-step".into()),
            ..Default::default()
        };
        let writes = validate_update(&req).unwrap();
        assert!(writes.contains(&(KEY_API_ALLOWED_IPS, "10.0.0.0/8\n1.2.3.4".to_string())));
        assert!(writes.contains(&(KEY_CONFIRMATION, "two_step".to_string())));
    }

    #[tokio::test]
    async fn migration_seeds_and_registration_gate_against_db() {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::db::init(dir.path()).await.unwrap();

        // Fresh install: MCP off, registration off, first user may still sign up.
        let map = crate::db::load_setting_map(&pool).await.unwrap();
        let s = AdvancedSettings::from_map(&map);
        assert!(!s.mcp_enabled);
        assert!(!s.registration_enabled);
        assert!(registration_allowed_now(&pool, SignupMethod::Password).await);

        sqlx::query(
            "INSERT INTO users (id, email, password_hash, name, role) VALUES ('u1', 'a@b.c', 'x', 'A', 'admin')",
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(!registration_allowed_now(&pool, SignupMethod::Password).await);
        assert!(!registration_allowed_now(&pool, SignupMethod::External).await);

        AdvancedSettings::save(
            &pool,
            &UpdateAdvancedSettings {
                registration_enabled: Some(true),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert!(registration_allowed_now(&pool, SignupMethod::Password).await);
        assert!(registration_allowed_now(&pool, SignupMethod::External).await);
    }
}
