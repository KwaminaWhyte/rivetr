//! Deploy-time health check driven by the app's structured healthcheck config.

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;

use crate::db::{HealthcheckConfig, HealthcheckType};
use crate::runtime::ContainerRuntime;
use crate::DbPool;

use super::super::add_deployment_log;

/// Longest start period honoured by the pipeline, so a typo cannot stall a deploy.
const MAX_START_PERIOD_SECS: u32 = 300;

/// Whether a response status satisfies the expected code. 200 (the default and the
/// value legacy path-only checks migrate to) accepts any 2xx, as the old check did.
pub fn status_matches(expected: u16, actual: u16) -> bool {
    if expected == 200 {
        (200..300).contains(&actual)
    } else {
        expected == actual
    }
}

/// Run the configured check against a freshly started container. Returns `Ok(true)`
/// once an attempt passes, `Ok(false)` after all retries fail. Attempt results are
/// written to the deployment log.
pub(super) async fn run_health_check(
    db: &DbPool,
    runtime: &Arc<dyn ContainerRuntime>,
    deployment_id: &str,
    container_id: &str,
    app_port: u16,
    hc: &HealthcheckConfig,
) -> Result<bool> {
    let start_period = hc.start_period_secs.min(MAX_START_PERIOD_SECS);
    if start_period > 0 {
        tokio::time::sleep(Duration::from_secs(u64::from(start_period))).await;
    }

    let timeout = Duration::from_secs(u64::from(hc.timeout_secs.max(1)));
    let retries = super::effective_health_check_retries(hc.retries, hc.interval_secs);

    // Resolve the host address once for HTTP checks.
    let http_target = if hc.check_type == HealthcheckType::Http {
        let info = runtime.inspect(container_id).await?;
        let target_port = hc.target_port(app_port);
        let host_port = info
            .host_port_for(target_port)
            .or(if target_port == app_port {
                info.port
            } else {
                None
            });
        match host_port {
            Some(p) => Some(format!("{}://127.0.0.1:{}{}", hc.scheme, p, hc.path)),
            None => {
                add_deployment_log(
                    db,
                    deployment_id,
                    "warn",
                    &format!(
                        "Health check port {} is not published on the host; skipping the HTTP check",
                        target_port
                    ),
                )
                .await?;
                return Ok(true);
            }
        }
    } else {
        None
    };

    let client = reqwest::Client::builder()
        .timeout(timeout)
        .danger_accept_invalid_certs(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    for attempt in 1..=retries {
        let outcome = match &http_target {
            Some(url) => http_attempt(&client, url, hc).await,
            None => command_attempt(runtime, container_id, hc, timeout).await,
        };
        match outcome {
            Ok(()) => return Ok(true),
            Err(reason) => {
                add_deployment_log(
                    db,
                    deployment_id,
                    "warn",
                    &format!("Health check attempt {}/{}: {}", attempt, retries, reason),
                )
                .await?;
            }
        }
        if attempt < retries {
            tokio::time::sleep(Duration::from_secs(u64::from(hc.interval_secs.max(1)))).await;
        }
    }
    Ok(false)
}

async fn http_attempt(
    client: &reqwest::Client,
    url: &str,
    hc: &HealthcheckConfig,
) -> std::result::Result<(), String> {
    let method = reqwest::Method::from_bytes(hc.method.as_bytes()).unwrap_or(reqwest::Method::GET);
    let mut req = client.request(method, url);
    if !matches!(hc.host.as_str(), "localhost" | "127.0.0.1" | "0.0.0.0") {
        req = req.header(reqwest::header::HOST, hc.host.clone());
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    if !status_matches(hc.expected_status, status) {
        return Err(format!(
            "status {} (expected {})",
            status, hc.expected_status
        ));
    }
    if let Some(expected) = hc.expected_body.as_deref() {
        let body = resp.text().await.map_err(|e| e.to_string())?;
        if !body.contains(expected) {
            return Err(format!("response does not contain {:?}", expected));
        }
    }
    Ok(())
}

async fn command_attempt(
    runtime: &Arc<dyn ContainerRuntime>,
    container_id: &str,
    hc: &HealthcheckConfig,
    timeout: Duration,
) -> std::result::Result<(), String> {
    let command = hc.command.clone().unwrap_or_default();
    let run = runtime.run_command(
        container_id,
        vec!["sh".to_string(), "-c".to_string(), command],
    );
    match tokio::time::timeout(timeout, run).await {
        Err(_) => Err(format!("timed out after {}s", timeout.as_secs())),
        Ok(Err(e)) => Err(e.to_string()),
        Ok(Ok(res)) if res.exit_code == 0 => Ok(()),
        Ok(Ok(res)) => {
            let detail = res.stderr.trim();
            Err(if detail.is_empty() {
                format!("exit code {}", res.exit_code)
            } else {
                format!("exit code {}: {}", res.exit_code, truncate(detail, 200))
            })
        }
    }
}

fn truncate(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_200_accepts_any_2xx_but_explicit_codes_are_exact() {
        assert!(status_matches(200, 200));
        assert!(status_matches(200, 204));
        assert!(!status_matches(200, 301));
        assert!(status_matches(204, 204));
        assert!(!status_matches(204, 200));
        assert!(status_matches(401, 401));
    }
}
