//! Self-update settings (channel, check schedule, auto-update schedule).
//!
//! `rivetr.toml [auto_update]` provides the defaults; values saved from the
//! dashboard override them and are applied by the updater loop without a
//! restart.

use std::collections::HashMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::parse_bool;
use crate::config::AutoUpdateConfig;

const KEY_CHANNEL: &str = "update_channel";
const KEY_CHECK_ENABLED: &str = "update_check_enabled";
const KEY_CHECK_CRON: &str = "update_check_cron";
const KEY_AUTO_UPDATE: &str = "auto_update_enabled";
const KEY_AUTO_UPDATE_CRON: &str = "auto_update_cron";

/// Default time for automatic updates: every day at 00:00 UTC.
pub const DEFAULT_AUTO_UPDATE_CRON: &str = "0 0 * * *";

/// Release channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    Stable,
    /// Includes pre-releases.
    Beta,
}

/// Effective update settings.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UpdateSettings {
    pub channel: UpdateChannel,
    pub check_enabled: bool,
    /// 5-field cron (UTC) for update checks.
    pub check_cron: String,
    pub auto_update_enabled: bool,
    /// 5-field cron (UTC) for applying updates automatically.
    pub auto_update_cron: String,
}

/// Partial update from the dashboard.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct UpdateUpdateSettings {
    pub channel: Option<UpdateChannel>,
    pub check_enabled: Option<bool>,
    pub check_cron: Option<String>,
    pub auto_update_enabled: Option<bool>,
    pub auto_update_cron: Option<String>,
}

impl UpdateSettings {
    /// The toml-only settings (no dashboard overrides).
    pub fn from_toml(toml: &AutoUpdateConfig) -> Self {
        let hours = toml.check_interval_hours.clamp(1, 24);
        let check_cron = if hours >= 24 {
            "0 0 * * *".to_string()
        } else {
            format!("0 */{hours} * * *")
        };
        Self {
            channel: if toml.include_prereleases {
                UpdateChannel::Beta
            } else {
                UpdateChannel::Stable
            },
            check_enabled: toml.enabled,
            check_cron,
            auto_update_enabled: toml.auto_apply,
            auto_update_cron: DEFAULT_AUTO_UPDATE_CRON.to_string(),
        }
    }

    /// Merge: every key saved in the database wins over the toml default.
    pub fn effective(toml: &AutoUpdateConfig, map: &HashMap<String, String>) -> Self {
        let mut s = Self::from_toml(toml);
        match map.get(KEY_CHANNEL).map(String::as_str) {
            Some("beta") => s.channel = UpdateChannel::Beta,
            Some("stable") => s.channel = UpdateChannel::Stable,
            _ => {}
        }
        if let Some(v) = map.get(KEY_CHECK_ENABLED).and_then(|v| parse_bool(v)) {
            s.check_enabled = v;
        }
        if let Some(v) = map.get(KEY_CHECK_CRON).filter(|v| parse_cron(v).is_some()) {
            s.check_cron = v.clone();
        }
        if let Some(v) = map.get(KEY_AUTO_UPDATE).and_then(|v| parse_bool(v)) {
            s.auto_update_enabled = v;
        }
        if let Some(v) = map
            .get(KEY_AUTO_UPDATE_CRON)
            .filter(|v| parse_cron(v).is_some())
        {
            s.auto_update_cron = v.clone();
        }
        s
    }

    pub fn include_prereleases(&self) -> bool {
        self.channel == UpdateChannel::Beta
    }

    pub async fn load(db: &SqlitePool, toml: &AutoUpdateConfig) -> Result<Self, sqlx::Error> {
        let map = crate::db::load_setting_map(db).await?;
        Ok(Self::effective(toml, &map))
    }

    /// Validate and persist; returns the new effective settings.
    pub async fn save(
        db: &SqlitePool,
        toml: &AutoUpdateConfig,
        req: &UpdateUpdateSettings,
    ) -> Result<Self, String> {
        for (label, cron) in [
            ("Update check frequency", &req.check_cron),
            ("Auto update frequency", &req.auto_update_cron),
        ] {
            if let Some(c) = cron {
                if parse_cron(c).is_none() {
                    return Err(format!("{label}: '{c}' is not a valid cron expression"));
                }
            }
        }
        let mut writes: Vec<(&str, String)> = Vec::new();
        if let Some(c) = req.channel {
            writes.push((
                KEY_CHANNEL,
                match c {
                    UpdateChannel::Stable => "stable",
                    UpdateChannel::Beta => "beta",
                }
                .to_string(),
            ));
        }
        if let Some(v) = req.check_enabled {
            writes.push((KEY_CHECK_ENABLED, v.to_string()));
        }
        if let Some(v) = &req.check_cron {
            writes.push((KEY_CHECK_CRON, v.trim().to_string()));
        }
        if let Some(v) = req.auto_update_enabled {
            writes.push((KEY_AUTO_UPDATE, v.to_string()));
        }
        if let Some(v) = &req.auto_update_cron {
            writes.push((KEY_AUTO_UPDATE_CRON, v.trim().to_string()));
        }
        for (k, v) in writes {
            crate::db::set_setting_value(db, k, Some(&v))
                .await
                .map_err(|e| e.to_string())?;
        }
        Self::load(db, toml).await.map_err(|e| e.to_string())
    }
}

