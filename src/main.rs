use anyhow::Result;
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use rivetr::config::Config;
use rivetr::engine::DeploymentEngine;
use rivetr::proxy::ProxyServer;
use rivetr::runtime::detect_runtime;
use rivetr::AppState;

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

    // Ensure data directory exists
    rivetr::utils::ensure_dir(&config.server.data_dir)?;

    // Initialize database
    let db = rivetr::db::init(&config.server.data_dir).await?;

    // Ensure default admin user exists
    rivetr::api::auth::ensure_admin_user(
        &db,
        &config.auth.admin_email,
        &config.auth.admin_password,
    )
    .await?;

    // Detect container runtime
    let runtime = detect_runtime(&config.runtime).await?;

    // Create deployment channel
    let (deploy_tx, deploy_rx) = mpsc::channel(100);

    // Create app state
    let state = Arc::new(AppState::new(config.clone(), db.clone(), deploy_tx));

    // Start proxy server
    let proxy_addr: SocketAddr = format!("{}:{}", config.server.host, config.server.proxy_port)
        .parse()
        .expect("Invalid proxy address");
    let proxy_server = ProxyServer::new(proxy_addr);
    let routes = proxy_server.routes();

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
