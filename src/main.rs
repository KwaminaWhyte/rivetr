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
use rivetr::engine::{
    reconcile_container_status, spawn_cleanup_task as spawn_deployment_cleanup_task,
    spawn_container_monitor_task, spawn_disk_monitor_task, spawn_stats_collector_task,
    spawn_stats_history_task, spawn_stats_retention_task, BuildLimits, DeploymentEngine,
};
use rivetr::proxy::{Backend, HealthChecker, HealthCheckerConfig, ProxyServer, RouteTable};
use rivetr::runtime::{detect_runtime, ContainerRuntime};
use rivetr::startup::run_startup_checks;
use rivetr::AppState;
use rivetr::DbPool;

#[tokio::main]
async fn main() -> Result<()> {
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

    // Reconcile container status on startup
    // This updates database records for containers that stopped while server was down
    reconcile_container_status(&db, &runtime).await;

    // Create app state (now includes routes for rollback functionality)
    let state = Arc::new(
        AppState::new(
            config.clone(),
            db.clone(),
            deploy_tx,
            runtime.clone(),
            routes.clone(),
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

    // Create API router
    let api_router = rivetr::api::create_router(state.clone());

    let app = api_router;

    tokio::spawn(async move {
        if let Err(e) = proxy_server.run().await {
            tracing::error!(error = %e, "Proxy server error");
        }
    });

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

/// Restore proxy routes from running containers on startup
async fn restore_routes(
    db: &DbPool,
    runtime: &Arc<dyn ContainerRuntime>,
    routes: &Arc<ArcSwap<RouteTable>>,
) -> Result<()> {
    // Get all apps with domains from the database
    let apps: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT name, domain, healthcheck FROM apps WHERE domain IS NOT NULL AND domain != ''",
    )
    .fetch_all(db)
    .await?;

    tracing::info!(
        "Checking {} apps with domains for running containers",
        apps.len()
    );

    // List all running rivetr containers
    let containers = runtime.list_containers("rivetr-").await?;

    for (app_name, domain, healthcheck) in apps {
        let container_name = format!("rivetr-{}", app_name);

        // Find the running container for this app
        if let Some(container) = containers.iter().find(|c| c.name == container_name) {
            if let Some(port) = container.port {
                let backend = Backend::new(container.id.clone(), "127.0.0.1".to_string(), port)
                    .with_healthcheck(healthcheck);

                routes.load().add_route(domain.clone(), backend);
                tracing::info!(
                    domain = %domain,
                    port = port,
                    container = %container_name,
                    "Restored proxy route for app {}",
                    app_name
                );
            }
        }
    }

    Ok(())
}
