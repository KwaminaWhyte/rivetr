//! Start the proxy listeners: HTTP on the proxy port, and HTTPS with the SNI
//! certificate store when ACME is enabled or custom certificates exist.

use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use arc_swap::ArcSwap;
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tracing::{error, info, warn};

use super::{bootstrap, CertManager, CertSource};
use crate::config::Config;
use crate::proxy::acme::{AcmeChallenges, AcmeConfig};
use crate::proxy::{HttpsProxyServer, ProxyServer, RouteTable};

pub async fn launch_proxies(
    proxy_server: ProxyServer,
    http_listener: Option<TcpListener>,
    https_listener: Option<TcpListener>,
    routes: Arc<ArcSwap<RouteTable>>,
    db: SqlitePool,
    config: &Config,
) {
    let https_port = config.server.proxy_https_port;
    let acme_enabled = config.proxy.acme_enabled && config.proxy.acme_email.is_some();
    if config.proxy.acme_enabled && config.proxy.acme_email.is_none() {
        warn!("proxy.acme_enabled is set but proxy.acme_email is missing; ACME stays off");
    }

    let runtime = match bootstrap(
        &db,
        &config.proxy.acme_cache_dir,
        config.proxy.instance_domain.clone(),
        acme_enabled,
        config.auth.encryption_key.as_deref(),
    )
    .await
    {
        Ok(r) => Some(r),
        Err(e) => {
            error!(error = %e, "Certificate store unavailable; running HTTP only");
            None
        }
    };
    let has_custom = runtime
        .as_ref()
        .is_some_and(|r| !r.store.ids(CertSource::Custom).is_empty());

    let Some(runtime) = runtime.filter(|_| acme_enabled || has_custom) else {
        tokio::spawn(async move {
            if let Err(e) = proxy_server
                .run_with_options(http_listener, None, None, 443)
                .await
            {
                error!(error = %e, "Proxy server error");
            }
        });
        return;
    };

    // HTTP first, so HTTP-01 challenges can be answered. The redirect flag is
    // on, but the handler only redirects hosts the store has a certificate for.
    let challenges = AcmeChallenges::new();
    let redirect = Arc::new(AtomicBool::new(true));
    let http = proxy_server.with_cert_store(runtime.store.clone());
    let http_challenges = acme_enabled.then(|| challenges.clone());
    tokio::spawn(async move {
        if let Err(e) = http
            .run_with_options(http_listener, http_challenges, Some(redirect), https_port)
            .await
        {
            error!(error = %e, "HTTP proxy server error");
        }
    });

    match format!("{}:{}", config.server.host, https_port).parse::<SocketAddr>() {
        Ok(https_addr) => {
            let https = HttpsProxyServer::new(https_addr, routes, runtime.store.clone())
                .with_db(db.clone());
            tokio::spawn(async move {
                if let Err(e) = https.run(https_listener).await {
                    error!(error = %e, "HTTPS proxy server error");
                }
            });
            info!(addr = %https_addr, "HTTPS proxy listening (HTTP/2 and HTTP/1.1, per-domain certificates)");
        }
        Err(e) => error!(error = %e, "Invalid HTTPS proxy address"),
    }

    let acme_config = acme_enabled.then(|| AcmeConfig {
        email: config.proxy.acme_email.clone().unwrap_or_default(),
        cache_dir: config.proxy.acme_cache_dir.clone(),
        staging: config.proxy.acme_staging,
        directory_url: None,
    });
    let manager = CertManager::new(db, runtime, acme_config, challenges);
    tokio::spawn(manager.run());
}
