//! Missed scheduled backups.
//!
//! A schedule's `next_run_at` is the run it owes. When that time is more than the
//! grace window in the past and the schedule has not run since, the run was missed
//! (scheduler disabled, server down at the time, the job hanging, ...). Each missed
//! run is reported once: `backup_missed_alerts` remembers `(schedule, expected_at)`.

use chrono::{DateTime, Duration, Utc};

use super::notify::{send_backup_notification, BackupEvent, BackupOutcome, BackupResourceKind};
use crate::DbPool;

/// How late a run may be before it counts as missed (on top of the scheduler's own
/// check interval).
pub const MISSED_GRACE_MINUTES: i64 = 15;

/// Which table a schedule lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleKind {
    /// `database_backup_schedules`
    Database,
    /// `volume_backup_schedules`
    Volume,
    /// `backup_schedules` (instance and S3 schedules)
    Instance,
}

impl ScheduleKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Database => "database",
            Self::Volume => "volume",
            Self::Instance => "instance",
        }
    }
}

/// The parts of a schedule the check needs.
#[derive(Debug, Clone)]
pub struct ScheduleSnapshot {
    pub kind: ScheduleKind,
    pub schedule_id: String,
    pub resource_kind: BackupResourceKind,
    pub resource_id: Option<String>,
    pub resource_name: String,
    pub app_id: Option<String>,
    pub next_run_at: Option<String>,
    pub last_run_at: Option<String>,
}

/// A run that should have happened and did not.
#[derive(Debug, Clone)]
pub struct MissedRun {
    pub schedule: ScheduleSnapshot,
    pub expected_at: DateTime<Utc>,
}

/// Parse an RFC 3339 or SQLite `YYYY-MM-DD HH:MM:SS` (UTC) timestamp.
pub fn parse_timestamp(s: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .ok()
        .map(|n| n.and_utc())
}

/// True when the run due at `expected` is overdue by more than `grace` at `now` and
/// the schedule has not run since it was due.
pub fn is_missed(
    expected: DateTime<Utc>,
    last_run: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    grace: Duration,
) -> bool {
    if now <= expected + grace {
        return false;
    }
    !matches!(last_run, Some(last) if last >= expected)
}

/// Enabled schedules whose owed run is missed. Schedules without a parsable
/// `next_run_at` are ignored.
pub fn find_missed(
    schedules: Vec<ScheduleSnapshot>,
    now: DateTime<Utc>,
    grace: Duration,
) -> Vec<MissedRun> {
    schedules
        .into_iter()
        .filter_map(|s| {
            let expected = parse_timestamp(s.next_run_at.as_deref()?)?;
            let last = s.last_run_at.as_deref().and_then(parse_timestamp);
            is_missed(expected, last, now, grace).then_some(MissedRun {
                schedule: s,
                expected_at: expected,
            })
        })
        .collect()
}

/// Grace window used by the periodic check: fixed slack plus the slowest scheduler
/// interval, so a run that is merely waiting for the next tick is not reported.
pub fn grace_window(scheduler_interval_secs: u64) -> Duration {
    Duration::minutes(MISSED_GRACE_MINUTES) + Duration::seconds(scheduler_interval_secs as i64)
}

