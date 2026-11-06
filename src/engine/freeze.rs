//! Deployment freeze windows: the one place that decides whether a deploy may
//! start now.
//!
//! Every deploy trigger calls [`check_app`] (apps) or [`check_team`] (compose
//! services, which only have team-wide windows) before it queues work:
//! dashboard and API deploys, Git webhooks (GitHub, GitLab, Gitea, Bitbucket),
//! Docker Hub webhooks, scheduled deployments, deploy-by-tag and compose
//! auto-deploy. Preview deployments are exempt unless the window has
//! `include_previews` set.
//!
//! Window matching:
//! - an app window (`app_id` set) applies to that app only;
//! - a team-wide window (`app_id` NULL, `team_id` set) applies to every app and
//!   compose service of the team.

use chrono::{DateTime, Datelike, Utc};

use crate::db::DeploymentFreezeWindow;
use crate::DbPool;

/// What started a deployment. Decides which windows apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeployTrigger {
    /// Dashboard Deploy button or `POST /api/apps/:id/deploy`.
    Manual,
    /// Push webhook from GitHub, GitLab, Gitea or Bitbucket.
    GitWebhook,
    /// Docker Hub image push webhook.
    RegistryWebhook,
    /// A deployment scheduled for later, checked when it is due.
    Scheduled,
    /// Pull/merge request preview deployment.
    Preview,
    /// `POST /api/tags/:id/deploy`.
    Tag,
    /// Compose service redeploy on a Git push (auto-deploy).
    ComposeAutoDeploy,
}

impl DeployTrigger {
    /// Value stored in `deployments.trigger` for a skipped deployment.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::GitWebhook | Self::RegistryWebhook | Self::ComposeAutoDeploy => "webhook",
            Self::Scheduled => "scheduled",
            Self::Preview => "preview",
            Self::Tag => "tag",
        }
    }
}

/// An active window that blocks a deployment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreezeHit {
    pub window_id: String,
    pub name: String,
    pub start_time: String,
    pub end_time: String,
}

impl FreezeHit {
    /// Human readable reason, used for API errors, skipped deployment records
    /// and webhook event logs.
    pub fn reason(&self) -> String {
        format!(
            "Deployment frozen: '{}' freeze window is active ({} - {} UTC)",
            self.name, self.start_time, self.end_time
        )
    }
}

impl From<&DeploymentFreezeWindow> for FreezeHit {
    fn from(w: &DeploymentFreezeWindow) -> Self {
        Self {
            window_id: w.id.clone(),
            name: w.name.clone(),
            start_time: w.start_time.clone(),
            end_time: w.end_time.clone(),
        }
    }
}

/// Is `now` inside the window's days and `start_time..end_time` (UTC)?
/// A window whose end is before its start wraps midnight.
pub fn window_active_at(window: &DeploymentFreezeWindow, now: DateTime<Utc>) -> bool {
    if window.is_active == 0 {
        return false;
    }
    let current_time = now.format("%H:%M").to_string();
    // 0=Sun as per the schema convention
    let dow = now.weekday().num_days_from_sunday().to_string();
    if !window.days_of_week.split(',').any(|d| d.trim() == dow) {
        return false;
    }
    if window.start_time <= window.end_time {
        current_time >= window.start_time && current_time < window.end_time
    } else {
        current_time >= window.start_time || current_time < window.end_time
    }
}

/// Does this window apply to this kind of trigger? Previews are exempt unless
/// the window opts them in.
pub fn window_applies_to(window: &DeploymentFreezeWindow, trigger: DeployTrigger) -> bool {
    trigger != DeployTrigger::Preview || window.include_previews != 0
}

/// The first window that blocks `trigger` at `now`, if any.
pub fn first_blocking(
    windows: &[DeploymentFreezeWindow],
    trigger: DeployTrigger,
    now: DateTime<Utc>,
) -> Option<FreezeHit> {
    windows
        .iter()
        .find(|w| window_applies_to(w, trigger) && window_active_at(w, now))
        .map(FreezeHit::from)
}

/// Load the active windows for an app (its own plus its team's team-wide ones).
async fn windows_for(
    db: &DbPool,
    app_id: Option<&str>,
    team_id: Option<&str>,
) -> anyhow::Result<Vec<DeploymentFreezeWindow>> {
    Ok(sqlx::query_as(
        "SELECT * FROM deployment_freeze_windows WHERE is_active = 1 \
         AND ((? IS NOT NULL AND app_id = ?) \
              OR (app_id IS NULL AND ? IS NOT NULL AND team_id = ?))",
    )
    .bind(app_id)
    .bind(app_id)
    .bind(team_id)
    .bind(team_id)
    .fetch_all(db)
    .await?)
}