/// Parse a 5-field (minute-first) or 6/7-field (seconds-first) cron expression.
pub fn parse_cron(expr: &str) -> Option<cron::Schedule> {
    let expr = expr.trim();
    if expr.is_empty() {
        return None;
    }
    let fields: Vec<&str> = expr.split_whitespace().collect();
    let normalized = if fields.len() == 5 {
        // Classic crontab: weekday 0-7 with 0 and 7 = Sunday. The cron crate
        // numbers weekdays 1-7 from Sunday, so translate to names.
        format!(
            "0 {} {} {} {} {}",
            fields[0],
            fields[1],
            fields[2],
            fields[3],
            crontab_weekdays(fields[4])
        )
    } else {
        expr.to_string()
    };
    cron::Schedule::from_str(&normalized).ok()
}

/// Turn crontab weekday numbers into names (`1-5` -> `MON-FRI`), leaving
/// step values (`*/2`) and names alone.
fn crontab_weekdays(field: &str) -> String {
    const NAMES: [&str; 8] = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"];
    let map_num = |s: &str| -> String {
        match s.parse::<usize>() {
            Ok(n) if n < NAMES.len() => NAMES[n].to_string(),
            _ => s.to_string(),
        }
    };
    field
        .split(',')
        .map(|part| {
            let (range, step) = match part.split_once('/') {
                Some((r, s)) => (r, Some(s)),
                None => (part, None),
            };
            let range = range.split('-').map(map_num).collect::<Vec<_>>().join("-");
            match step {
                Some(s) => format!("{range}/{s}"),
                None => range,
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Did `expr` fire in the half-open window `(since, now]`?
pub fn cron_fired_between(
    expr: &str,
    since: chrono::DateTime<chrono::Utc>,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    parse_cron(expr)
        .and_then(|s| s.after(&since).next())
        .map(|next| next <= now)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn toml_is_the_default() {
        let toml = AutoUpdateConfig {
            include_prereleases: true,
            auto_apply: true,
            check_interval_hours: 12,
            ..Default::default()
        };
        let s = UpdateSettings::effective(&toml, &HashMap::new());
        assert_eq!(s.channel, UpdateChannel::Beta);
        assert!(s.auto_update_enabled);
        assert_eq!(s.check_cron, "0 */12 * * *");
        assert_eq!(s.auto_update_cron, DEFAULT_AUTO_UPDATE_CRON);
    }

    #[test]
    fn database_overrides_toml() {
        let toml = AutoUpdateConfig {
            include_prereleases: true,
            auto_apply: true,
            ..Default::default()
        };
        let s = UpdateSettings::effective(
            &toml,
            &map(&[
                ("update_channel", "stable"),
                ("auto_update_enabled", "false"),
                ("update_check_cron", "*/30 * * * *"),
                ("auto_update_cron", "0 3 * * 0"),
                ("update_check_enabled", "false"),
            ]),
        );
        assert_eq!(s.channel, UpdateChannel::Stable);
        assert!(!s.auto_update_enabled);
        assert!(!s.check_enabled);
        assert_eq!(s.check_cron, "*/30 * * * *");
        assert_eq!(s.auto_update_cron, "0 3 * * 0");
    }

    #[test]
    fn invalid_saved_cron_falls_back_to_toml() {
        let toml = AutoUpdateConfig::default();
        let s = UpdateSettings::effective(&toml, &map(&[("update_check_cron", "whenever")]));
        assert_eq!(s.check_cron, "0 */6 * * *");
    }

    #[test]
    fn cron_window() {
        let since = chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 30).unwrap();
        let now = chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 1, 30).unwrap();
        assert!(cron_fired_between("* * * * *", since, now));
        assert!(!cron_fired_between("0 3 * * *", since, now));
        assert!(!cron_fired_between("garbage", since, now));
    }

    #[test]
    fn crontab_weekday_numbers_are_accepted() {
        assert!(parse_cron("0 3 * * 0").is_some());
        assert!(parse_cron("0 3 * * 7").is_some());
        assert!(parse_cron("0 3 * * 1-5").is_some());
        assert!(parse_cron("0 3 * * */2").is_some());
        assert_eq!(crontab_weekdays("0,6"), "SUN,SAT");
        assert_eq!(crontab_weekdays("1-5"), "MON-FRI");
        // 2026-01-04 is a Sunday.
        let since = chrono::Utc.with_ymd_and_hms(2026, 1, 4, 2, 59, 30).unwrap();
        let now = chrono::Utc.with_ymd_and_hms(2026, 1, 4, 3, 0, 30).unwrap();
        assert!(cron_fired_between("0 3 * * 0", since, now));
        assert!(!cron_fired_between("0 3 * * 1", since, now));
    }
}
