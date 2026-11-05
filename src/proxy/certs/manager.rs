//! Background task that keeps every routed hostname's certificate issued.
//!
//! Each tick (every minute, or right away when woken by the API) it plans the
//! certificates that should exist, then orders the missing or due ones one at
//! a time. A failure is recorded on that domain's row with an exponential
//! backoff and never stops the loop, so one bad domain cannot block others.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::SqlitePool;
use tracing::{debug, error, info, warn};

use super::db::{self, DomainCertRow, MODE_CUSTOM};
use super::store::{self, CertSource};
use super::{
    backoff, plan_orders, registered_domain, renewal_due_at, save_domain_cert, CertRuntime,
    Challenge, OrderPlan, RateBudget, WildcardSetting, CUSTOM_EXPIRY_WARNING_DAYS,
    WEEKLY_CERTS_PER_DOMAIN,
};
use crate::proxy::acme::{AcmeChallenges, AcmeClient, AcmeConfig, ChallengeSolver};

const TICK: Duration = Duration::from_secs(60);
const EXPIRY_CHECK_EVERY: Duration = Duration::from_secs(12 * 60 * 60);

pub struct CertManager {
    db: SqlitePool,
    runtime: Arc<CertRuntime>,
    acme_config: Option<AcmeConfig>,
    challenges: AcmeChallenges,
    acme: Option<Arc<AcmeClient>>,
    budget: RateBudget,
}

/// Outcome of one order, for logs and tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderOutcome {
    Issued(String),
    Failed(String, String),
    Deferred(String, String),
}

impl CertManager {
    /// `acme_config` is `None` when ACME is disabled (custom certificates only).
    pub fn new(
        db: SqlitePool,
        runtime: Arc<CertRuntime>,
        acme_config: Option<AcmeConfig>,
        challenges: AcmeChallenges,
    ) -> Self {
        Self {
            db,
            runtime,
            acme_config,
            challenges,
            acme: None,
            budget: RateBudget::lets_encrypt(),
        }
    }

    /// Use an already-initialised ACME client (tests).
    pub fn with_client(mut self, client: Arc<AcmeClient>) -> Self {
        self.acme = Some(client);
        self
    }

    pub async fn run(mut self) {
        info!("Certificate manager started");
        let mut since_expiry_check = EXPIRY_CHECK_EVERY;
        loop {
            self.tick().await;
            if since_expiry_check >= EXPIRY_CHECK_EVERY {
                self.check_custom_expiry().await;
                since_expiry_check = Duration::ZERO;
            }
            let runtime = self.runtime.clone();
            tokio::select! {
                _ = tokio::time::sleep(TICK) => since_expiry_check += TICK,
                _ = runtime.woken() => {}
            }
        }
    }

    /// The ACME client, created lazily so a CA outage at boot only delays
    /// issuance instead of disabling it until restart.
    async fn client(&mut self) -> Option<Arc<AcmeClient>> {
        if self.acme.is_none() {
            let config = self.acme_config.clone()?;
            match AcmeClient::with_challenges(config, self.challenges.clone()).await {
                Ok(c) => self.acme = Some(Arc::new(c)),
                Err(e) => {
                    warn!(error = %e, "ACME client unavailable; will retry");
                    return None;
                }
            }
        }
        self.acme.clone()
    }

    /// One pass: order every missing or due certificate the budget allows.
    pub async fn tick(&mut self) -> Vec<OrderOutcome> {
        let mut outcomes = Vec::new();
        let Some(acme) = self.client().await else {
            return outcomes;
        };
        let plans = match self.current_plans().await {
            Ok(p) => p,
            Err(e) => {
                warn!(error = %e, "Could not plan certificates");
                return outcomes;
            }
        };
        let rows: HashMap<String, DomainCertRow> = db::list(&self.db)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|r| (r.domain.clone(), r))
            .collect();

        let now = Utc::now();
        let store = self.runtime.store.clone();
        let mut due: Vec<(u8, OrderPlan)> = Vec::new();
        for plan in plans {
            let existing = store.get(CertSource::Acme, &plan.id);
            let needed = match &existing {
                None => true,
                Some(c) => {
                    now >= renewal_due_at(c.info.not_after, &plan.id)
                        || !plan.names.iter().all(|n| c.info.names.contains(n))
                }
            };
            if !needed {
                continue;
            }
            if rows
                .get(&plan.id)
                .and_then(|r| r.next_attempt())
                .is_some_and(|t| t > now)
            {
                continue;
            }
            // Uncovered names first, then names the legacy SAN cert still
            // serves, then renewals of per-domain certificates.
            let priority = if existing.is_some() {
                2
            } else if plan.names.iter().all(|n| store.covers(n)) {
                1
            } else {
                0
            };
            due.push((priority, plan));
        }
        due.sort_by_key(|(p, _)| *p);

