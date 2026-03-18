use anyhow::Result;
use arc_swap::ArcSwap;
use clap::Parser;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use rivetr::api::rate_limit::spawn_cleanup_task as spawn_rate_limit_cleanup_task;
use rivetr::cli::{self, Cli};
use rivetr::config::Config;
use rivetr::db::AppRedirectRule;
use rivetr::db::InstanceSettings;
use rivetr::db::Service;
use rivetr::engine::{
    reconcile_container_status, spawn_cleanup_task as spawn_deployment_cleanup_task,
    spawn_container_monitor_task, spawn_cost_calculator_task, spawn_disk_monitor_task,
    spawn_resource_metrics_collector_task_with_notifications, spawn_stats_collector_task,
    spawn_stats_history_task, spawn_stats_retention_task, updater, BuildLimits, DeploymentEngine,
};
use rivetr::proxy::{
    AcmeClient, AcmeConfig, Backend, BasicAuthConfig, CertificateRenewalManager, HealthChecker,
    HealthCheckerConfig, HttpsProxyServer, ProxyServer, RedirectRule, RouteTable,
};
use rivetr::runtime::{detect_runtime, ContainerRuntime};
use rivetr::startup::run_startup_checks;
use rivetr::AppState;
use rivetr::DbPool;

#[tokio::main]
async fn main() -> Result<()> {
    // Install the ring crypto provider for rustls (required for TLS)
    let _ = rustls::crypto::ring::default_provider().install_default();

    let cli = Cli::parse();

    // If a subcommand is provided, run it and exit
    if cli.command.is_some() {
        return cli::run_command(&cli).await;
    }

    // No subcommand - start the server

    // Load configuration
    let config = Config::load(&cli.config)?;

    // Initialize logging
    let log_level = cli
        .log_level
        .as_ref()
        .unwrap_or(&config.logging.level)
        .clone();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&log_level)),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Rivetr v{}", env!("CARGO_PKG_VERSION"));

    // Initialize Prometheus metrics
    let metrics_handle = rivetr::api::metrics::init_metrics();
    tracing::info!("Prometheus metrics initialized at /metrics");

    // Ensure data directory exists
    rivetr::utils::ensure_dir(&config.server.data_dir)?;

    // Initialize database
    let db = rivetr::db::init(&config.server.data_dir).await?;

    // DB takes priority over toml for instance_domain: load from instance_settings table
    // and override the config value so the rest of startup uses the correct domain.
    let mut config = config;
    if let Ok(db_settings) = InstanceSettings::load(&db).await {
        if let Some(ref db_domain) = db_settings.instance_domain {
            if !db_domain.is_empty() {
                tracing::info!(
                    domain = %db_domain,
                    "Using instance_domain from database (overrides toml)"
                );
                config.proxy.instance_domain = Some(db_domain.clone());
            }
        }
    }

    // Run startup self-checks
    if cli.skip_checks {
        tracing::warn!("Startup self-checks skipped (--skip-checks flag)");
    } else {
        let check_report = run_startup_checks(&config, &db).await;

        if !check_report.all_critical_passed {
            tracing::error!("Critical startup checks failed. Server cannot start safely.");
            for check in &check_report.checks {
                if check.critical && !check.passed {
                    tracing::error!(
                        check = %check.name,
                        message = %check.message,
                        details = ?check.details,
                        "Critical check failure"
                    );
                }
            }
            std::process::exit(1);
        }

        if !check_report.all_passed {
            tracing::warn!(
                "Some non-critical startup checks failed. Server will start with limited functionality."
            );
        }
    }

    // Detect container runtime
    let runtime = detect_runtime(&config.runtime).await?;

    // Create deployment channel
    let (deploy_tx, deploy_rx) = mpsc::channel(100);

    // Start proxy server
    let proxy_addr: SocketAddr = format!("{}:{}", config.server.host, config.server.proxy_port)
        .parse()
        .expect("Invalid proxy address");
    let proxy_server = ProxyServer::new(proxy_addr);
    let routes = proxy_server.routes();

    // Restore routes from running containers
    if let Err(e) = restore_routes(&db, &runtime, &routes).await {
        tracing::warn!("Failed to restore routes: {}", e);
    }

    // Register instance domain → API server so users can access the dashboard via a custom domain
    if let Some(ref instance_domain) = config.proxy.instance_domain {
        let backend = Backend::new(
            "rivetr-api".to_string(),
            "127.0.0.1".to_string(),
            config.server.api_port,
        );
        routes.load().add_route(instance_domain.clone(), backend);
        tracing::info!(
            domain = %instance_domain,
            port = config.server.api_port,
            "Registered instance domain for Rivetr dashboard"
        );
    }

    // Mark any in-progress deployments as failed on startup.
    // If the server was restarted mid-build, the Docker process was killed but the
    // deployment record was left in "building"/"cloning"/etc. state. Fail them now.
    let stuck_statuses = ["pending", "cloning", "building", "starting", "checking"];
    for status in &stuck_statuses {
        let _ = sqlx::query(
            "UPDATE deployments SET status = 'failed', \
             error_message = 'Server restarted during deployment', \
             finished_at = datetime('now') \
             WHERE status = ?",
        )
        .bind(*status)
        .execute(&db)
        .await;
    }
    tracing::info!("Cleaned up any stuck in-progress deployments from previous server run");

    // Reconcile container status on startup
    // This updates database records for containers that stopped while server was down
    reconcile_container_status(&db, &runtime).await;

    // Ensure the shared container network exists and connect all existing
    // Rivetr-managed containers to it (enables hostname-based inter-container discovery).
    runtime.setup_shared_network().await;

    // Start auto-update checker
    let update_checker = updater::start_update_checker(config.auto_update.clone());
    if config.auto_update.enabled {
        tracing::info!(
            "Auto-update checker enabled (interval: {} hours, auto-apply: {})",
            config.auto_update.check_interval_hours,
            config.auto_update.auto_apply
        );
    }

    // Create app state (now includes routes for rollback functionality)
    let state = Arc::new(
        AppState::new(
            config.clone(),
            db.clone(),
            deploy_tx,
            runtime.clone(),
            routes.clone(),
            update_checker,
        )
        .with_metrics(metrics_handle),
    );

    // Start rate limiter cleanup task
    spawn_rate_limit_cleanup_task(
        state.rate_limiter.clone(),
        config.rate_limit.cleanup_interval,
    );
    tracing::info!(
        "Rate limiting enabled: {} req/min (API), {} req/min (webhooks), {} req/min (auth)",
        config.rate_limit.api_requests_per_window,
        config.rate_limit.webhook_requests_per_window,
        config.rate_limit.auth_requests_per_window
    );

    // Start deployment engine with route table and build limits
    let build_limits = BuildLimits::from_runtime_config(&config.runtime);
    tracing::info!(
        "Build resource limits: cpu={}, memory={}",
        config.runtime.build_cpu_limit,
        config.runtime.build_memory_limit
    );
    let engine = DeploymentEngine::new(
        db.clone(),
        runtime.clone(),
        routes.clone(),
        deploy_rx,
        build_limits,
        &config.auth,
    );
    tokio::spawn(async move {
        engine.run().await;
    });

    // Start deployment cleanup task
    spawn_deployment_cleanup_task(db.clone(), runtime.clone(), config.cleanup.clone());

    // Start disk space monitoring task
    spawn_disk_monitor_task(config.server.data_dir.clone(), config.disk_monitor.clone());

    // Start container stats collection task
    spawn_stats_collector_task(runtime.clone());

    // Start stats history recording task (for dashboard charts)
    spawn_stats_history_task(db.clone(), runtime.clone());

    // Start container crash monitor task (monitors apps, databases, and services)
    spawn_container_monitor_task(
        db.clone(),
        runtime.clone(),
        config.container_monitor.clone(),
        config.server.data_dir.clone(),
    );

    // Start database backup scheduler
    rivetr::engine::spawn_database_backup_task(
        db.clone(),
        runtime.clone(),
        config.database_backup.clone(),
        config.server.data_dir.clone(),
    );

    // Start stats retention and aggregation task
    spawn_stats_retention_task(db.clone(), config.stats_retention.clone());

    // Start per-app resource metrics collection task with alert email notifications
    // The external_url is used for building dashboard links in alert emails
    spawn_resource_metrics_collector_task_with_notifications(
        db.clone(),
        runtime.clone(),
        config.server.external_url.clone(),
    );

    // Start cost calculation background task
    // This computes daily cost snapshots from resource metrics
    spawn_cost_calculator_task(db.clone());

    // Start scheduled jobs scheduler (cron-based commands in containers)
    rivetr::engine::scheduler::spawn_scheduler(db.clone(), runtime.clone());

    // Start scheduled deployment checker (queues deployments whose scheduled_at has passed)
    rivetr::engine::scheduler::spawn_scheduled_deployment_checker(
        db.clone(),
        state.deploy_tx.clone(),
    );

    // Start backup scheduler (runs backup_schedules entries on their cron expressions)
    rivetr::engine::scheduler::spawn_backup_scheduler(db.clone());

    // Start autoscaling checker (evaluates autoscaling rules every 60s)
    rivetr::engine::scheduler::spawn_autoscaling_checker(db.clone());

    // Start advanced monitoring tasks (uptime checker + log cleaner)
    rivetr::monitoring::spawn_uptime_checker_task(db.clone());
    rivetr::monitoring::spawn_log_cleaner_task(db.clone());

    // Create API router
    let api_router = rivetr::api::create_router(state.clone());

    let app = api_router;

    // Start HTTP proxy server and optionally HTTPS with ACME
    let https_port = config.server.proxy_https_port;
    let acme_enabled = config.proxy.acme_enabled
        && config.proxy.acme_email.is_some()
        && config.proxy.instance_domain.is_some();

    if acme_enabled {
        let acme_email = config.proxy.acme_email.clone().unwrap();
        let instance_domain = config.proxy.instance_domain.clone().unwrap();
        let acme_cfg = AcmeConfig {
            email: acme_email,
            cache_dir: config.proxy.acme_cache_dir.clone(),
            staging: config.proxy.acme_staging,
        };

        match AcmeClient::new(acme_cfg).await {
            Ok(acme_client) => {
                let acme_client = std::sync::Arc::new(acme_client);
                let acme_challenges = acme_client.challenges();

                // IMPORTANT: Start HTTP proxy FIRST so ACME HTTP-01 challenges can be served
                // The proxy must be listening on port 80 before Let's Encrypt tries to verify.
                // HTTP→HTTPS redirect starts disabled; enabled only once TLS cert is confirmed.
                let https_redirect_flag =
                    std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                let https_redirect_flag_clone = https_redirect_flag.clone();
                let http_challenges = acme_challenges.clone();
                tokio::spawn(async move {
                    if let Err(e) = proxy_server
                        .run_with_options(
                            Some(http_challenges),
                            Some(https_redirect_flag_clone),
                            https_port,
                        )
                        .await
                    {
                        tracing::error!(error = %e, "HTTP proxy server error");
                    }
                });

                // Give the HTTP proxy a moment to start listening
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;

                // Collect all domains to include in the TLS cert (SAN list)
                // Start with the instance_domain, then add all app domains from DB
                let mut all_cert_domains: Vec<String> = vec![instance_domain.clone()];
                if let Ok(app_domains) = collect_all_app_domains(&db).await {
                    for d in app_domains {
                        if !all_cert_domains.contains(&d) && all_cert_domains.len() < 100 {
                            all_cert_domains.push(d);
                        }
                    }
                }
                tracing::info!(
                    "Requesting TLS cert covering {} domain(s): {:?}",
                    all_cert_domains.len(),
                    all_cert_domains
                );

                // Now request or load the certificate
                let cert_dir = acme_client.cert_dir(&instance_domain);
                // `cert_domains` tracks what's *actually in the cert* (not just the DB list).
                // The renewal manager uses this as its baseline so it can detect new app
                // subdomains that aren't yet covered and reissue immediately.
                let mut cert_domains = all_cert_domains.clone();
                let tls_config_result = if cert_dir.join("fullchain.pem").exists() {
                    tracing::info!(domain = %instance_domain, "Loading cached TLS certificate");
                    // Restore the saved domain list so the renewal manager knows exactly
                    // what SANs the cached cert covers.
                    if let Some(saved) = AcmeClient::load_cert_domains(&cert_dir).await {
                        cert_domains = saved;
                    }
                    AcmeClient::load_certificate(&cert_dir).await.ok()
                } else {
                    tracing::info!(domain = %instance_domain, "Requesting Let's Encrypt certificate");
                    match acme_client.request_certificate(&all_cert_domains).await {
                        Ok(result) => {
                            let _ = acme_client.save_certificate(&result).await;
                            rivetr::proxy::TlsConfig::from_pem(
                                &result.certificate_chain_pem,
                                &result.private_key_pem,
                            )
                            .ok()
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to get Let's Encrypt certificate; running HTTP-only");
                            None
                        }
                    }
                };

                if let Some(tls_config) = tls_config_result {
                    // TLS cert is available — enable HTTP→HTTPS redirect now
                    https_redirect_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                    let https_addr: SocketAddr = format!("{}:{}", config.server.host, https_port)
                        .parse()
                        .expect("Invalid HTTPS proxy address");

                    // Wrap the TLS acceptor in a hot-reload handle so cert renewals take
                    // effect immediately without restarting the HTTPS server.
                    let tls_reload = std::sync::Arc::new(rivetr::proxy::TlsReloadHandle::new(
                        tls_config.acceptor,
                    ));
                    let https_server =
                        HttpsProxyServer::new(https_addr, routes.clone(), tls_reload.clone());
                    tokio::spawn(async move {
                        if let Err(e) = https_server.run().await {
                            tracing::error!(error = %e, "HTTPS proxy server error");
                        }
                    });
                    tracing::info!("HTTPS proxy listening on https://{}", https_addr);

                    // Start certificate renewal manager. `cert_domains` reflects what's
                    // actually in the cert; the DB is queried each cycle for new subdomains.
                    let renewal_mgr = CertificateRenewalManager::new(acme_client, cert_domains)
                        .with_db_and_reload(db.clone(), Some(tls_reload));
                    tokio::spawn(async move { renewal_mgr.run().await });
                } else {
                    tracing::warn!("Running HTTP-only (no TLS certificate available)");
                    // Start renewal manager anyway — it will retry on next cycle.
                    // No tls_reload handle since HTTPS server is not running yet.
                    let renewal_mgr = CertificateRenewalManager::new(acme_client, cert_domains)
                        .with_db_and_reload(db.clone(), None);
                    tokio::spawn(async move { renewal_mgr.run().await });
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "ACME client init failed, starting HTTP-only proxy");
                tokio::spawn(async move {
                    if let Err(e) = proxy_server.run().await {
                        tracing::error!(error = %e, "Proxy server error");
                    }
                });
            }
        }
    } else {
        tokio::spawn(async move {
            if let Err(e) = proxy_server.run().await {
                tracing::error!(error = %e, "Proxy server error");
            }
        });
    }

    // Start health checker for backend health monitoring
    let health_config = HealthCheckerConfig::from_proxy_config(&config.proxy);
    let health_checker = HealthChecker::new(routes.clone(), health_config);
    tokio::spawn(async move {
        health_checker.run().await;
    });

    // Start API server
    let api_addr = format!("{}:{}", config.server.host, config.server.api_port);
    let listener = tokio::net::TcpListener::bind(&api_addr).await?;

    tracing::info!("API server listening on http://{}", api_addr);
    tracing::info!("Proxy server listening on http://{}", proxy_addr);
    tracing::info!("Admin token: {}", config.auth.admin_token);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received");
}

