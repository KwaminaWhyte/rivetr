pub mod ai;
pub mod api;
pub mod backup;
pub mod cli;
pub mod config;
pub mod crypto;
pub mod db;
pub mod engine;
pub mod github;
pub mod logging;
pub mod mcp;
pub mod monitoring;
pub mod notifications;
pub mod proxy;
pub mod runtime;
pub mod startup;
#[cfg(feature = "tui")]
pub mod tui;
pub mod ui;
pub mod utils;

pub use db::DbPool;

use arc_swap::ArcSwap;
use config::Config;
use metrics_exporter_prometheus::PrometheusHandle;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::api::rate_limit::RateLimiter;
use crate::db::App;
use crate::engine::updater::UpdateChecker;
use crate::proxy::RouteTable;
use crate::runtime::ContainerRuntime;

pub struct AppState {
    pub config: Config,
    pub db: DbPool,
    pub deploy_tx: mpsc::Sender<(String, App)>,
    pub runtime: Arc<dyn ContainerRuntime>,
    pub routes: Arc<ArcSwap<RouteTable>>,
    pub rate_limiter: Arc<RateLimiter>,
    pub metrics_handle: Option<PrometheusHandle>,
    pub update_checker: Arc<UpdateChecker>,
    /// Cancellation tokens for in-progress deployments. Keyed by deployment ID.
    pub deployment_cancel_tokens: dashmap::DashMap<String, tokio_util::sync::CancellationToken>,
    /// Optional AI client — only present when an API key is configured in [ai] section.
    pub ai_client: Option<Arc<crate::ai::AiClient>>,
}

impl AppState {
    pub fn new(
        config: Config,
        db: DbPool,
        deploy_tx: mpsc::Sender<(String, App)>,
        runtime: Arc<dyn ContainerRuntime>,
        routes: Arc<ArcSwap<RouteTable>>,
        update_checker: Arc<UpdateChecker>,
    ) -> Self {
        let rate_limiter = Arc::new(RateLimiter::new(config.rate_limit.clone()));
        Self {
            config,
            db,
            deploy_tx,
            runtime,
            routes,
            rate_limiter,
            metrics_handle: None,
            update_checker,
            deployment_cancel_tokens: dashmap::DashMap::new(),
            ai_client: None,
        }
    }

    /// Attach an AI client to the application state.
    pub fn with_ai_client(mut self, client: Option<Arc<crate::ai::AiClient>>) -> Self {
        self.ai_client = client;
        self
    }

    /// Set the Prometheus metrics handle
    pub fn with_metrics(mut self, handle: PrometheusHandle) -> Self {
        self.metrics_handle = Some(handle);
        self
    }
}
