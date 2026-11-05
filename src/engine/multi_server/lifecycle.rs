//! Stop, start and remove the containers of a multi-server app on every
//! server, for the app's Start/Stop/Restart buttons and app deletion.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use arc_swap::ArcSwap;

use crate::crypto;
use crate::db::{App, DeploymentTarget, Server, LOCAL_SERVER_ID};
use crate::engine::server_ops::{shell_quote, HostTarget};
use crate::proxy::RouteTable;
use crate::DbPool;

use super::{distribute, plan, routes, shell};

const TIMEOUT: Duration = Duration::from_secs(120);

/// Live (healthy or stopped) targets of an app, grouped by server.
async fn live_targets(db: &DbPool, app_id: &str) -> BTreeMap<String, Vec<DeploymentTarget>> {
    let rows: Vec<DeploymentTarget> = sqlx::query_as(
        "SELECT * FROM deployment_targets WHERE app_id = ? AND status IN ('healthy', 'stopped')",
    )
    .bind(app_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let mut out: BTreeMap<String, Vec<DeploymentTarget>> = BTreeMap::new();
    for r in rows {
        out.entry(r.server_id.clone()).or_default().push(r);
    }
    out
}

async fn host_target(
    db: &DbPool,
    server_id: &str,
    key: Option<&[u8; 32]>,
    bind_private: bool,
) -> Option<(HostTarget, String)> {
    if server_id == LOCAL_SERVER_ID {
        return Some((
            HostTarget::Local {
                prefer_podman: false,
            },
            plan::local_addressing().upstream_host,
        ));
    }
    let server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(server_id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()?;
    let dec = |v: Option<&str>| {
        v.filter(|s| !s.is_empty())
            .map(|s| crypto::decrypt_if_encrypted(s, key).unwrap_or_else(|_| s.to_string()))
    };
    let addressing = plan::remote_addressing(&server, bind_private);
    Some((
        HostTarget::Remote {
            host: server.ssh_host().to_string(),
            port: u16::try_from(server.port).unwrap_or(22),
            username: server.username.clone(),
            private_key: dec(server.ssh_private_key.as_deref()),
            password: dec(server.ssh_password.as_deref()),
        },
        addressing.upstream_host,
    ))
}

fn names(rows: &[DeploymentTarget]) -> Vec<String> {
    rows.iter()
        .filter_map(|r| r.container_name.clone())
        .collect()
}

/// Start (`run = true`) or stop every container of the app on every server.
/// Starting re-reads each published port (Docker may pick a new one) and
/// reinstalls the proxy pool. Returns the number of containers handled.
pub async fn set_running(
    db: &DbPool,
    routes_table: &Arc<ArcSwap<RouteTable>>,
    app: &App,
    key: Option<&[u8; 32]>,
    run: bool,
) -> Result<usize> {
    let settings = super::load_settings(db, &app.id).await;
    let mut handled = 0;
    let mut errors = Vec::new();
    for (server_id, rows) in live_targets(db, &app.id).await {
        let Some((target, upstream_host)) =
            host_target(db, &server_id, key, settings.bind_private).await
        else {
            errors.push(format!("server {} not found", server_id));
            continue;
        };
        for row in rows {
            let Some(name) = row.container_name.clone() else {
                continue;
            };
            let script = if run {
                format!(
                    "$D start {n} >/dev/null && $D port {n} {p}/tcp",
                    n = shell_quote(&name),
                    p = app.port
                )
            } else {
                format!(
                    "$D stop -t {} {}",
                    app.stop_grace_period.unwrap_or(10).max(1),
                    shell_quote(&name)
                )
            };
            match shell::exec(&target, &script, None, TIMEOUT).await {
                Ok(o) if o.success() => {
                    handled += 1;
                    if run {
                        let port = distribute::parse_run_output(&format!("x\n{}", o.stdout))
                            .map(|(_, p)| p);
                        let upstream = port.map(|p| format!("{}:{}", upstream_host, p));
                        let _ = sqlx::query(
                            "UPDATE deployment_targets SET status = 'healthy', upstream = COALESCE(?, upstream) WHERE id = ?",
                        )
                        .bind(upstream)
                        .bind(&row.id)
                        .execute(db)
                        .await;
                    } else {
                        let _ = sqlx::query(
                            "UPDATE deployment_targets SET status = 'stopped' WHERE id = ?",
                        )
                        .bind(&row.id)
                        .execute(db)
                        .await;
                    }
                }
                Ok(o) => errors.push(format!("{}: {}", name, o.combined())),
                Err(e) => errors.push(format!("{}: {:#}", name, e)),
            }
        }
    }

    let table = routes_table.load();
    if run {
        let upstreams = routes::healthy_upstreams(db, &app.id).await;
        routes::install_routes(db, routes_table, app, &upstreams, &settings).await;
    } else {
        for (d, _) in app.get_all_domains_with_redirects() {
            table.remove_route(&d);
        }
    }
    let _ = sqlx::query(
        "UPDATE deployments SET status = ? WHERE id = (SELECT id FROM deployments WHERE app_id = ? \
         AND status IN ('running', 'stopped') ORDER BY started_at DESC LIMIT 1)",
    )
    .bind(if run { "running" } else { "stopped" })
    .bind(&app.id)
    .execute(db)
    .await;

    if handled == 0 && !errors.is_empty() {
        anyhow::bail!(errors.join("; "));
    }
    if !errors.is_empty() {
        tracing::warn!(app = %app.name, errors = ?errors, "Some multi-server containers did not change state");
    }
    Ok(handled)
}

/// Remove every container of the app on every server (app deletion).
pub async fn remove_all(db: &DbPool, app_id: &str, key: Option<&[u8; 32]>) {
    for (server_id, rows) in live_targets(db, app_id).await {
        if let Some((target, _)) = host_target(db, &server_id, key, true).await {
            let script = distribute::remove_script(&names(&rows));
            if let Err(e) = shell::exec(&target, &script, None, TIMEOUT).await {
                tracing::warn!(server = %server_id, error = %e, "Failed to remove multi-server containers");
            }
        }
        for r in rows {
            let _ = sqlx::query("UPDATE deployment_targets SET status = 'replaced' WHERE id = ?")
                .bind(&r.id)
                .execute(db)
                .await;
        }
    }
}