/// Collect all configured domain names across all apps (for TLS SAN list)
async fn collect_all_app_domains(db: &DbPool) -> Result<Vec<String>> {
    let apps: Vec<(Option<String>, Option<String>, Option<String>)> =
        sqlx::query_as("SELECT domain, domains, auto_subdomain FROM apps")
            .fetch_all(db)
            .await?;

    let mut result: Vec<String> = Vec::new();
    for (legacy_domain, domains_json, auto_subdomain) in apps {
        if let Some(d) = legacy_domain {
            if !d.is_empty() && !result.contains(&d) {
                result.push(d);
            }
        }
        if let Some(ref json) = domains_json {
            if let Ok(arr) = serde_json::from_str::<serde_json::Value>(json) {
                if let Some(list) = arr.as_array() {
                    for entry in list {
                        if let Some(d) = entry.get("domain").and_then(|v| v.as_str()) {
                            let d = d.to_string();
                            if !d.is_empty() && !result.contains(&d) {
                                result.push(d);
                            }
                        }
                    }
                }
            }
        }
        if let Some(d) = auto_subdomain {
            if !d.is_empty()
                && !result.contains(&d)
                && !d.ends_with(".traefik.me")
                && !d.ends_with(".sslip.io")
            {
                result.push(d);
            }
        }
    }
    Ok(result)
}

