pub mod api;
pub mod config;
pub mod db;
pub mod engine;
pub mod proxy;
pub mod runtime;
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
}

impl AppState {
    pub fn new(
        config: Config,
        db: DbPool,
        deploy_tx: mpsc::Sender<(String, App)>,
        runtime: Arc<dyn ContainerRuntime>,
        routes: Arc<ArcSwap<RouteTable>>,
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
        }
    }

    /// Set the Prometheus metrics handle
    pub fn with_metrics(mut self, handle: PrometheusHandle) -> Self {
        self.metrics_handle = Some(handle);
        self
    }
}
