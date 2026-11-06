//! Runs scheduled container restarts (`scheduled_restarts` rows).
//!
//! Every 60 seconds the loop looks at each enabled schedule's `next_restart`.
//! When it has passed, the schedule is claimed (its `next_restart` moves to
//! the next cron time after now) and the restart runs through the app's normal
//! restart path (`crate::api::scheduled_restarts::execute_run`).
//!
//! Downtime: if Rivetr was down when one or more runs were due, only one run
//! happens on startup (a "catch-up", recorded with `catch_up = 1`), and only
//! when the missed time is within [`CATCH_UP_WINDOW`]. Older missed runs are
//! dropped and the schedule simply moves on to its next time.

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::AppState;

/// A due run later than this is reported as a catch-up.
pub const CATCH_UP_GRACE: chrono::Duration = chrono::Duration::minutes(5);
/// Missed runs older than this are not caught up.
pub const CATCH_UP_WINDOW: chrono::Duration = chrono::Duration::hours(24);

/// Parse a 5-field (minute hour day month weekday) or 6/7-field cron
/// expression, the same way the scheduled restarts API validates it.
pub fn parse_cron(expr: &str) -> Option<cron::Schedule> {
    let fields = expr.split_whitespace().count();
    let normalized = if fields >= 6 {
        expr.to_string()
    } else {
        format!("0 {}", expr)
    };
    cron::Schedule::from_str(&normalized).ok()
}

/// The next cron time strictly after `after`.
pub fn next_after(expr: &str, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
    parse_cron(expr)?.after(&after).next()
}

/// A run to perform now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DueRun {
    /// The cron time this run stands for.
    pub scheduled_for: DateTime<Utc>,
    /// True when the run is late (Rivetr was down or busy at that time).
    pub catch_up: bool,
}

/// What to do with one schedule at `now`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub run: Option<DueRun>,
    /// New `next_restart` to store; `None` keeps the stored value.
    pub next: Option<DateTime<Utc>>,
}

/// Decide whether a schedule runs now and what its next time becomes.
///
/// - no stored next time: just compute one (never runs immediately);
/// - next time in the future: nothing to do;
/// - next time passed: run once (a catch-up when late), and move the next
///   time past `now`, so any further missed times are skipped;
/// - next time passed more than [`CATCH_UP_WINDOW`] ago: skip it.
pub fn plan(expr: &str, stored_next: Option<DateTime<Utc>>, now: DateTime<Utc>) -> Plan {
    let upcoming = next_after(expr, now);
    match stored_next {
        None => Plan {
            run: None,
            next: upcoming,
        },
        Some(due) if due > now => Plan {
            run: None,
            next: None,
        },
        Some(due) => {
            let late = now - due;
            let run = (late <= CATCH_UP_WINDOW).then_some(DueRun {
                scheduled_for: due,
                catch_up: late > CATCH_UP_GRACE,
            });
            Plan {
                run,
                next: upcoming,
            }
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct ScheduleRow {
    pub id: String,
    pub app_id: String,
    pub cron_expression: String,
    pub next_restart: Option<String>,
}

fn parse_time(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|n| n.and_utc())
        })
}

/// One pass over all enabled schedules. Returns the runs it claimed.
pub async fn claim_due(db: &crate::DbPool, now: DateTime<Utc>) -> Vec<(ScheduleRow, DueRun)> {
    let rows: Vec<ScheduleRow> = match sqlx::query_as(
        "SELECT id, app_id, cron_expression, next_restart FROM scheduled_restarts WHERE enabled = 1",
    )
    .fetch_all(db)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::debug!("Scheduled restarts unavailable: {}", e);
            return Vec::new();
        }
    };

    let mut claimed = Vec::new();
    for row in rows {
        let stored = row.next_restart.as_deref().and_then(parse_time);
        let plan = plan(&row.cron_expression, stored, now);
        let Some(next) = plan.next else {
            continue;
        };
        // Claim: move next_restart forward only if nobody else did, so a slow
        // restart never runs twice.
        let res = sqlx::query(
            "UPDATE scheduled_restarts SET next_restart = ?, \
             last_restart = CASE WHEN ? THEN ? ELSE last_restart END \
             WHERE id = ? AND next_restart IS ?",
        )
        .bind(next.to_rfc3339())
        .bind(plan.run.is_some())
        .bind(now.to_rfc3339())
        .bind(&row.id)
        .bind(&row.next_restart)
        .execute(db)
        .await;
        match (res, plan.run) {
            (Ok(r), Some(run)) if r.rows_affected() == 1 => claimed.push((row, run)),
            (Ok(_), None) => {
                if stored.is_some_and(|d| d <= now) {
                    tracing::info!(
                        schedule = %row.id,
                        "Scheduled restart missed by more than {}h; not catching up",
                        CATCH_UP_WINDOW.num_hours()
                    );
                }
            }
            (Err(e), _) => {
                tracing::warn!(schedule = %row.id, "Failed to claim scheduled restart: {}", e)
            }
            _ => {}
        }
    }
    claimed
}