/// Restore proxy routes from running containers on startup
async fn restore_routes(
    db: &DbPool,
    runtime: &Arc<dyn ContainerRuntime>,
    routes: &Arc<ArcSwap<RouteTable>>,
) -> Result<()> {
    // Fetch all apps that have any domain configured (domain, domains JSON, or auto_subdomain),
    // including basic auth fields so they can be re-applied to the restored routes.
    #[allow(clippy::type_complexity)]
    let apps: Vec<(
        String, // id
        String, // name
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        i32,
        Option<String>,
        Option<String>,
        Option<String>, // strip_prefix
    )> = sqlx::query_as(
        "SELECT id, name, domain, domains, healthcheck, auto_subdomain, \
                basic_auth_enabled, basic_auth_username, basic_auth_password_hash, \
                strip_prefix \
         FROM apps \
         WHERE (domain IS NOT NULL AND domain != '') \
            OR (domains IS NOT NULL AND domains != '' AND domains != '[]') \
            OR (auto_subdomain IS NOT NULL AND auto_subdomain != '')",
    )
    .fetch_all(db)
    .await?;

    tracing::info!(
        "Checking {} apps with domains for running containers",
        apps.len()
    );

    // List all running rivetr containers once (avoids repeated calls per app)
    let containers = runtime.list_containers("rivetr-").await?;

    tracing::info!("Found {} running rivetr containers", containers.len());

    for (
        app_id,
        app_name,
        legacy_domain,
        domains_json,
        healthcheck,
        auto_subdomain,
        basic_auth_enabled,
        basic_auth_username,
        basic_auth_password_hash,
        strip_prefix,
    ) in apps
    {
        let container_name = format!("rivetr-{}", app_name);

        // Find the running container for this app
        if let Some(container) = containers.iter().find(|c| c.name == container_name) {
            // Use the port from list_containers if available; otherwise fall back to inspect()
            // so that containers with only internal port mappings are still captured.
            let port = if let Some(p) = container.port {
                Some(p)
            } else {
                tracing::debug!(
                    container = %container_name,
                    "Port not found in list_containers, falling back to inspect()"
                );
                match runtime.inspect(&container.id).await {
                    Ok(info) => info.port,
                    Err(e) => {
                        tracing::warn!(
                            container = %container_name,
                            error = %e,
                            "Failed to inspect container during route restore"
                        );
                        None
                    }
                }
            };

            if let Some(port) = port {
                // Collect all domain names for this app (deduplicated)
                let mut domain_names: Vec<String> = Vec::new();

                // New domains JSON array: [{domain: "...", primary: true, redirect_www: bool}, ...]
                if let Some(ref json) = domains_json {
                    if let Ok(arr) = serde_json::from_str::<serde_json::Value>(json) {
                        if let Some(list) = arr.as_array() {
                            for entry in list {
                                if let Some(d) = entry.get("domain").and_then(|v| v.as_str()) {
                                    if !d.is_empty() && !domain_names.contains(&d.to_string()) {
                                        domain_names.push(d.to_string());
                                    }
                                    // Handle redirect_www variants (www. <-> non-www)
                                    let redirect_www = entry
                                        .get("redirect_www")
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(false);
                                    if redirect_www {
                                        let variant = if d.starts_with("www.") {
                                            d.trim_start_matches("www.").to_string()
                                        } else {
                                            format!("www.{}", d)
                                        };
                                        if !variant.is_empty() && !domain_names.contains(&variant) {
                                            domain_names.push(variant);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Legacy domain field (add if not already captured from domains JSON)
                if let Some(ref d) = legacy_domain {
                    if !d.is_empty() && !domain_names.contains(d) {
                        domain_names.push(d.clone());
                    }
                }

                // Auto-generated subdomain (e.g., app-name.rivetr.example.com)
                if let Some(ref d) = auto_subdomain {
                    if !d.is_empty() && !domain_names.contains(d) {
                        domain_names.push(d.clone());
                    }
                }

                // Load redirect rules for this app
                let redirect_rules_db: Vec<AppRedirectRule> = sqlx::query_as(
                    "SELECT * FROM app_redirect_rules WHERE app_id = ? AND is_enabled = 1 \
                     ORDER BY sort_order ASC, created_at ASC",
                )
                .bind(&app_id)
                .fetch_all(db)
                .await
                .unwrap_or_default();

                let proxy_redirect_rules: Vec<RedirectRule> = redirect_rules_db
                    .into_iter()
                    .map(|r| RedirectRule {
                        source_pattern: r.source_pattern,
                        destination: r.destination,
                        is_permanent: r.is_permanent != 0,
                    })
                    .collect();

                let route_table = routes.load();

                for domain in &domain_names {
                    let mut backend =
                        Backend::new(container.id.clone(), "127.0.0.1".to_string(), port)
                            .with_healthcheck(healthcheck.clone())
                            .with_strip_prefix(strip_prefix.clone());

                    // Restore HTTP Basic Auth configuration if it was enabled
                    if basic_auth_enabled != 0 {
                        if let (Some(ref username), Some(ref password_hash)) =
                            (&basic_auth_username, &basic_auth_password_hash)
                        {
                            backend.set_basic_auth(BasicAuthConfig::new(
                                username.clone(),
                                password_hash.clone(),
                            ));
                        }
                    }

                    // Restore redirect rules
                    if !proxy_redirect_rules.is_empty() {
                        backend.set_redirect_rules(proxy_redirect_rules.clone());
                    }

                    route_table.add_route(domain.clone(), backend);
                    tracing::info!(
                        domain = %domain,
                        port = port,
                        container = %container_name,
                        basic_auth = basic_auth_enabled != 0,
                        redirect_rules = proxy_redirect_rules.len(),
                        "Restored proxy route for app {}",
                        app_name
                    );
                }
            } else {
                tracing::warn!(
                    container = %container_name,
                    "Running container found but could not determine port — route not restored for app {}",
                    app_name
                );
            }
        } else {
            tracing::debug!(
                container = %container_name,
                "No running container found for app {} — skipping route restore",
                app_name
            );
        }
    }

    // Restore service routes for running Docker Compose services
    let services: Vec<Service> = sqlx::query_as(
        "SELECT * FROM services WHERE status = 'running' AND domain IS NOT NULL AND domain != ''",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();

    tracing::info!(
        "Restoring proxy routes for {} running services with domains",
        services.len()
    );

    let route_table = routes.load();
    for service in services {
        if let Some(ref domain) = service.domain {
            let backend = Backend::new(
                format!("rivetr-svc-{}", service.name),
                "127.0.0.1".to_string(),
                service.port as u16,
            );
            route_table.add_route(domain.clone(), backend);
            tracing::info!(
                domain = %domain,
                port = service.port,
                "Restored service proxy route for {}",
                service.name
            );
        }
    }

    Ok(())
}
