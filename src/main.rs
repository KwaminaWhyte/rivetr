use anyhow::Result;
use arc_swap::ArcSwap;
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use rivetr::api::rate_limit::spawn_cleanup_task;
use rivetr::config::Config;
use rivetr::engine::DeploymentEngine;
use rivetr::proxy::{Backend, HealthChecker, HealthCheckerConfig, ProxyServer, RouteTable};
use rivetr::runtime::{detect_runtime, ContainerRuntime};
use rivetr::AppState;
use rivetr::DbPool;

#[derive(Parser, Debug)]
#[command(name = "rivetr")]
#[command(author, version, about = "A fast, lightweight deployment engine", long_about = None)]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "rivetr.toml")]
    config: PathBuf,

    /// Override log level
    #[arg(short, long)]
    log_level: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

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

    // Create app state (now includes routes for rollback functionality)
    let state = Arc::new(
        AppState::new(config.clone(), db.clone(), deploy_tx, runtime.clone(), routes.clone())
            .with_metrics(metrics_handle)
    );

    // Start rate limiter cleanup task
    spawn_cleanup_task(
        state.rate_limiter.clone(),
        config.rate_limit.cleanup_interval,
    );
    tracing::info!(
        "Rate limiting enabled: {} req/min (API), {} req/min (webhooks), {} req/min (auth)",
        config.rate_limit.api_requests_per_window,
        config.rate_limit.webhook_requests_per_window,
        config.rate_limit.auth_requests_per_window
    );

    // Start deployment engine with route table
    let engine = DeploymentEngine::new(db.clone(), runtime.clone(), routes.clone(), deploy_rx);
    tokio::spawn(async move {
        engine.run().await;
    });

    // Create API router
    let api_router = rivetr::api::create_router(state.clone());

    // Serve React static files with SPA fallback
    let static_dir = PathBuf::from("static/dist");
    let index_file = static_dir.join("index.html");
    let serve_static = ServeDir::new(&static_dir)
        .not_found_service(ServeFile::new(&index_file));

    // Combine routers - API first, then static files as fallback
    let app = axum::Router::new()
        .merge(api_router)
        .fallback_service(serve_static);

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
        "SELECT name, domain, healthcheck FROM apps WHERE domain IS NOT NULL AND domain != ''"
    )
    .fetch_all(db)
    .await?;

    tracing::info!("Checking {} apps with domains for running containers", apps.len());

    // List all running rivetr containers
    let containers = runtime.list_containers("rivetr-").await?;

    for (app_name, domain, healthcheck) in apps {
        let container_name = format!("rivetr-{}", app_name);

        // Find the running container for this app
        if let Some(container) = containers.iter().find(|c| c.name == container_name) {
            if let Some(port) = container.port {
                let backend = Backend::new(
                    container.id.clone(),
                    "127.0.0.1".to_string(),
                    port,
                )
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
