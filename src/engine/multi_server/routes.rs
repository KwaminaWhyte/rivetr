//! Proxy routes for multi-server apps: every domain of the app points at an
//! upstream pool holding all healthy containers across servers.

use std::sync::Arc;

use arc_swap::ArcSwap;

use crate::db::{App, AppRedirectRule, MultiServerSettings, RedirectRule};
use crate::proxy::{Backend, BasicAuthConfig, LbPolicy, RouteTable, UpstreamPool};
use crate::DbPool;

/// Container id recorded on pooled routes (the pool spans several containers).
pub fn pool_route_id(app_id: &str) -> String {
    format!("multi-server:{}", app_id)
}

async fn redirect_rules(db: &DbPool, app_id: &str) -> Vec<RedirectRule> {
    sqlx::query_as::<_, AppRedirectRule>(
        "SELECT * FROM app_redirect_rules WHERE app_id = ? AND is_enabled = 1 \
         ORDER BY sort_order ASC, created_at ASC",
    )
    .bind(app_id)
    .fetch_all(db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| RedirectRule {
        source_pattern: r.source_pattern,
        destination: r.destination,
        is_permanent: r.is_permanent != 0,
    })
    .collect()
}

/// Point every domain of `app` at `upstreams` ("host:port"). An empty list
/// leaves the current routes alone.
pub async fn install_routes(
    db: &DbPool,
    routes: &Arc<ArcSwap<RouteTable>>,
    app: &App,
    upstreams: &[String],
    settings: &MultiServerSettings,
) {
    let Some(first) = upstreams.first() else {
        return;
    };
    let (host, port) = match first.rsplit_once(':') {
        Some((h, p)) => (h.to_string(), p.parse().unwrap_or(app.port as u16)),
        None => (first.clone(), app.port as u16),
    };
    let rules = redirect_rules(db, &app.id).await;
    let app_domains = app.get_domains();
    let pool = Arc::new(UpstreamPool::new(
        upstreams.to_vec(),
        LbPolicy::parse(&settings.lb_policy),
        settings.sticky_sessions,
    ));
    let table = routes.load();

    let mut entries = app.get_all_domains_with_redirects();
    if entries.is_empty() {
        if let Some(d) = app.domain.as_ref().filter(|d| !d.is_empty()) {
            entries.push((d.clone(), None));
        }
    }
    for (domain, www_target) in entries {
        let mut backend = Backend::new(pool_route_id(&app.id), host.clone(), port)
            .with_healthcheck(app.healthcheck.clone())
            .with_access_logging(app.access_logging != 0)
            .with_strip_prefix(app.strip_prefix.clone())
            .with_redirect_rules(rules.clone());
        // Per-domain internal ports are not published on remote servers, so only
        // indexing and redirect settings apply here.
        backend.apply_domain_options(&crate::db::domain_proxy_options(&app_domains, &domain), &[]);
        if www_target.is_some() {
            backend.www_redirect_target = www_target;
            table.add_route(domain, backend);
            continue;
        }
        if app.basic_auth_enabled != 0 {
            if let (Some(u), Some(h)) = (&app.basic_auth_username, &app.basic_auth_password_hash) {
                backend.set_basic_auth(BasicAuthConfig::new(u.clone(), h.clone()));
            }
        }
        table.add_pool(domain, pool.clone(), backend);
    }
}

/// Upstreams of an app's healthy multi-server containers, newest first.
pub async fn healthy_upstreams(db: &DbPool, app_id: &str) -> Vec<String> {
    sqlx::query_scalar::<_, String>(
        "SELECT upstream FROM deployment_targets \
         WHERE app_id = ? AND status = 'healthy' AND upstream IS NOT NULL \
         ORDER BY created_at DESC",
    )
    .bind(app_id)
    .fetch_all(db)
    .await
    .unwrap_or_default()
}

/// Re-create pooled routes on startup for every app with a placement.
pub async fn restore_routes(db: &DbPool, routes: &Arc<ArcSwap<RouteTable>>) {
    let apps: Vec<App> = sqlx::query_as(
        "SELECT * FROM apps WHERE id IN (SELECT DISTINCT app_id FROM app_server_placements)",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    for app in apps {
        let upstreams = healthy_upstreams(db, &app.id).await;
        if upstreams.is_empty() {
            continue;
        }
        let settings = super::load_settings(db, &app.id).await;
        install_routes(db, routes, &app, &upstreams, &settings).await;
        tracing::info!(
            app = %app.name,
            upstreams = upstreams.len(),
            "Restored multi-server proxy routes"
        );
    }
}
