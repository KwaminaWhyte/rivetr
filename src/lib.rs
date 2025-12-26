pub mod api;
pub mod config;
pub mod db;
pub mod engine;
pub mod proxy;
pub mod runtime;
pub mod ui;
pub mod utils;

pub use db::DbPool;

use config::Config;
use tokio::sync::mpsc;

use crate::db::App;

pub struct AppState {
    pub config: Config,
    pub db: DbPool,
    pub deploy_tx: mpsc::Sender<(String, App)>,
}

impl AppState {
    pub fn new(config: Config, db: DbPool, deploy_tx: mpsc::Sender<(String, App)>) -> Self {
        Self {
            config,
            db,
            deploy_tx,
        }
    }
}
