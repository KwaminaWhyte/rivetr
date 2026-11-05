//! Cron schedule parsing for backup schedules.
//!
//! Users type standard 5-field cron (`0 2 * * *`, weekday 0-7 with 0 and 7 = Sunday);
//! the `cron` crate wants a seconds field first and numbers weekdays 1-7 from Sunday.
//! 5-field expressions get `0 ` prepended and their numeric weekdays shifted.
//! 6/7-field expressions (already in the crate's format) and the `@daily`-style
//! shortcuts are passed through.

use std::str::FromStr;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use cron::Schedule;

/// Turn a user cron expression into the form the `cron` crate parses.
pub fn normalize_cron(expr: &str) -> Result<String> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("Cron expression is empty"));
    }
    if trimmed.starts_with('@') {
        return Ok(trimmed.to_string());
    }
    let fields = trimmed.split_whitespace().count();
    match fields {
        5 => {
            let mut parts: Vec<String> = trimmed.split_whitespace().map(str::to_string).collect();
            parts[4] = convert_weekday_field(&parts[4])?;
            Ok(format!("0 {}", parts.join(" ")))
        }
        6 | 7 => Ok(trimmed.split_whitespace().collect::<Vec<_>>().join(" ")),
        n => Err(anyhow!(
            "Cron expression must have 5 fields (minute hour day month weekday), got {n}"
        )),
    }
}

/// Map a standard weekday number (0-7, 0 and 7 = Sunday) to the crate's 1-7.
fn map_weekday(token: &str, range_end: bool) -> Result<String> {
    match token.parse::<u8>() {
        Ok(7) if range_end => Ok("7+SUN".to_string()),
        Ok(n @ 0..=7) => Ok(((n % 7) + 1).to_string()),
        Ok(n) => Err(anyhow!("Weekday {n} is out of range (0-7)")),
        // Names (MON, Sun, ...) and `*` are understood by the crate as they are.
        Err(_) => Ok(token.to_string()),
    }
}

/// Convert the weekday field of a standard cron expression for the `cron` crate.
fn convert_weekday_field(field: &str) -> Result<String> {
    let mut out: Vec<String> = Vec::new();
    for item in field.split(',') {
        let (base, step) = match item.split_once('/') {
            Some((b, s)) => (b, Some(s)),
            None => (item, None),
        };
        let converted = match base.split_once('-') {
            Some((start, end)) => {
                let start = map_weekday(start, false)?;
                let end = map_weekday(end, true)?;
                match end.strip_suffix("+SUN") {
                    // `a-7` means a through Sunday: a..Saturday plus Sunday.
                    Some(end) => {
                        out.push("1".to_string());
                        format!("{start}-{end}")
                    }
                    None => format!("{start}-{end}"),
                }
            }
            None => map_weekday(base, false)?,
        };
        out.push(match step {
            Some(step) => format!("{converted}/{step}"),
            None => converted,
        });
    }
    Ok(out.join(","))
}

/// Parse a user cron expression.
pub fn parse_cron(expr: &str) -> Result<Schedule> {
    let normalized = normalize_cron(expr)?;
    Schedule::from_str(&normalized).map_err(|e| anyhow!("Invalid cron expression '{expr}': {e}"))
}

/// Next run strictly after `after`.
pub fn next_run_after(expr: &str, after: DateTime<Utc>) -> Result<DateTime<Utc>> {
    parse_cron(expr)?
        .after(&after)
        .next()
        .ok_or_else(|| anyhow!("Cron expression '{expr}' has no upcoming run"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn five_field_cron_gets_seconds() {
        assert_eq!(normalize_cron("0 2 * * *").unwrap(), "0 0 2 * * *");
        assert_eq!(
            normalize_cron("  */15  * * * * ").unwrap(),
            "0 */15 * * * *"
        );
    }

    #[test]
    fn standard_weekdays_are_shifted_for_the_crate() {
        assert_eq!(normalize_cron("0 3 * * 0").unwrap(), "0 0 3 * * 1");
        assert_eq!(normalize_cron("0 3 * * 7").unwrap(), "0 0 3 * * 1");
        assert_eq!(normalize_cron("0 3 * * 1-5").unwrap(), "0 0 3 * * 2-6");
        assert_eq!(normalize_cron("0 3 * * 1,3").unwrap(), "0 0 3 * * 2,4");
        assert_eq!(normalize_cron("0 3 * * 5-7").unwrap(), "0 0 3 * * 1,6-7");
        assert_eq!(normalize_cron("0 3 * * MON").unwrap(), "0 0 3 * * MON");
        assert!(normalize_cron("0 3 * * 8").is_err());
    }

    #[test]
    fn standard_sunday_lands_on_sunday() {
        // 2026-11-05 is a Thursday; next Sunday 03:00 is the 8th.
        let base = Utc.with_ymd_and_hms(2026, 11, 5, 12, 0, 0).unwrap();
        let next = next_run_after("0 3 * * 0", base).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 11, 8, 3, 0, 0).unwrap());
        // Monday to Friday from Thursday noon: Friday 03:00.
        let next = next_run_after("0 3 * * 1-5", base).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 11, 6, 3, 0, 0).unwrap());
    }

    #[test]
    fn six_and_seven_fields_and_shortcuts_pass_through() {
        assert_eq!(normalize_cron("30 0 2 * * *").unwrap(), "30 0 2 * * *");
        assert_eq!(
            normalize_cron("0 0 2 * * * 2030").unwrap(),
            "0 0 2 * * * 2030"
        );
        assert_eq!(normalize_cron("@daily").unwrap(), "@daily");
    }

    #[test]
    fn bad_expressions_are_rejected() {
        assert!(normalize_cron("").is_err());
        assert!(normalize_cron("* * *").is_err());
        assert!(parse_cron("99 99 * * *").is_err());
        assert!(parse_cron("not a cron").is_err());
    }

    #[test]
    fn next_run_is_computed_from_five_field_cron() {
        let base = Utc.with_ymd_and_hms(2026, 11, 5, 1, 30, 0).unwrap();
        let next = next_run_after("0 2 * * *", base).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 11, 5, 2, 0, 0).unwrap());
        let after_that = next_run_after("0 2 * * *", next).unwrap();
        assert_eq!(
            after_that,
            Utc.with_ymd_and_hms(2026, 11, 6, 2, 0, 0).unwrap()
        );
    }

    #[test]
    fn weekly_cron_and_shortcut_compute_next_run() {
        // 2026-11-05 is a Thursday; next Sunday 03:00 is the 8th.
        let base = Utc.with_ymd_and_hms(2026, 11, 5, 12, 0, 0).unwrap();
        let next = next_run_after("0 3 * * Sun", base).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 11, 8, 3, 0, 0).unwrap());
        let daily = next_run_after("@daily", base).unwrap();
        assert_eq!(daily, Utc.with_ymd_and_hms(2026, 11, 6, 0, 0, 0).unwrap());
    }
}