/// Check the freeze windows of an app for `trigger` at `now`.
pub async fn check_app_at(
    db: &DbPool,
    app_id: &str,
    team_id: Option<&str>,
    trigger: DeployTrigger,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<FreezeHit>> {
    let windows = windows_for(db, Some(app_id), team_id).await?;
    Ok(first_blocking(&windows, trigger, now))
}

/// Check the freeze windows of an app for `trigger` right now.
pub async fn check_app(
    db: &DbPool,
    app: &crate::db::App,
    trigger: DeployTrigger,
) -> anyhow::Result<Option<FreezeHit>> {
    check_app_at(db, &app.id, app.team_id.as_deref(), trigger, Utc::now()).await
}

/// Check a team's team-wide windows (compose services) right now.
pub async fn check_team(
    db: &DbPool,
    team_id: Option<&str>,
    trigger: DeployTrigger,
) -> anyhow::Result<Option<FreezeHit>> {
    if team_id.is_none() {
        return Ok(None);
    }
    let windows = windows_for(db, None, team_id).await?;
    Ok(first_blocking(&windows, trigger, Utc::now()))
}

/// Webhook-style check: a lookup error is logged and treated as "not frozen"
/// so a broken window row never silently drops every push.
pub async fn check_app_or_open(
    db: &DbPool,
    app: &crate::db::App,
    trigger: DeployTrigger,
) -> Option<FreezeHit> {
    match check_app(db, app, trigger).await {
        Ok(hit) => hit,
        Err(e) => {
            tracing::warn!(app = %app.name, "Freeze window check failed, allowing deploy: {}", e);
            None
        }
    }
}

/// Record a deployment that a freeze window blocked: status `skipped`, the
/// reason in `error_message`, so it shows in the app's deployment history.
pub async fn record_skipped_deployment(
    db: &DbPool,
    app_id: &str,
    trigger: DeployTrigger,
    commit_sha: Option<&str>,
    commit_message: Option<&str>,
    hit: &FreezeHit,
) -> anyhow::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO deployments \
         (id, app_id, commit_sha, commit_message, status, error_message, started_at, finished_at, trigger) \
         VALUES (?, ?, ?, ?, 'skipped', ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(app_id)
    .bind(commit_sha)
    .bind(commit_message)
    .bind(hit.reason())
    .bind(&now)
    .bind(&now)
    .bind(trigger.as_str())
    .execute(db)
    .await?;
    let _ = sqlx::query(
        "INSERT INTO deployment_logs (deployment_id, level, message) VALUES (?, 'warn', ?)",
    )
    .bind(&id)
    .bind(format!("{} Trigger: {}.", hit.reason(), trigger.as_str()))
    .execute(db)
    .await;
    tracing::info!(app_id, window = %hit.name, trigger = trigger.as_str(), "Deployment skipped by freeze window");
    Ok(id)
}

/// Record a compose service deployment that a freeze window blocked.
pub async fn record_skipped_service_deployment(
    db: &DbPool,
    service_id: &str,
    commit_sha: Option<&str>,
    commit_message: Option<&str>,
    hit: &FreezeHit,
) -> anyhow::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO service_deployments \
         (id, service_id, trigger, triggered_by_name, status, commit_sha, commit_message, error_message, started_at, finished_at, duration_ms) \
         VALUES (?, ?, 'webhook', 'push webhook', 'skipped', ?, ?, ?, ?, ?, 0)",
    )
    .bind(&id)
    .bind(service_id)
    .bind(commit_sha)
    .bind(commit_message)
    .bind(hit.reason())
    .bind(&now)
    .bind(&now)
    .execute(db)
    .await?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn window(
        start: &str,
        end: &str,
        days: &str,
        include_previews: bool,
    ) -> DeploymentFreezeWindow {
        DeploymentFreezeWindow {
            id: "w1".into(),
            app_id: Some("a1".into()),
            team_id: None,
            name: "release".into(),
            start_time: start.into(),
            end_time: end.into(),
            days_of_week: days.into(),
            is_active: 1,
            created_at: String::new(),
            include_previews: include_previews as i32,
        }
    }

    // 2026-11-06 is a Friday (dow 5).
    fn at(h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 6, h, m, 0).unwrap()
    }

    const ALL: [DeployTrigger; 7] = [
        DeployTrigger::Manual,
        DeployTrigger::GitWebhook,
        DeployTrigger::RegistryWebhook,
        DeployTrigger::Scheduled,
        DeployTrigger::Preview,
        DeployTrigger::Tag,
        DeployTrigger::ComposeAutoDeploy,
    ];

    #[test]
    fn every_trigger_is_blocked_inside_a_window_except_previews_by_default() {
        let w = [window("09:00", "17:00", "0,1,2,3,4,5,6", false)];
        for (trigger, blocked) in [
            (DeployTrigger::Manual, true),
            (DeployTrigger::GitWebhook, true),
            (DeployTrigger::RegistryWebhook, true),
            (DeployTrigger::Scheduled, true),
            (DeployTrigger::Preview, false),
            (DeployTrigger::Tag, true),
            (DeployTrigger::ComposeAutoDeploy, true),
        ] {
            assert_eq!(
                first_blocking(&w, trigger, at(12, 0)).is_some(),
                blocked,
                "{trigger:?}"
            );
        }
    }

    #[test]
    fn include_previews_blocks_previews_too() {
        let w = [window("09:00", "17:00", "0,1,2,3,4,5,6", true)];
        for trigger in ALL {
            assert!(
                first_blocking(&w, trigger, at(12, 0)).is_some(),
                "{trigger:?}"
            );
        }
    }

    #[test]
    fn nothing_is_blocked_outside_a_window() {
        let cases = [
            (window("09:00", "17:00", "0,1,2,3,4,5,6", true), at(17, 0)),
            (window("09:00", "17:00", "0,1,2,3,4,5,6", true), at(8, 59)),
            // Right hours, wrong day (Friday is 5).
            (window("09:00", "17:00", "1,2,3,4", true), at(12, 0)),
        ];
        for (w, now) in cases {
            for trigger in ALL {
                assert!(first_blocking(std::slice::from_ref(&w), trigger, now).is_none());
            }
        }
    }

    #[test]
    fn windows_wrap_midnight_and_inactive_windows_are_ignored() {
        let w = window("22:00", "02:00", "5", false);
        assert!(window_active_at(&w, at(23, 30)));
        assert!(window_active_at(&w, at(1, 0)));
        assert!(!window_active_at(&w, at(12, 0)));
        let mut off = window("00:00", "23:59", "0,1,2,3,4,5,6", false);
        off.is_active = 0;
        assert!(!window_active_at(&off, at(12, 0)));
    }

    #[test]
    fn reason_names_the_window() {
        let hit = FreezeHit::from(&window("09:00", "17:00", "5", false));
        assert_eq!(
            hit.reason(),
            "Deployment frozen: 'release' freeze window is active (09:00 - 17:00 UTC)"
        );
    }

    #[tokio::test]
    async fn app_and_team_wide_windows_match_the_right_resources() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init(dir.path()).await.unwrap();
        sqlx::query("INSERT INTO teams (id, name, slug) VALUES ('t1', 'T', 't')")
            .execute(&db)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO apps (id, name, git_url, branch, port, team_id) VALUES \
             ('a1', 'one', 'https://e.com/r.git', 'main', 3000, 't1'), \
             ('a2', 'two', 'https://e.com/r.git', 'main', 3000, 't1')",
        )
        .execute(&db)
        .await
        .unwrap();
        // App window on a1 (team_id set, as the API stores it).
        sqlx::query(
            "INSERT INTO deployment_freeze_windows (id, app_id, team_id, name, start_time, end_time) \
             VALUES ('w1', 'a1', 't1', 'app', '00:00', '23:59')",
        )
        .execute(&db)
        .await
        .unwrap();
        let now = at(12, 0);
        let t = DeployTrigger::GitWebhook;
        assert!(check_app_at(&db, "a1", Some("t1"), t, now)
            .await
            .unwrap()
            .is_some());
        // An app window does not freeze the rest of the team.
        assert!(check_app_at(&db, "a2", Some("t1"), t, now)
            .await
            .unwrap()
            .is_none());

        sqlx::query(
            "INSERT INTO deployment_freeze_windows (id, app_id, team_id, name, start_time, end_time) \
             VALUES ('w2', NULL, 't1', 'team', '00:00', '23:59')",
        )
        .execute(&db)
        .await
        .unwrap();
        assert!(check_app_at(&db, "a2", Some("t1"), t, now)
            .await
            .unwrap()
            .is_some());

        let hit = check_app_at(&db, "a2", Some("t1"), t, now)
            .await
            .unwrap()
            .unwrap();
        let dep = record_skipped_deployment(&db, "a2", t, Some("abc"), None, &hit)
            .await
            .unwrap();
        let (status, err, trigger): (String, String, String) =
            sqlx::query_as("SELECT status, error_message, trigger FROM deployments WHERE id = ?")
                .bind(&dep)
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(status, "skipped");
        assert!(err.contains("'team'"));
        assert_eq!(trigger, "webhook");
    }
}