/// Spawn the restart scheduler (checks every 60 seconds).
pub fn spawn_restart_scheduler(state: Arc<AppState>) {
    tracing::info!("Starting scheduled restart runner (60s interval)");
    tokio::spawn(async move {
        // Let the engine and proxy settle before restarting anything.
        tokio::time::sleep(Duration::from_secs(30)).await;
        let mut tick = tokio::time::interval(Duration::from_secs(60));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            crate::utils::supervise::guarded("restart_scheduler", async {
                for (row, run) in claim_due(&state.db, Utc::now()).await {
                    let state = state.clone();
                    tokio::spawn(async move {
                        crate::api::scheduled_restarts::execute_run(&state, &row, &run).await;
                    });
                }
            })
            .await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 6, h, m, 0).unwrap()
    }

    #[test]
    fn five_and_six_field_crons_parse() {
        assert_eq!(next_after("0 3 * * *", t(2, 0)), Some(t(3, 0)));
        assert_eq!(next_after("0 0 3 * * *", t(2, 0)), Some(t(3, 0)));
        assert_eq!(next_after("*/15 * * * *", t(2, 1)), Some(t(2, 15)));
        assert!(parse_cron("not a cron").is_none());
    }

    #[test]
    fn first_sighting_only_schedules() {
        let p = plan("0 3 * * *", None, t(2, 0));
        assert_eq!(p.run, None);
        assert_eq!(p.next, Some(t(3, 0)));
    }

    #[test]
    fn future_runs_wait() {
        assert_eq!(
            plan("0 3 * * *", Some(t(3, 0)), t(2, 59)),
            Plan {
                run: None,
                next: None
            }
        );
    }

    #[test]
    fn on_time_run_is_not_a_catch_up() {
        let p = plan("0 3 * * *", Some(t(3, 0)), t(3, 1));
        assert_eq!(
            p.run,
            Some(DueRun {
                scheduled_for: t(3, 0),
                catch_up: false
            })
        );
        assert_eq!(p.next, Utc.with_ymd_and_hms(2026, 11, 7, 3, 0, 0).single());
    }

    #[test]
    fn many_missed_runs_catch_up_once() {
        // Hourly schedule, down from 01:00 to 06:30: five runs were missed.
        let p = plan("0 * * * *", Some(t(1, 0)), t(6, 30));
        assert_eq!(
            p.run,
            Some(DueRun {
                scheduled_for: t(1, 0),
                catch_up: true
            })
        );
        // The next time is after now, so the other missed runs are skipped.
        assert_eq!(p.next, Some(t(7, 0)));
        // The following tick does nothing.
        assert_eq!(plan("0 * * * *", p.next, t(6, 31)).run, None);
    }

    #[test]
    fn very_old_missed_runs_are_dropped() {
        let long_ago = t(1, 0) - chrono::Duration::days(3);
        let p = plan("0 * * * *", Some(long_ago), t(6, 30));
        assert_eq!(p.run, None);
        assert_eq!(p.next, Some(t(7, 0)));
    }

    #[test]
    fn stored_times_parse_in_both_formats() {
        assert_eq!(parse_time("2026-11-06T03:00:00+00:00"), Some(t(3, 0)));
        assert_eq!(parse_time("2026-11-06 03:00:00"), Some(t(3, 0)));
    }

    #[tokio::test]
    async fn claim_runs_a_due_schedule_once() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init(dir.path()).await.unwrap();
        sqlx::query(
            "INSERT INTO apps (id, name, git_url, branch, port) VALUES ('a1', 'one', 'https://e.com/r.git', 'main', 3000)",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO scheduled_restarts (id, app_id, cron_expression, enabled, next_restart) VALUES \
             ('s1', 'a1', '0 * * * *', 1, '2026-11-06T01:00:00+00:00'), \
             ('s2', 'a1', '0 * * * *', 0, '2026-11-06T01:00:00+00:00')",
        )
        .execute(&db)
        .await
        .unwrap();

        let now = t(6, 30);
        let claimed = claim_due(&db, now).await;
        assert_eq!(claimed.len(), 1, "disabled schedules never run");
        assert_eq!(claimed[0].0.id, "s1");
        assert!(claimed[0].1.catch_up);

        // Next tick: already claimed, nothing to run.
        assert!(claim_due(&db, t(6, 31)).await.is_empty());
        let next: Option<String> =
            sqlx::query_scalar("SELECT next_restart FROM scheduled_restarts WHERE id = 's1'")
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(next.as_deref(), Some("2026-11-06T07:00:00+00:00"));
    }
}