#[allow(clippy::type_complexity)] // plain row tuples, read once below
async fn load_schedules(db: &DbPool) -> Vec<ScheduleSnapshot> {
    let mut out = Vec::new();

    let rows: Vec<(String, String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT s.id, s.database_id, d.name, s.next_run_at, s.last_run_at \
         FROM database_backup_schedules s JOIN databases d ON d.id = s.database_id \
         WHERE s.enabled = 1",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    out.extend(
        rows.into_iter()
            .map(|(id, db_id, name, next, last)| ScheduleSnapshot {
                kind: ScheduleKind::Database,
                schedule_id: id,
                resource_kind: BackupResourceKind::Database,
                resource_id: Some(db_id),
                resource_name: name,
                app_id: None,
                next_run_at: next,
                last_run_at: last,
            }),
    );

    let rows: Vec<(
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT s.id, s.volume_id, v.name, v.app_id, s.next_run_at, s.last_run_at \
             FROM volume_backup_schedules s JOIN volumes v ON v.id = s.volume_id \
             WHERE s.enabled = 1",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    out.extend(
        rows.into_iter()
            .map(|(id, vol_id, name, app_id, next, last)| ScheduleSnapshot {
                kind: ScheduleKind::Volume,
                schedule_id: id,
                resource_kind: BackupResourceKind::Volume,
                resource_id: Some(vol_id),
                resource_name: name,
                app_id: Some(app_id),
                next_run_at: next,
                last_run_at: last,
            }),
    );

    let rows: Vec<(
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT id, backup_type, target_id, next_run_at, last_run_at \
             FROM backup_schedules WHERE enabled = 1",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    for (id, backup_type, target_id, next, last) in rows {
        let (resource_kind, resource_name) = match backup_type.as_str() {
            "s3_database" => {
                let name: Option<String> =
                    sqlx::query_scalar("SELECT name FROM databases WHERE id = ?")
                        .bind(&target_id)
                        .fetch_optional(db)
                        .await
                        .ok()
                        .flatten();
                (
                    BackupResourceKind::Database,
                    name.unwrap_or_else(|| target_id.clone().unwrap_or_default()),
                )
            }
            "s3_volume" => {
                let name: Option<String> =
                    sqlx::query_scalar("SELECT name FROM volumes WHERE id = ?")
                        .bind(&target_id)
                        .fetch_optional(db)
                        .await
                        .ok()
                        .flatten();
                (
                    BackupResourceKind::Volume,
                    name.unwrap_or_else(|| target_id.clone().unwrap_or_default()),
                )
            }
            _ => (BackupResourceKind::Instance, "Rivetr instance".to_string()),
        };
        out.push(ScheduleSnapshot {
            kind: ScheduleKind::Instance,
            schedule_id: id,
            resource_kind,
            resource_id: if resource_kind == BackupResourceKind::Instance {
                None
            } else {
                target_id
            },
            resource_name,
            app_id: None,
            next_run_at: next,
            last_run_at: last,
        });
    }

    out
}