        for (priority, plan) in due {
            if priority < 2 {
                let reg = registered_domain(&plan.id);
                let issued = db::issued_last_week_under(&self.db, &reg)
                    .await
                    .unwrap_or(0);
                if issued >= WEEKLY_CERTS_PER_DOMAIN {
                    let msg = format!(
                        "Waiting: {issued} certificates were issued under {reg} this week \
                         (Let's Encrypt limit). Consider a wildcard certificate via a DNS provider."
                    );
                    let _ = db::record_failure(
                        &self.db,
                        &plan.id,
                        &msg,
                        rows.get(&plan.id).map(|r| r.failure_count).unwrap_or(0),
                        now + chrono::Duration::hours(12),
                    )
                    .await;
                    outcomes.push(OrderOutcome::Deferred(plan.id.clone(), msg));
                    continue;
                }
            }
            if !self.budget.try_take(Utc::now()) {
                warn!("ACME order budget for this window is used up; remaining domains wait");
                outcomes.push(OrderOutcome::Deferred(
                    plan.id.clone(),
                    "order budget used up".into(),
                ));
                break;
            }
            match self.issue(&acme, &plan).await {
                Ok(info) => {
                    info!(domain = %plan.id, not_after = %info.not_after, "Certificate issued");
                    if let Err(e) = db::record_issued(&self.db, &plan.id, &info).await {
                        error!(domain = %plan.id, error = %e, "Could not record certificate");
                    }
                    outcomes.push(OrderOutcome::Issued(plan.id.clone()));
                }
                Err(e) => {
                    let failures = rows.get(&plan.id).map(|r| r.failure_count).unwrap_or(0) + 1;
                    let wait = backoff(failures);
                    let msg = format!("{e:#}");
                    warn!(
                        domain = %plan.id,
                        failures,
                        retry_in_minutes = wait.num_minutes(),
                        error = %msg,
                        "Certificate order failed"
                    );
                    let _ =
                        db::record_failure(&self.db, &plan.id, &msg, failures, Utc::now() + wait)
                            .await;
                    outcomes.push(OrderOutcome::Failed(plan.id.clone(), msg));
                }
            }
        }
        outcomes
    }

    async fn current_plans(&self) -> Result<Vec<OrderPlan>> {
        let mut domains = crate::proxy::acme::collect_cert_domains(&self.db).await?;
        if let Some(d) = self.runtime.store.default_name() {
            domains.insert(0, d);
        }
        let rows: HashMap<String, DomainCertRow> = db::list(&self.db)
            .await?
            .into_iter()
            .map(|r| (r.domain.clone(), r))
            .collect();
        let wildcard = load_wildcard_setting(&self.db).await;
        Ok(plan_orders(&domains, &rows, wildcard.as_ref()))
    }

    async fn issue(&self, acme: &AcmeClient, plan: &OrderPlan) -> Result<store::CertInfo> {
        let result = match &plan.challenge {
            Challenge::Http01 => {
                acme.request_certificate_with(&plan.names, &ChallengeSolver::Http01)
                    .await?
            }
            Challenge::Dns01 { provider_id } => {
                let solver = dns_solver(&self.db, &self.runtime, provider_id).await?;
                acme.request_certificate_with(&plan.names, &ChallengeSolver::Dns01(solver.as_ref()))
                    .await?
            }
        };
        let loaded = store::load_pem(&result.certificate_chain_pem, &result.private_key_pem)
            .context("The CA returned a certificate we could not load")?;
        save_domain_cert(
            &self.runtime.cache_dir,
            &plan.id,
            &result.certificate_chain_pem,
            &result.private_key_pem,
        )
        .await?;
        let info = loaded.info.clone();
        self.runtime
            .store
            .insert(CertSource::Acme, &plan.id, loaded);
        Ok(info)
    }

    /// Log (once) uploaded certificates that expire soon or already did.
    async fn check_custom_expiry(&self) {
        let Ok(rows) = db::list(&self.db).await else {
            return;
        };
        let soon = Utc::now() + chrono::Duration::days(CUSTOM_EXPIRY_WARNING_DAYS);
        for row in rows.iter().filter(|r| r.mode == MODE_CUSTOM) {
            let Some(not_after) = db::parse_ts(row.not_after.as_deref()) else {
                continue;
            };
            if not_after <= soon && row.expiry_notified_at.is_none() {
                warn!(
                    domain = %row.domain,
                    expires = %not_after,
                    "Uploaded certificate expires soon; upload a renewed one"
                );
                let _ = db::mark_expiry_notified(&self.db, &row.domain).await;
            } else {
                debug!(domain = %row.domain, "Uploaded certificate still valid");
            }
        }
    }
}

/// Instance wildcard setting (`instance_settings` keys), if both parts are set.
pub async fn load_wildcard_setting(db: &SqlitePool) -> Option<WildcardSetting> {
    let rows: Vec<(String, Option<String>)> =
        sqlx::query_as("SELECT key, value FROM instance_settings WHERE key IN (?, ?)")
            .bind(super::KEY_WILDCARD_DOMAIN)
            .bind(super::KEY_WILDCARD_PROVIDER)
            .fetch_all(db)
            .await
            .ok()?;
    let get = |k: &str| {
        rows.iter()
            .find(|(key, _)| key == k)
            .and_then(|(_, v)| v.clone())
            .filter(|v| !v.trim().is_empty())
    };
    Some(WildcardSetting {
        domain: get(super::KEY_WILDCARD_DOMAIN)?,
        provider_id: get(super::KEY_WILDCARD_PROVIDER)?,
    })
}

async fn dns_solver(
    db: &SqlitePool,
    runtime: &CertRuntime,
    provider_id: &str,
) -> Result<Box<dyn crate::proxy::acme::Dns01Solver>> {
    let solver = super::dns::solver_for(db, runtime.encryption_key(), provider_id).await?;
    Ok(Box::new(solver))
}