/// Find missed runs, record them, and notify once per missed run.
/// Returns how many new missed runs were reported.
pub async fn check_missed_backups(db: &DbPool, grace: Duration) -> usize {
    let schedules = load_schedules(db).await;
    let missed = find_missed(schedules, Utc::now(), grace);
    let mut reported = 0;
    for run in missed {
        let expected = run.expected_at.to_rfc3339();
        let inserted = sqlx::query(
            "INSERT OR IGNORE INTO backup_missed_alerts (schedule_kind, schedule_id, expected_at) \
             VALUES (?, ?, ?)",
        )
        .bind(run.schedule.kind.as_str())
        .bind(&run.schedule.schedule_id)
        .bind(&expected)
        .execute(db)
        .await
        .map(|r| r.rows_affected())
        .unwrap_or(0);
        if inserted == 0 {
            continue;
        }
        tracing::warn!(
            schedule_kind = run.schedule.kind.as_str(),
            schedule_id = %run.schedule.schedule_id,
            resource = %run.schedule.resource_name,
            expected_at = %expected,
            "Scheduled backup did not run"
        );
        let mut event = BackupEvent::new(
            BackupOutcome::Missed,
            run.schedule.resource_kind,
            run.schedule.resource_name.clone(),
        )
        .app_id(run.schedule.app_id.clone())
        .trigger("scheduled")
        .expected_at(expected);
        if let Some(id) = run.schedule.resource_id.clone() {
            event = event.resource_id(id);
        }
        send_backup_notification(db, &event).await;
        reported += 1;
    }

    // Keep the dedup table small.
    let _ = sqlx::query(
        "DELETE FROM backup_missed_alerts WHERE notified_at < datetime('now', '-90 days')",
    )
    .execute(db)
    .await;

    reported
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 5, h, m, 0).unwrap()
    }

    fn snap(id: &str, next: Option<&str>, last: Option<&str>) -> ScheduleSnapshot {
        ScheduleSnapshot {
            kind: ScheduleKind::Database,
            schedule_id: id.into(),
            resource_kind: BackupResourceKind::Database,
            resource_id: Some(format!("db-{id}")),
            resource_name: format!("db {id}"),
            app_id: None,
            next_run_at: next.map(str::to_string),
            last_run_at: last.map(str::to_string),
        }
    }

    #[test]
    fn not_missed_within_grace() {
        let grace = Duration::minutes(15);
        assert!(!is_missed(at(2, 0), None, at(2, 10), grace));
        assert!(!is_missed(at(2, 0), None, at(2, 15), grace));
        assert!(is_missed(at(2, 0), None, at(2, 16), grace));
    }

    #[test]
    fn not_missed_when_it_ran_after_it_was_due() {
        let grace = Duration::minutes(15);
        assert!(!is_missed(at(2, 0), Some(at(2, 1)), at(3, 0), grace));
        assert!(!is_missed(at(2, 0), Some(at(2, 0)), at(3, 0), grace));
        // A run from before the due time does not count.
        assert!(is_missed(at(2, 0), Some(at(1, 0)), at(3, 0), grace));
    }

    #[test]
    fn find_missed_picks_overdue_schedules_only() {
        let schedules = vec![
            snap(
                "late",
                Some("2026-11-05T02:00:00Z"),
                Some("2026-11-04T02:00:01Z"),
            ),
            snap("fine", Some("2026-11-05T04:00:00Z"), None),
            snap(
                "ran",
                Some("2026-11-05T02:00:00Z"),
                Some("2026-11-05T02:00:05+00:00"),
            ),
            snap("sqlite-format", Some("2026-11-05 01:00:00"), None),
            snap("never", None, None),
            snap("garbage", Some("not a date"), None),
        ];
        let missed = find_missed(schedules, at(3, 0), Duration::minutes(15));
        let ids: Vec<&str> = missed
            .iter()
            .map(|m| m.schedule.schedule_id.as_str())
            .collect();
        assert_eq!(ids, vec!["late", "sqlite-format"]);
        assert_eq!(missed[0].expected_at, at(2, 0));
        assert_eq!(missed[1].expected_at, at(1, 0));
    }

    #[test]
    fn grace_includes_scheduler_interval() {
        assert_eq!(grace_window(60), Duration::minutes(16));
        assert_eq!(grace_window(0), Duration::minutes(15));
    }

    #[test]
    fn timestamps_parse_in_both_formats() {
        assert_eq!(parse_timestamp("2026-11-05T02:00:00Z"), Some(at(2, 0)));
        assert_eq!(parse_timestamp("2026-11-05T04:00:00+02:00"), Some(at(2, 0)));
        assert_eq!(parse_timestamp("2026-11-05 02:00:00"), Some(at(2, 0)));
        assert_eq!(parse_timestamp("yesterday"), None);
    }

    #[tokio::test]
    async fn missed_run_is_recorded_once_and_ran_schedules_are_not() {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::db::init(dir.path()).await.unwrap();
        sqlx::query(
            "INSERT INTO databases (id, name, db_type, version, status, internal_port, credentials) \
             VALUES ('d1', 'orders', 'postgres', '16', 'running', 5432, '{}'), \
                    ('d2', 'billing', 'postgres', '16', 'running', 5432, '{}')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let overdue = (Utc::now() - Duration::hours(3)).to_rfc3339();
        let upcoming = (Utc::now() + Duration::hours(3)).to_rfc3339();
        sqlx::query(
            "INSERT INTO database_backup_schedules (id, database_id, next_run_at) \
             VALUES ('s1', 'd1', ?), ('s2', 'd2', ?)",
        )
        .bind(&overdue)
        .bind(&upcoming)
        .execute(&pool)
        .await
        .unwrap();

        let grace = Duration::minutes(15);
        assert_eq!(check_missed_backups(&pool, grace).await, 1);
        // The same missed run is not reported twice.
        assert_eq!(check_missed_backups(&pool, grace).await, 0);
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM backup_missed_alerts")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rows, 1);

        // Disabled schedules are ignored.
        sqlx::query(
            "UPDATE database_backup_schedules SET enabled = 0, next_run_at = ? WHERE id = 's1'",
        )
        .bind((Utc::now() - Duration::hours(5)).to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(check_missed_backups(&pool, grace).await, 0);
    }
}
